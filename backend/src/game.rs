use std::sync::atomic::{AtomicI32, Ordering};
use std::collections::HashMap;
use std::net::SocketAddr;

use rand::Rng;
use nalgebra::Vector3;

use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
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
static ACTOR_COUNTER: AtomicI32 = AtomicI32::new(1);

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

    // Client Messages
    Hello(Client, UnboundedSender<GameResponse>, String),
    Goodbye(Client),
    Ping(Client, f64),
    Move(Client, f32, f32, f32),

    // Actor Messages
    Respawn(u32),
    Scan(u32, oneshot::Sender<(Actor, GameObject, Vec<GameObject>)>),
    ActorMove(u32, f32, f32, f32),
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
    pub alive: bool,
    pub object_id: u32,
    pub object_type: ObjectType,
    pub position: Vector3<f64>,
    pub velocity: Vector3<f64>,
}

impl GameObject {
    pub fn new(object_type: ObjectType) -> GameObject {
        GameObject {
            alive: true,
            object_type,
            object_id: GAME_OBJECT_COUNTER.fetch_add(1, Ordering::Relaxed) as u32,
            position: Vector3::new(0.0, 0.0, 0.0),
            velocity: Vector3::new(0.0, 0.0, 0.0),
        }
    }
}

#[derive(Clone, Debug)]
pub enum ActorType {
    Walker,
}


#[derive(Clone, Debug)]
pub struct Actor {
    pub actor_id: u32,
    pub actor_type: ActorType,
    pub object_id: u32,
}

impl Actor {
    pub fn new(actor_type: ActorType, object_id: u32) -> Actor {
        Actor {
            actor_id: ACTOR_COUNTER.fetch_add(1, Ordering::Relaxed) as u32,
            actor_type,
            object_id,
        }
    }
}

async fn actor_main(actor: Actor, tx: UnboundedSender<GameMessage>) {
    let mut interval = time::interval(Duration::from_millis(100));
    let mut count = (17 * actor.actor_id * actor.object_id) % 1000;

    loop {
        count += 1;
        interval.tick().await;

        if count % 1000 == 0 {
            if let Err(e) = tx.send(GameMessage::Respawn(actor.actor_id)) {
                log::error!("error sending respawn: {}", e);
            }
            break
        }

        let (sender, receiver) = oneshot::channel::<(Actor, GameObject, Vec<GameObject>)>();
        let result = tx.send(GameMessage::Scan(actor.actor_id, sender));
        if let Err(e) = result {
            log::error!("actor error sending to game server {:?}: {}", actor, e);
        }

        let response = receiver.await;
        match response {
            Ok((_, actor_obj, players)) => {
                if let Some(player) = players.iter().nth(0) {
                    let dir = (player.position - actor_obj.position).normalize();
                    if let Err(e) = tx.send(GameMessage::ActorMove(actor.actor_id, dir.x as f32, dir.y as f32, dir.z as f32)) {
                        log::error!("error sending move: {:?}", e);
                    }
                }
            },
            Err(e) => {
                log::error!("error getting response to scan: {}", e);
            },
        }
    }
}

pub struct GameArea {
    pub area_size: u32,
    pub objects: HashMap<u32, GameObject>,
    pub actors: HashMap<u32, Actor>,
    pub actor_handles: HashMap<u32, JoinHandle<()>>,
    pub players: HashMap<u32, Player>,
    pub game_tx: UnboundedSender<GameMessage>,
}

impl GameArea {
    pub fn new(area_size: u32, game_tx: UnboundedSender<GameMessage>) -> GameArea {
        GameArea {
            area_size,
            objects: HashMap::new(),
            actors: HashMap::new(),
            players: HashMap::new(),
            actor_handles: HashMap::new(),
            game_tx,
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

    pub fn spawn_actor(&mut self, actor_type: ActorType) {
        let tx = self.game_tx.clone();
        let mut rng = rand::thread_rng();

        let size = self.area_size as f64;
        let item = self.add_object(ObjectType::Actor);
        item.position.x = -size + (2.0 * rng.gen::<f64>() * size);
        item.position.z = -size + (2.0 * rng.gen::<f64>() * size);

        let actor = Actor::new(actor_type, item.object_id);
        let actor_id = actor.actor_id;
        let handle_actor = actor.clone();
        self.actors.insert(actor_id, actor);

        let handle = tokio::spawn(async {
            actor_main(handle_actor, tx).await;
        });
        self.actor_handles.insert(actor_id, handle);
    }

    pub fn populate(&mut self, num_items: u32, num_actors: u32) {
        let mut rng = rand::thread_rng();

        let size = self.area_size as f64;
        for _n in 0..num_items {
            let item = self.add_object(ObjectType::Item);
            item.position.x = -size + (2.0 * rng.gen::<f64>() * size);
            item.position.z = -size + (2.0 * rng.gen::<f64>() * size);
        }

        for _n in 0..num_actors {
            self.spawn_actor(ActorType::Walker);
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
                incremental: false,
            }));
        }
    }

    async fn handle_goodbye(&mut self, client: Client) {
        if let Some(player) = self.players.remove(&client.client_id) {
            if let Some(mut player_obj) = self.objects.remove(&player.object_id) {
                player_obj.alive = false;
                for other in self.players.values() {
                    other.send(GameResponse::StateUpdate(StateUpdate {
                        object_id: other.object_id,
                        area_size: self.area_size,
                        objects: vec![player_obj.clone()],
                        incremental: true
                    }));
                }
            }
            player.send(GameResponse::Goodbye());
        }
    }

    async fn handle_ping(&mut self, client: Client, timestamp: f64) {
        if let Some(player) = self.players.get(&client.client_id) {
            player.send(GameResponse::Pong(timestamp));
        }
    }

    async fn handle_move(&mut self, client: Client, x: f32, y: f32, z: f32) {
        let size = self.area_size as f64;

        if let Some(player) = self.players.get(&client.client_id) {
            if let Some(player_obj) = self.objects.get_mut(&player.object_id) {
                player_obj.position.x += x as f64;
                player_obj.position.y += y as f64;
                player_obj.position.z += z as f64;

                // FIXME add a helper to broadcast an update
                for other in self.players.values() {
                    other.send(GameResponse::StateUpdate(StateUpdate {
                        object_id: other.object_id,
                        area_size: size as u32,
                        incremental: true,
                        objects: vec![player_obj.clone()],
                    }));
                }
            }
        }
    }

    async fn handle_scan(&mut self, actor_id: u32, response_conn: oneshot::Sender<(Actor, GameObject, Vec<GameObject>)>) {
        let actor = self.actors.get(&actor_id).unwrap();
        let actor_obj = self.objects.get(&actor.object_id).unwrap();

        let mut player_objs: Vec<&GameObject> = Vec::new();
        for player in self.players.values() {
            if let Some(player_obj) = self.objects.get(&player.object_id) {
                player_objs.push(player_obj)
            }
        }

        player_objs.sort_by(|a, b| {
            let a_dist = a.position.metric_distance(&actor_obj.position);
            let b_dist = b.position.metric_distance(&actor_obj.position);
            a_dist.partial_cmp(&b_dist).unwrap()
        });

        let result = player_objs.iter().take(10).cloned().cloned().collect();
        if let Err(e) = response_conn.send((actor.clone(), actor_obj.clone(), result)) {
            log::error!("error sending response: {:?}", e);
        }
    }

    async fn handle_actor_move(&mut self, actor_id: u32, x: f32, y: f32, z: f32) {
        let size = self.area_size as f64;

        if let Some(actor) = self.actors.get(&actor_id) {
            if let Some(actor_obj) = self.objects.get_mut(&actor.object_id) {
                actor_obj.position.x += x as f64;
                actor_obj.position.y += y as f64;
                actor_obj.position.z += z as f64;

                // FIXME add a helper to broadcast an update
                for other in self.players.values() {
                    other.send(GameResponse::StateUpdate(StateUpdate {
                        object_id: other.object_id,
                        area_size: size as u32,
                        incremental: true,
                        objects: vec![actor_obj.clone()],
                    }));
                }
            }
        }
    }

    async fn handle_respawn(&mut self, actor_id: u32) {
        let size = self.area_size as f64;

        if let Some(handle) = self.actor_handles.remove(&actor_id) {
            if let Err(e) = handle.await {
                log::error!("error waiting for actor to reap: {}", e);
            }
        }

        if let Some(actor) = self.actors.remove(&actor_id) {
            if let Some(mut actor_obj) = self.objects.remove(&actor.object_id) {
                actor_obj.alive = false;
                // FIXME add a helper to broadcast an update
                for other in self.players.values() {
                    other.send(GameResponse::StateUpdate(StateUpdate {
                        object_id: other.object_id,
                        area_size: size as u32,
                        incremental: true,
                        objects: vec![actor_obj.clone()],
                    }));
                }
            }
        }

        self.spawn_actor(ActorType::Walker);
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
                GameMessage::Scan(actor_id, response_conn) => {
                    self.handle_scan(actor_id, response_conn).await;
                },
                GameMessage::ActorMove(actor_id, x, y, z) => {
                    self.handle_actor_move(actor_id, x, y, z).await;
                },
                GameMessage::Respawn(actor_id) => {
                    self.handle_respawn(actor_id).await;
                },
            }
        }
    }
}
