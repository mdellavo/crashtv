extern crate fps_counter;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::ops::AddAssign;
use std::sync::atomic::{AtomicI32, Ordering};

use nalgebra::Vector3;

use rand::Rng;

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::{Duration, Instant};

use serde::Serialize;

use fps_counter::FPSCounter;

use bevy_ecs::prelude::*;

use crate::actor::{actor_main, Actor, ActorType};
use crate::net::StateUpdate;
use crate::terrain::{Terrain, TerrainType};

use crate::data_structs::BinLattice;

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

    // Game Messages
    Tick(Instant),

    // Actor Messages
    Die(u32),
    Respawn(u32),
    Scan(
        u32,
        oneshot::Sender<(
            Actor,
            FrozenGameObject,
            Vec<FrozenGameObject>,
            Vec<FrozenGameObject>,
        )>,
    ),
    ActorMove(u32, f32, f32, f32),
}

#[derive(Debug, Serialize)]
pub enum GameResponse {
    Error(u32, String),
    StateUpdate(StateUpdate),
    Pong(u64),
    Goodbye(),
    Notice(String),
    ElevationMap(u32, u32, Vec<f32>),
    TerrainMap(u32, u32, Vec<u8>),
}

impl Player {
    pub fn send(&self, response: GameResponse) {
        let result = self.conn.send(response);
        if let Err(e) = result {
            log::error!("game response write error {:?}: {}", self.client, e);
        }
    }
}

#[derive(Component, Debug, Copy, Clone)]
struct Position {
    value: Vector3<f32>,
}

#[derive(Component, Debug, Copy, Clone)]
struct Velocity {
    value: Vector3<f32>,
}

#[derive(Component, Debug, Copy, Clone)]
struct Acceleration {
    value: Vector3<f32>,
}

#[derive(Component)]
struct Alive;

#[derive(Clone, Debug, Serialize)]
pub struct GameObject {
    pub alive: bool,
    pub age: u32,
    pub object_id: u32,
    pub entity: Entity,
    pub object_type: ObjectType,
    pub health: u8,
}

#[derive(Clone, Debug, Serialize)]
pub struct FrozenGameObject {
    pub object: GameObject,
    pub position: Vector3<f32>,
    pub velocity: Vector3<f32>,
    pub acceleration: Vector3<f32>,
}

impl GameObject {
    pub fn new(object_type: ObjectType, entity: Entity) -> GameObject {
        GameObject {
            object_type,
            entity,
            alive: true,
            age: 0,
            object_id: GAME_OBJECT_COUNTER.fetch_add(1, Ordering::Relaxed) as u32,
            health: 100,
        }
    }
}

pub struct GameArea {
    pub world: World,
    pub schedule: Schedule,
    pub terrain: Terrain,
    pub entities: HashMap<Entity, u32>, // maps entity id to object id
    pub objects: HashMap<u32, GameObject>,
    pub actors: HashMap<u32, Actor>,
    pub actor_handles: HashMap<u32, JoinHandle<()>>,
    pub players: HashMap<u32, Player>,
    pub game_tx: UnboundedSender<GameMessage>,
    pub actor_index: BinLattice,
    pub ticks: u32,
    pub last_tick: Instant,
    pub fps_counter: FPSCounter,
}

impl GameArea {
    pub fn new(area_size: u32, game_tx: UnboundedSender<GameMessage>) -> GameArea {
        let mut area = GameArea {
            world: World::new(),
            schedule: Schedule::default(),
            terrain: Terrain::new(area_size),
            entities: HashMap::new(),
            objects: HashMap::new(),
            actors: HashMap::new(),
            players: HashMap::new(),
            actor_handles: HashMap::new(),
            game_tx,
            actor_index: BinLattice::new(50),
            ticks: 0,
            last_tick: Instant::now(),
            fps_counter: FPSCounter::default(),
        };

        area.schedule.add_systems(
            |mut query: Query<(Entity, &mut Position, &mut Velocity, &Acceleration)>| {
                for (entity, mut position, mut velocity, acceleration) in &mut query {
                    velocity.value.add_assign(acceleration.value);
                    position.value.add_assign(velocity.value);
                }
            },
        );

        area
    }

    fn freeze_game_object(&self, object: &GameObject) -> FrozenGameObject {
        let pos = self.world.entity(object.entity).get::<Position>().unwrap();
        let vel = self.world.entity(object.entity).get::<Velocity>().unwrap();
        let accel = self
            .world
            .entity(object.entity)
            .get::<Acceleration>()
            .unwrap();

        FrozenGameObject {
            object: object.clone(),
            position: pos.value.clone(),
            velocity: vel.value.clone(),
            acceleration: accel.value.clone(),
        }
    }

    pub fn has_username(&self, username: &String) -> bool {
        let usernames: Vec<String> = self.players.values().map(|x| x.username.clone()).collect();
        usernames.iter().any(|x| x.eq(username))
    }

    pub fn add_object(
        &mut self,
        object_type: ObjectType,
        entity: Entity,
        x: f32,
        y: f32,
        z: f32,
    ) -> &mut GameObject {
        let obj = GameObject::new(object_type.clone(), entity);
        let key = obj.object_id;
        self.entities.insert(entity, obj.object_id);
        self.objects.insert(key, obj);
        self.objects.get_mut(&key).unwrap()
    }

    pub fn add_item(&mut self, x: f32, y: f32, z: f32) -> &mut GameObject {
        let entity = self
            .world
            .spawn((
                Position {
                    value: Vector3::new(x, y, z),
                },
                Velocity {
                    value: Vector3::new(0.0, 0.0, 0.0),
                },
                Acceleration {
                    value: Vector3::new(0.0, 0.0, 0.0),
                },
            ))
            .id();

        let obj = self.add_object(ObjectType::Item, entity, x, y, z);
        obj
    }

    pub fn add_actor(&mut self, x: f32, y: f32, z: f32) -> &mut GameObject {
        let entity = self
            .world
            .spawn((
                Alive,
                Position {
                    value: Vector3::new(x, y, z),
                },
                Velocity {
                    value: Vector3::new(1.0, 0.0, 1.0),
                },
                Acceleration {
                    value: Vector3::new(0.0, 0.0, 0.0),
                },
            ))
            .id();

        let obj = self.add_object(ObjectType::Actor, entity, x, y, z);
        obj
    }

    pub fn add_player(&mut self, x: f32, y: f32, z: f32) -> &mut GameObject {
        let entity = self
            .world
            .spawn((
                Alive,
                Position {
                    value: Vector3::new(x, y, z),
                },
                Velocity {
                    value: Vector3::new(0.0, 0.0, 0.0),
                },
                Acceleration {
                    value: Vector3::new(0.0, 0.0, 0.0),
                },
            ))
            .id();

        let obj = self.add_object(ObjectType::Player, entity, x, y, z);
        obj
    }

    pub fn spawn_actor(&mut self, actor_type: ActorType) -> u32 {
        let tx = self.game_tx.clone();
        let mut rng = rand::thread_rng();

        let size = self.terrain.size as f32;
        let x = rng.gen::<f32>() * size;
        let z = rng.gen::<f32>() * size;

        let elevation = self.terrain.get_elevation(
            x.clamp(0.0, size - 1.0) as u32,
            z.clamp(0.0, size - 1.0) as u32,
        );
        let y = elevation;

        let obj = self.add_actor(x, y, z);

        let object_id = obj.object_id;
        self.actor_index.put(x, z, object_id);
        let actor = Actor::new(actor_type, object_id);
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

        let size = self.terrain.size as f32;
        for _n in 0..num_items {
            let x = rng.gen::<f32>() * size;
            let y = 0.0;
            let z = rng.gen::<f32>() * size;
            self.add_item(x, y, z);
        }

        for _n in 0..num_actors {
            self.spawn_actor(ActorType::Walker);
        }
    }

    async fn handle_hello(
        &mut self,
        client: Client,
        client_conn: UnboundedSender<GameResponse>,
        username: String,
    ) {
        if self.players.contains_key(&client.client_id) {
            let result = client_conn.send(GameResponse::Error(1, "Incorrect hello".to_string()));
            if let Err(e) = result {
                log::error!("game response write error {:?}: {}", client, e);
            }
            return;
        }

        if self.has_username(&username) {
            let result =
                client_conn.send(GameResponse::Error(1, "Username already taken".to_string()));
            if let Err(e) = result {
                log::error!("game response write error {:?}: {}", client, e);
            }
            return;
        }

        let mut rng = rand::thread_rng();
        let size = self.terrain.size as f32;
        let x = rng.gen::<f32>() * size;
        let y = 0.0;
        let z = rng.gen::<f32>() * size;
        let player_obj = self.add_player(x, y, z);
        let player_object_id = player_obj.object_id;

        let player = Player {
            client,
            conn: client_conn,
            username,
            object_id: player_obj.object_id,
        };

        player.send(GameResponse::ElevationMap(
            self.terrain.size,
            self.terrain.size,
            self.terrain.elevation_map.clone(),
        ));
        player.send(GameResponse::TerrainMap(
            self.terrain.size,
            self.terrain.size,
            self.terrain.terrain_map.clone(),
        ));

        let notice = format!("Hello {}", player.username);
        player.send(GameResponse::Notice(notice));
        player.send(GameResponse::StateUpdate(StateUpdate {
            object_id: player_object_id,
            area_size: self.terrain.size,
            objects: self
                .objects
                .values()
                .map(|obj| self.freeze_game_object(obj))
                .collect(),
            incremental: false,
        }));

        self.players.insert(client.client_id, player);

        if let Some(player_obj) = self.objects.get(&player_object_id) {
            for other in self.players.values() {
                other.send(GameResponse::StateUpdate(StateUpdate {
                    object_id: other.object_id,
                    area_size: self.terrain.size,
                    objects: vec![self.freeze_game_object(player_obj)],
                    incremental: true,
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
                        area_size: self.terrain.size,
                        objects: vec![self.freeze_game_object(&player_obj)],
                        incremental: true,
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
        if let Some(player) = self.players.get(&client.client_id) {
            if let Some(player_obj) = self.objects.get(&player.object_id) {
                let mut entity = self.world.entity_mut(player_obj.entity);
                let mut velocity = entity.get_mut::<Velocity>().unwrap();
                velocity.value.x = 10. * x;
                velocity.value.y = 10. * y;
                velocity.value.z = 10. * z;

                for other in self.players.values() {
                    other.send(GameResponse::StateUpdate(StateUpdate {
                        object_id: other.object_id,
                        area_size: self.terrain.size as u32,
                        incremental: true,
                        objects: vec![self.freeze_game_object(player_obj)],
                    }));
                }
            }
        }
    }

    fn query(
        &self,
        actor_id: u32,
        actor: &GameObject,
        object_type: ObjectType,
        limit: usize,
    ) -> Vec<GameObject> {
        let actor_pos = self.world.entity(actor.entity).get::<Position>().unwrap();

        let object_ids = self
            .actor_index
            .get_nearby(actor_pos.value.x, actor_pos.value.z, 50.0);
        let mut objects: Vec<GameObject> = object_ids
            .iter()
            .filter(|object_id| **object_id != actor.object_id)
            .flat_map(|object_id| self.objects.get(object_id))
            .cloned()
            .collect();

        objects.sort_by(|a, b| {
            let a_pos = self.world.entity(a.entity).get::<Position>().unwrap();
            let b_pos = self.world.entity(b.entity).get::<Position>().unwrap();

            let dist_a = a_pos.value.metric_distance(&actor_pos.value);
            let dist_b = b_pos.value.metric_distance(&actor_pos.value);
            dist_a
                .partial_cmp(&dist_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        objects.iter().take(limit).cloned().collect()
    }

    async fn handle_scan(
        &mut self,
        actor_id: u32,
        response_conn: oneshot::Sender<(
            Actor,
            FrozenGameObject,
            Vec<FrozenGameObject>,
            Vec<FrozenGameObject>,
        )>,
    ) {
        let actor = self.actors.get(&actor_id).unwrap();
        let actor_obj = self.objects.get(&actor.object_id).unwrap();
        let actor_pos = self
            .world
            .entity(actor_obj.entity)
            .get::<Position>()
            .unwrap();
        let actor_vel = self
            .world
            .entity(actor_obj.entity)
            .get::<Velocity>()
            .unwrap();
        let actor_accel = self
            .world
            .entity(actor_obj.entity)
            .get::<Acceleration>()
            .unwrap();

        let frozen_actor = FrozenGameObject {
            object: actor_obj.clone(),
            position: actor_pos.value,
            velocity: actor_vel.value,
            acceleration: actor_accel.value,
        };

        let players: Vec<FrozenGameObject> = self
            .players
            .values()
            .flat_map(|player| self.objects.get(&player.object_id))
            .filter(|player| {
                let player_pos = self.world.entity(player.entity).get::<Position>().unwrap();
                player_pos.value.metric_distance(&actor_pos.value) < 100.0
            })
            .map(|player| self.freeze_game_object(player))
            .collect();
        let actors: Vec<FrozenGameObject> = self
            .query(actor_id, &actor_obj, ObjectType::Actor, 20)
            .iter()
            .map(|actor| self.freeze_game_object(actor))
            .collect();

        if let Err(e) = response_conn.send((actor.clone(), frozen_actor, players, actors)) {
            log::error!("error sending response: {:?}", e);
        }
    }

    async fn handle_actor_move(&mut self, actor_id: u32, x: f32, y: f32, z: f32) {
        if let Some(actor) = self.actors.get(&actor_id) {
            if let Some(actor_obj) = self.objects.get(&actor.object_id) {
                // FIXME send impulse to actor

                // log::debug!("accel: {:?} / vel: {:?} / pos: {:?}", actor_obj.acceleration, actor_obj.velocity, actor_obj.position);

                for other in self.players.values() {
                    other.send(GameResponse::StateUpdate(StateUpdate {
                        object_id: other.object_id,
                        area_size: self.terrain.size as u32,
                        incremental: true,
                        objects: vec![self.freeze_game_object(actor_obj)],
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

        let size = self.terrain.size as f32;
        if let Some(actor) = self.actors.remove(&actor_id) {
            if let Some(mut actor_obj) = self.objects.remove(&actor.object_id) {
                actor_obj.alive = false;

                for other in self.players.values() {
                    other.send(GameResponse::StateUpdate(StateUpdate {
                        object_id: other.object_id,
                        area_size: size as u32,
                        incremental: true,
                        objects: vec![self.freeze_game_object(&actor_obj)],
                    }));
                }
            }
        }
    }

    async fn handle_respawn(&mut self, actor_id: u32) {
        self._handle_actor_death(actor_id).await;

        let size = self.terrain.size as f32;
        let new_actor_id = self.spawn_actor(ActorType::Walker);

        if let Some(new_object) = self.objects.get(&new_actor_id) {
            for other in self.players.values() {
                other.send(GameResponse::StateUpdate(StateUpdate {
                    object_id: other.object_id,
                    area_size: size as u32,
                    incremental: true,
                    objects: vec![self.freeze_game_object(new_object)],
                }));
            }
        }
    }

    async fn handle_die(&mut self, actor_id: u32) {
        self._handle_actor_death(actor_id).await;
    }

    async fn handle_tick(&mut self, tick_time: Instant) {
        let now = Instant::now();
        let delta = now - self.last_tick;

        if delta >= Duration::from_millis(16) {
            self.ticks += 1;
            self.tick(delta);
            self.last_tick = now;
            let fps = self.fps_counter.tick();
            if self.ticks % 63 == 0 {
                log::debug!("ticks: {}", fps);
            }
        }
    }

    pub async fn handle_message(&mut self, msg: GameMessage) {
        match msg {
            GameMessage::Hello(client, client_conn, username) => {
                self.handle_hello(client, client_conn, username).await;
            }
            GameMessage::Goodbye(client) => {
                self.handle_goodbye(client).await;
            }
            GameMessage::Ping(client, timestamp) => {
                self.handle_ping(client, timestamp).await;
            }
            GameMessage::Move(client, x, y, z) => {
                self.handle_move(client, x, y, z).await;
            }
            GameMessage::Scan(actor_id, response_conn) => {
                self.handle_scan(actor_id, response_conn).await;
            }
            GameMessage::ActorMove(actor_id, x, y, z) => {
                self.handle_actor_move(actor_id, x, y, z).await;
            }
            GameMessage::Respawn(actor_id) => {
                self.handle_respawn(actor_id).await;
            }
            GameMessage::Die(actor_id) => {
                self.handle_die(actor_id).await;
            }
            GameMessage::Tick(tick_time) => {
                self.handle_tick(tick_time).await;
            }
        }
    }

    pub fn tick(&mut self, elapsed: Duration) {
        self.schedule.run(&mut self.world);

        self.objects.values_mut().for_each(|obj| {
            // if is_actor {
            //     self.actor_index.remove(obj.position.x, obj.position.z, obj.object_id);
            // }

            // if obj.acceleration.magnitude() > 50.0 {
            //     obj.acceleration = obj.acceleration.normalize() * 50.0;
            // }

            // if obj.acceleration.magnitude() < 0.1 {
            //     obj.acceleration.x = 0.0;
            //     obj.acceleration.y = 0.0;
            //     obj.acceleration.z = 0.0;
            // }

            // obj.velocity += obj.acceleration;
            // if obj.velocity.magnitude() > 100.0 {
            //     obj.velocity = obj.velocity.normalize() * 100.0;
            // }

            // if obj.velocity.magnitude() < 0.1 {
            //     obj.velocity.x = 0.0;
            //     obj.velocity.y = 0.0;
            //     obj.velocity.z = 0.0;
            // }

            // obj.position += obj.velocity;

            // let elevation = self.terrain.get_elevation(obj.position.x.clamp(0.0, self.terrain.size as f32 - 1.0) as u32,
            //                                            obj.position.z.clamp(0.0, self.terrain.size as f32 - 1.0) as u32);
            // obj.position.y = elevation;

            // if is_actor {
            //     self.actor_index.put(obj.position.x, obj.position.z, obj.object_id);
            // }

            // log::debug!("pos: {:?} | vel: {:?}", obj.position, obj.velocity);

            obj.age += 1;
        });
    }

    pub async fn process(&mut self, mut game_rx: UnboundedReceiver<GameMessage>) {
        loop {
            if let Some(msg) = game_rx.recv().await {
                self.handle_message(msg).await;
            }
        }
    }
}
