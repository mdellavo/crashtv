// #![deny(warnings)]

extern crate pretty_env_logger;
#[macro_use]
extern crate rmp_serde as rmps;
extern crate log;

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::net::SocketAddr;

use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

use futures_util::{StreamExt, SinkExt};

use warp::ws::{WebSocket, Message};
use warp::Filter;

use serde::Serialize;
use rmps::Serializer;

mod game;
mod net;
mod actor;

use game::{Client, GameArea, GameMessage, GameResponse};

const AREA_SIZE: u32 = 2500;

#[derive(Debug, Deserialize)]
pub enum ClientMessage {
    Hello(String),
    Ping(u64),
    Goodbye(),
    Move(f32, f32, f32),
}

async fn user_connected(client: Client, websocket: WebSocket, game_conn: UnboundedSender<GameMessage>) {

    let (mut websocket_tx, mut websocket_rx) = websocket.split();
    let (client_tx, mut client_rx) = unbounded_channel::<GameResponse>();

    // one thread reading from websocket_rx, decoding messages, and pumping to game_conn
    // one thread reading from client_rx and, encoding, and pumping to websocket_tx
    // messages over game conn send client_tx for responses
    tokio::spawn(async move {
        while let Some(msg) = client_rx.recv().await {
            let mut buf = Vec::new();
            let mut serializer = Serializer::new(&mut buf);
            let result = msg.serialize(&mut serializer);
            if let Err(e) = result {
                log::error!("websocket serialize error {:?}: {}", client, e);
                break;
            }

            let result = websocket_tx.send(Message::binary(buf)).await;
            if let Err(e) = result {
                log::error!("websocket write error {:?}: {}", client, e);
                break;
            }
        }
    });

    while let Some(result) = websocket_rx.next().await {
        let encoded_msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                log::error!("websocket read error {:?}: {}", client, e);
                break;
            }
        };

        let msg_bytes = encoded_msg.as_bytes();

        if msg_bytes.len() == 0 {
            break;
        }

        let msg : ClientMessage = match rmp_serde::from_slice(encoded_msg.as_bytes()) {
            Ok(msg) => msg,
            Err(e) => {
                log::error!("websocket deserialize error {:?}: {}", client, e);
                break;
            }
        };

        let game_msg = match msg {
            ClientMessage::Hello(username) => {
                GameMessage::Hello(client, client_tx.clone(), username)
            },
            ClientMessage::Ping(timestamp) => {
                GameMessage::Ping(client, timestamp)
            },
            ClientMessage::Goodbye() => {
                GameMessage::Goodbye(client)
            },
            ClientMessage::Move(x, y, z) => {
                GameMessage::Move(client, x, y, z)
            },
        };

        let result = game_conn.send(game_msg);
        if let Err(e) = result {
            log::error!("error sending hello {:?}: {}", client, e);
        }
    }

    log::info!("client shutdown");
    let result = game_conn.send(GameMessage::Goodbye(client));
    if let Err(e) = result {
        log::error!("error sending hello {:?}: {}", client, e);
    }
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let (game_tx, game_rx) = unbounded_channel::<GameMessage>();

    let tx = game_tx.clone();
    tokio::spawn(async move {
        let mut area = GameArea::new(AREA_SIZE, tx.clone());
        area.populate(AREA_SIZE, 500);
        log::info!("game server running");
        area.process(game_rx).await
    });

    let next_client_id = Arc::new(AtomicU32::new(1));

    let route = warp::path("ws")
        .and(warp::any().map(move || next_client_id.clone().fetch_add(1, Ordering::Relaxed)))
        .and(warp::addr::remote())
        .and(warp::ws())
        .and(warp::any().map(move || game_tx.clone()))
        .map(move |client_id: u32, addr: Option<SocketAddr>, ws: warp::ws::Ws, game_conn: UnboundedSender<GameMessage>| {
            let client = Client {
                client_id,
                addr: addr.unwrap(),
            };

            log::info!("{:?} connected", client);

            ws.on_upgrade(move |websocket| {
                async move {
                    user_connected(client, websocket, game_conn).await;
                }
            })
        });

    warp::serve(route).run(([127, 0, 0, 1], 3030)).await;
}
