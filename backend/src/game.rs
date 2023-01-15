use std::sync::atomic::{AtomicI32, Ordering};
use std::collections::HashMap;
use std::net::SocketAddr;

use rand::Rng;
use nalgebra::Vector3;

use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver};
use tokio::time::{self, Duration};

use serde::Serialize;

use crate::net::StateUpdate;

#[derive(Clone, Debug, Serialize)]
pub enum ObjectType {
    Actor,
    Item,
    Player,
}

static GAME_OBJECT_COUNTER: AtomicI32 = AtomicI32::new(1);

#[derive(Debug, Copy, Clone)]
pub struct Client {
    pub client_id: u32,
    pub addr: SocketAddr,
}

#[derive(Debug)]
pub struct Player {
    pub client: Client,
    pub conn: UnboundedSender<GameResponse>,
    pub username: String,
    pub object_id: u32,
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

impl Player {
    pub fn send(&self, response: GameResponse) {
        let result = self.conn.send(response);
        if let Err(e) = result {
            log::error!("game response write error {:?}: {}", self.client, e);
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct GameObject {
    pub object_id: u32,
    pub object_type: ObjectType,
    pub position: Vector3<f64>,
    pub velocity: Vector3<f64>,
}

impl GameObject {
    pub fn new(object_type: ObjectType) -> GameObject {
        GameObject {
            object_type,
            object_id: GAME_OBJECT_COUNTER.fetch_add(1, Ordering::Relaxed) as u32,
            position: Vector3::new(0.0, 0.0, 0.0),
            velocity: Vector3::new(0.0, 0.0, 0.0),
        }
    }
}

pub struct Actor {
    pub actor_id: u32,
    pub object_id: u32,
}

impl Actor {

}

pub struct GameArea {
    pub area_size: u32,
    pub objects: HashMap<u32, GameObject>,
    pub actors: HashMap<u32, Actor>,
    pub players: HashMap<u32, Player>,
}

impl GameArea {
    pub fn new(area_size: u32) -> GameArea {
        GameArea {
            area_size,
            objects: HashMap::new(),
            actors: HashMap::new(),
            players: HashMap::new(),
        }
    }

    pub fn has_username(&self, username: &String) -> bool {
        let usernames: Vec<String> = self.players.values().map(|x| x.username.clone()).collect();
        usernames.iter().any(|x| x.eq(username))
    }

    pub fn add_object(&mut self, object_type: ObjectType) -> &mut GameObject {
        let obj = GameObject::new(object_type);
        let key = obj.object_id;
        self.objects.insert(obj.object_id, obj);
        return self.objects.get_mut(&key).unwrap();
    }

    pub fn populate(&mut self, num_items: u32) {
        let mut rng = rand::thread_rng();

        let size = self.area_size as f64;
        for _n in 0..num_items {
            let item = self.add_object(ObjectType::Item);
            item.position.x = -size + (2.0 * rng.gen::<f64>() * size);
            item.position.z = -size + (2.0 * rng.gen::<f64>() * size);
        }
    }

    async fn handle_hello(&mut self, client: Client, client_conn: UnboundedSender<GameResponse>, username: String) {
        if self.players.contains_key(&client.client_id) {
            let result = client_conn.send(GameResponse::Error(1, "Incorrect hello".to_string()));
            if let Err(e) = result {
                log::error!("game response write error {:?}: {}", client, e);
            }
            return;
        }

        if self.has_username(&username) {
            let result = client_conn.send(GameResponse::Error(1, "Username already taken".to_string()));
            if let Err(e) = result {
                log::error!("game response write error {:?}: {}", client, e);
            }
            return;
        }

        let player_obj = self.add_object(ObjectType::Player);
        let player = Player {
            client,
            conn: client_conn,
            username,
            object_id: player_obj.object_id,
        };

        let notice = format!("Hello {}", player.username);
        player.send(GameResponse::Notice(notice));

        self.players.insert(client.client_id, player);

        for other in self.players.values() {
            other.send(GameResponse::StateUpdate(StateUpdate {
                object_id: other.object_id,
                area_size: self.area_size,
                objects: self.objects.values().cloned().collect(),
            }));
        }
    }

    async fn handle_goodbye(&mut self, client: Client) {
        if let Some(player) = self.players.get_mut(&client.client_id) {
            player.send(GameResponse::Goodbye());
            self.objects.remove(&player.object_id);
        }
        self.players.remove(&client.client_id);
    }

    async fn handle_ping(&mut self, client: Client, timestamp: f64) {
        if let Some(player) = self.players.get(&client.client_id) {
            player.send(GameResponse::Pong(timestamp));
        }
    }

    async fn handle_move(&mut self, client: Client, x: f32, y: f32, z: f32) {
        let size = self.area_size;

        if let Some(player) = self.players.get(&client.client_id) {

            if let Some(player_obj) = self.objects.get_mut(&player.object_id) {
                player_obj.position.x += x as f64;
                player_obj.position.y += y as f64;
                player_obj.position.z += z as f64;
            }

            // FIXME make incremental
            // FIXME add a helper to broadcast an update
            for other in self.players.values() {
                other.send(GameResponse::StateUpdate(StateUpdate {
                    object_id: player.object_id,
                    area_size: size,
                    objects: self.objects.values().cloned().collect(),
                }));
            }
        }
    }


    pub async fn process(&mut self, mut game_rx: UnboundedReceiver<GameMessage>) {
        while let Some(msg) = game_rx.recv().await {
            match msg {
                GameMessage::Hello(client, client_conn, username) => {
                    self.handle_hello(client, client_conn, username).await;
                },
                GameMessage::Goodbye(client) => {
                    self.handle_goodbye(client).await;
                },
                GameMessage::Ping(client, timestamp) => {
                    self.handle_ping(client, timestamp).await;
                },
                GameMessage::Move(client, x, y, z) => {
                    self.handle_move(client, x, y, z).await;
                },
            }
        }
    }
}
