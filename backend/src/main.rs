// #![deny(warnings)]

extern crate pretty_env_logger;

extern crate serde;

#[macro_use]
extern crate rmp_serde as rmps;
#[macro_use] extern crate log;

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::net::SocketAddr;

use tokio::sync::mpsc::{unbounded_channel, UnboundedSender, UnboundedReceiver};

use futures_util::{StreamExt, SinkExt};

use warp::ws::{WebSocket, Message};
use warp::Filter;

use serde::{Deserialize, Serialize};
use rmps::{Deserializer, Serializer};

mod game;
mod net;
use game::GameArea;
use net::StateUpdate;

const AREA_SIZE: u32 = 1000;

#[derive(Debug, Copy, Clone)]
pub struct Client {
    pub client_id: u32,
    pub addr: SocketAddr,
}

#[derive(Debug, Serialize)]
pub enum GameResponse {
    HelloResponse(StateUpdate)
}

#[derive(Debug)]
pub enum GameMessage {
    Hello(Client, UnboundedSender<GameResponse>),
    Goodbye(Client, UnboundedSender<GameResponse>),
    Ping(Client, UnboundedSender<GameResponse>),
}

async fn user_connected(client: Client, websocket: WebSocket, game_conn: UnboundedSender<GameMessage>) {

    let (mut websocket_tx, mut websocket_rx) = websocket.split();
    let (client_tx, mut client_rx) = unbounded_channel::<GameResponse>();

    // one thread reading from websocket_rx, decoding messages, and pumping to game_conn
    // one thread reading from client_rx and, encoding, and pumping to websocket_tx
    // messages over game conn send client_tx for responses

    let websocket_writer = tokio::spawn(async move {
        while let Some(msg) = client_rx.recv().await {
            log::info!("socket writer got {:?}", msg);

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

    let result = game_conn.send(GameMessage::Hello(client, client_tx));
    if let Err(e) = result {
        log::error!("error sending hello {:?}: {}", client, e);
    }

    while let Some(result) = websocket_rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                log::error!("websocket read error {:?}: {}", client, e);
                break;
            }
        };

        log::info!("socket reader got {:?}", msg);
    }

    log::info!("client shutdown");
}

async fn game_main(area: GameArea, mut game_rx: UnboundedReceiver<GameMessage>) {
    log::info!("game server running");

    while let Some(msg) = game_rx.recv().await {
        log::info!("game server got {:?}", msg);

        match msg {
            GameMessage::Hello(client, client_conn) => {
                let response = GameResponse::HelloResponse(StateUpdate {
                    area_size: area.area_size,
                    objects: area.objects.clone()
                });
                let result = client_conn.send(response);
                if let Err(e) = result {
                    log::error!("game response write error {:?}: {}", client, e);
                    break;
                }
            },
            GameMessage::Goodbye(client, client_conn) => {

            },
            GameMessage::Ping(client, client_conn) => {

            },
        }
    }
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let mut area = GameArea::new(AREA_SIZE);
    area.populate(500);

    let (game_tx, mut game_rx) = unbounded_channel::<GameMessage>();

    let game_server = tokio::spawn(game_main(area, game_rx));

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

            ws.on_upgrade(move |mut websocket| {
                async move {
                    user_connected(client, websocket, game_conn).await;
                }
            })
        });

    warp::serve(route).run(([127, 0, 0, 1], 3030)).await;
}
