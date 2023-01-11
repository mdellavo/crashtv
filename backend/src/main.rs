// #![deny(warnings)]

extern crate pretty_env_logger;
#[macro_use]
extern crate rmp_serde as rmps;
extern crate log;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::net::SocketAddr;

use tokio::sync::mpsc::{unbounded_channel, UnboundedSender, UnboundedReceiver};

use futures_util::{StreamExt, SinkExt};

use warp::ws::{WebSocket, Message};
use warp::Filter;

use serde::Serialize;
use rmps::Serializer;

mod game;
mod net;
use game::GameArea;
use net::StateUpdate;

use crate::game::ObjectType;

const AREA_SIZE: u32 = 1000;

#[derive(Debug, Copy, Clone)]
pub struct Client {
    pub client_id: u32,
    pub addr: SocketAddr,
}

#[derive(Debug, Deserialize)]
pub enum ClientMessage {
    Hello(String),
    Ping(f64),
    Goodbye(),
    Move(f32, f32, f32),
}

#[derive(Debug)]
pub enum GameMessage {
    Hello(Client, UnboundedSender<GameResponse>, String),
    Goodbye(Client),
    Ping(Client, f64),
    Move(Client, f32, f32, f32),
}

#[derive(Debug, Serialize)]
pub enum GameResponse {
    Error(u32, String),
    StateUpdate(StateUpdate),
    Pong(f64),
    Goodbye(),
    Notice(String),
}

#[derive(Debug)]
pub struct Player {
    client: Client,
    conn: UnboundedSender<GameResponse>,
    username: String,
    object_id: u32,
}

impl Player {
    pub fn send(&self, response: GameResponse) {
        let result = self.conn.send(response);
        if let Err(e) = result {
            log::error!("game response write error {:?}: {}", self.client, e);
        }
    }
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

async fn game_main(mut area: GameArea, mut game_rx: UnboundedReceiver<GameMessage>) {
    log::info!("game server running");

    let mut players = HashMap::<u32, Player>::new();

    while let Some(msg) = game_rx.recv().await {
        match msg {
            GameMessage::Hello(client, client_conn, username) => {

                if players.contains_key(&client.client_id) {
                    let result = client_conn.send(GameResponse::Error(1, "Incorrect hello".to_string()));
                    if let Err(e) = result {
                        log::error!("game response write error {:?}: {}", client, e);
                    }
                    return;
                }

                let usernames : Vec<String> = players.values().map(|x| x.username.clone()).collect();
                if usernames.iter().any(|x| x.eq(&username)) {
                    let result = client_conn.send(GameResponse::Error(1, "Username already taken".to_string()));
                    if let Err(e) = result {
                        log::error!("game response write error {:?}: {}", client, e);
                    }
                    return;
                }

                let center = area.area_size as f64 / 2.0;
                let player_obj = area.add_object(ObjectType::Actor);
                player_obj.position.x = center;
                player_obj.position.z = center;

                let player = Player {
                    client,
                    conn: client_conn,
                    username,
                    object_id: player_obj.object_id,
                };

                let notice = format!("Hello {}", player.username);
                player.send(GameResponse::Notice(notice));

                players.insert(client.client_id, player);

                for other in players.values() {
                    other.send(GameResponse::StateUpdate(StateUpdate {
                        area_size: area.area_size,
                        objects: area.objects.values().cloned().collect(),
                    }));
                }

            },
            GameMessage::Goodbye(client) => {
                let player = &players[&client.client_id];
                player.send(GameResponse::Goodbye());
                area.remove_object(player.object_id);
                players.remove(&client.client_id);
            },
            GameMessage::Ping(client, timestamp) => {
                let player = &players[&client.client_id];
                player.send(GameResponse::Pong(timestamp));
            },
            GameMessage::Move(client, x, y, z) => {
                let player = &players[&client.client_id];
                let player_obj = area.get_object(player.object_id).unwrap();
                player_obj.position.x += x as f64;
                player_obj.position.y += y as f64;
                player_obj.position.z += z as f64;

                for other in players.values() {
                    other.send(GameResponse::StateUpdate(StateUpdate {
                        area_size: area.area_size,
                        objects: area.objects.values().cloned().collect(),
                    }));
                }
            },
        }
    }
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let mut area = GameArea::new(AREA_SIZE);
    area.populate(500);

    let (game_tx, game_rx) = unbounded_channel::<GameMessage>();

    tokio::spawn(game_main(area, game_rx));

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
