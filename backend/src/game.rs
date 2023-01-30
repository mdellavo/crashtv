use std::sync::atomic::{AtomicI32, Ordering};
use std::collections::HashMap;
use std::net::SocketAddr;

use rand::Rng;
use nalgebra::Vector3;

use kdtree::KdTree;
use kdtree::distance::squared_euclidean;


use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::{Duration, Instant};
use tokio::time::timeout;

use serde::Serialize;

use crate::net::StateUpdate;
use crate::actor::{Actor, ActorType, actor_main};

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

    // Client Messages
    Hello(Client, UnboundedSender<GameResponse>, String),
    Goodbye(Client),
    Ping(Client, u64),
    Move(Client, f32, f32, f32),

    // Actor Messages
    Die(u32),
    Respawn(u32),
    Scan(u32, oneshot::Sender<(Actor, GameObject, Vec<GameObject>, Vec<GameObject>)>),
    ActorMove(u32, f32, f32, f32),
}

#[derive(Debug, Serialize)]
pub enum GameResponse {
    Error(u32, String),
    StateUpdate(StateUpdate),
    Pong(u64),
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
    pub age: u32,
    pub object_id: u32,
    pub object_type: ObjectType,
    pub position: Vector3<f32>,
    pub velocity: Vector3<f32>,
    pub health: u8,
}

impl GameObject {
    pub fn new(object_type: ObjectType) -> GameObject {
        GameObject {
            alive: true,
            age: 0,
            object_type,
            object_id: GAME_OBJECT_COUNTER.fetch_add(1, Ordering::Relaxed) as u32,
            position: Vector3::new(0.0, 0.0, 0.0),
            velocity: Vector3::new(0.0, 0.0, 0.0),
            health: 100,
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
    pub player_kdtree: KdTree<f32, u32, [f32; 3]>,
    pub actor_kdtree: KdTree<f32, u32, [f32; 3]>,
    pub item_kdtree: KdTree<f32, u32, [f32; 3]>,
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
            player_kdtree: KdTree::new(3),
            actor_kdtree: KdTree::new(3),
            item_kdtree: KdTree::new(3),
        }
    }

    pub fn has_username(&self, username: &String) -> bool {
        let usernames: Vec<String> = self.players.values().map(|x| x.username.clone()).collect();
        usernames.iter().any(|x| x.eq(username))
    }

    pub fn add_object(&mut self, object_type: ObjectType, x: f32, y: f32, z: f32) -> &mut GameObject {
        let mut obj = GameObject::new(object_type.clone());

        obj.position.x = x;
        obj.position.y = y;
        obj.position.z = z;

        let kdtree = match object_type {
            ObjectType::Player => &mut self.player_kdtree,
            ObjectType::Actor => &mut self.actor_kdtree,
            ObjectType::Item => &mut self.item_kdtree,
        };

        let p = [obj.position.x, obj.position.y, obj.position.z];
        if let Err(e) = kdtree.add(p, obj.object_id) {
            log::error!("error building kdtree: {}", e);
        }

        let key = obj.object_id;
        self.objects.insert(key, obj);
        self.objects.get_mut(&key).unwrap()
    }

    pub fn spawn_actor(&mut self, actor_type: ActorType) -> u32 {
        let tx = self.game_tx.clone();
        let mut rng = rand::thread_rng();

        let size = self.area_size as f32;
        let x = -size + (2.0 * rng.gen::<f32>() * size);
        let y = 0.0;
        let z = -size + (2.0 * rng.gen::<f32>() * size);

        let item = self.add_object(ObjectType::Actor, x, y, z);

        let actor = Actor::new(actor_type, item.object_id);
        let actor_id = actor.actor_id;
        let handle_actor = actor.clone();
        self.actors.insert(actor_id, actor);

        let handle = tokio::spawn(async {
            if let Err(e) = actor_main(handle_actor, tx).await {
                log::error!("actor error: {}", e);
            }
        });

        self.actor_handles.insert(actor_id, handle);

        return actor_id;
    }

    pub fn populate(&mut self, num_items: u32, num_actors: u32) {
        let mut rng = rand::thread_rng();

        let size = self.area_size as f32;
        for _n in 0..num_items {
            let x = -size + (2.0 * rng.gen::<f32>() * size);
            let y = 0.0;
            let z = -size + (2.0 * rng.gen::<f32>() * size);
            let item = self.add_object(ObjectType::Item, x, y, z);
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

        let mut rng = rand::thread_rng();
        let size = self.area_size as f32;
        let x = -size + (2.0 * rng.gen::<f32>() * size);
        let y = 0.0;
        let z = -size + (2.0 * rng.gen::<f32>() * size);
        let player_obj = self.add_object(ObjectType::Player, x, y, z);
        let player_object_id = player_obj.object_id;

        let player = Player {
            client,
            conn: client_conn,
            username,
            object_id: player_obj.object_id,
        };

        let notice = format!("Hello {}", player.username);
        player.send(GameResponse::Notice(notice));
        player.send(GameResponse::StateUpdate(StateUpdate {
            object_id: player_object_id,
            area_size: self.area_size,
            objects: self.objects.values().cloned().collect(),
            incremental: false,
        }));

        self.players.insert(client.client_id, player);

        if let Some(player_obj) = self.objects.get(&player_object_id) {
            for other in self.players.values() {
                other.send(GameResponse::StateUpdate(StateUpdate {
                    object_id: other.object_id,
                    area_size: self.area_size,
                    objects: vec![player_obj.clone()],
                    incremental: true
                }));
            }
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

    async fn handle_ping(&mut self, client: Client, timestamp: u64) {
        if let Some(player) = self.players.get(&client.client_id) {
            player.send(GameResponse::Pong(timestamp));
        }
    }

    async fn handle_move(&mut self, client: Client, x: f32, y: f32, z: f32) {
        let size = self.area_size as f32;

        if let Some(player) = self.players.get(&client.client_id) {
            if let Some(player_obj) = self.objects.get_mut(&player.object_id) {
                if let Err(e) = self.player_kdtree.remove(&[player_obj.position.x, player_obj.position.y, player_obj.position.z], &player_obj.object_id)  {
                    log::error!("error building kdtree: {}", e);
                }
                player_obj.position.x += x;
                player_obj.position.y += y;
                player_obj.position.z += z;
                if let Err(e) = self.player_kdtree.add([player_obj.position.x, player_obj.position.y, player_obj.position.z], player_obj.object_id)  {
                    log::error!("error building kdtree: {}", e);
                }

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

    fn query(&self, actor_id: u32, actor: &GameObject, object_type: ObjectType, limit: usize) -> Vec<GameObject>{
        let p = [actor.position.x, actor.position.y, actor.position.z];

        let kdtree = match object_type {
            ObjectType::Player => &self.player_kdtree,
            ObjectType::Actor => &self.actor_kdtree,
            ObjectType::Item => &self.item_kdtree,
        };

        kdtree
            .nearest(&p, limit, &squared_euclidean)
            .iter()
            .map(|pair| pair.get(0).unwrap().1)
            .map(|object_id| self.objects.get(&object_id).unwrap())
            .cloned()
            .collect()
    }

    async fn handle_scan(&mut self, actor_id: u32, response_conn: oneshot::Sender<(Actor, GameObject, Vec<GameObject>, Vec<GameObject>)>) {
        let actor = self.actors.get(&actor_id).unwrap();
        let actor_obj = self.objects.get(&actor.object_id).unwrap();

        let players;
        if self.players.len() > 0 {
            players = self.query(actor_id, actor_obj, ObjectType::Player, 10);
        } else {
            players = vec![];
        }
        let actors = self.query(actor_id, actor_obj, ObjectType::Actor, 10);

        if let Err(e) = response_conn.send((actor.clone(), actor_obj.clone(), players, actors)) {
            log::error!("error sending response: {:?}", e);
        }
    }

    async fn handle_actor_move(&mut self, actor_id: u32, x: f32, y: f32, z: f32) {
        let size = self.area_size as f32;

        if let Some(actor) = self.actors.get(&actor_id) {
            if let Some(actor_obj) = self.objects.get_mut(&actor.object_id) {

                if let Err(e) = self.actor_kdtree.remove(&[actor_obj.position.x, actor_obj.position.y, actor_obj.position.z], &actor_obj.object_id) {
                    log::error!("error building kdtree: {}", e);
                }

                actor_obj.position.x += x;
                actor_obj.position.y += y;
                actor_obj.position.z += z;

                if let Err(e) = self.actor_kdtree.add([actor_obj.position.x, actor_obj.position.y, actor_obj.position.z], actor_obj.object_id)  {
                    log::error!("error building kdtree: {}", e);
                }

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

    async fn _handle_actor_death(&mut self, actor_id: u32) {
        if let Some(handle) = self.actor_handles.remove(&actor_id) {
            if let Err(e) = handle.await {
                log::error!("error waiting for actor to reap: {}", e);
            }
        }

        let size = self.area_size as f32;
        if let Some(actor) = self.actors.remove(&actor_id) {
            if let Some(mut actor_obj) = self.objects.remove(&actor.object_id) {
                actor_obj.alive = false;

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
        self._handle_actor_death(actor_id).await;

        let size = self.area_size as f32;
        let new_actor_id = self.spawn_actor(ActorType::Walker);

        if let Some(new_object) = self.objects.get(&new_actor_id) {
            for other in self.players.values() {
                other.send(GameResponse::StateUpdate(StateUpdate {
                    object_id: other.object_id,
                    area_size: size as u32,
                    incremental: true,
                    objects: vec![new_object.clone()],
                }));
            }
        }
    }

    async fn handle_die(&mut self, actor_id: u32) {
        self._handle_actor_death(actor_id).await;
    }

    pub async fn handle_message(&mut self, msg: GameMessage) {
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
            GameMessage::Die(actor_id) => {
                self.handle_die(actor_id).await;
            },
        }
    }

    pub fn tick(&mut self) {
        self.objects.values_mut().for_each(|obj| {
            obj.position += obj.velocity;
            obj.age += 1;
        });
    }

    pub async fn process(&mut self, mut game_rx: UnboundedReceiver<GameMessage>) {

        let mut tick = 0;
        let mut last_tick = Instant::now();
        let period = Duration::from_millis(16);
        loop {

            let loop_duration = Instant::now();

            if let Ok(msg) = timeout(period/4, game_rx.recv()).await {
                if msg.is_none() {
                    break;
                }
                self.handle_message(msg.unwrap()).await;
            }

            if last_tick.elapsed() > period {
                self.tick();
                last_tick = Instant::now();
            }

            let elapsed = loop_duration.elapsed();
            if elapsed > period {
                log::debug!("tick: {} - process duration: {:?}", tick, loop_duration.elapsed());
            }

            tick += 1;
        }
    }
}
