use std::ops::Div;
use std::ops::Sub;
use std::sync::atomic::{AtomicI32, Ordering};

use nalgebra::Vector3;

use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::oneshot;
use tokio::time::{self, Duration};

use crate::game::{FrozenGameObject, GameMessage};

static ACTOR_COUNTER: AtomicI32 = AtomicI32::new(1);

#[derive(Clone, Debug)]
pub enum ActorType {
    Walker,
    Bullet,
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

fn compute_alignment(actor: &FrozenGameObject, others: &Vec<FrozenGameObject>) -> Vector3<f32> {
    let mut average_velocity = Vector3::new(0.0, 0.0, 0.0);
    let len = others.len();
    if len == 0 {
        return average_velocity;
    }
    for i in others {
        average_velocity += i.velocity;
    }
    average_velocity = average_velocity.div(len as f32);
    average_velocity.sub(actor.velocity) / 2.5
}

fn compute_cohesion(actor: &FrozenGameObject, others: &Vec<FrozenGameObject>) -> Vector3<f32> {
    let len = others.len();

    let mut average_position = Vector3::new(0.0, 0.0, 0.0);
    if len == 0 {
        return average_position;
    }

    for i in others {
        average_position += i.position;
    }
    average_position = average_position.div(len as f32);
    average_position.sub(actor.position) / 50.0
}

fn compute_separation(actor: &FrozenGameObject, others: &Vec<FrozenGameObject>) -> Vector3<f32> {
    let mut separation = Vector3::new(0.0, 0.0, 0.0);
    for i in others {
        let distance = actor.position.metric_distance(&i.position);
        if distance == 0.0 {
            continue;
        }
        let difference_vec = i
            .position
            .sub(actor.position)
            .div(actor.position.metric_distance(&i.position) * 2.0);
        separation -= difference_vec;
    }
    separation * 2.5
}

fn compute_attack(actor: &FrozenGameObject, players: &Vec<FrozenGameObject>) -> Vector3<f32> {
    if players.len() == 0 {
        return Vector3::new(0.0, 0.0, 0.0);
    }

    let mut average_velocity = Vector3::new(0.0, 0.0, 0.0);
    let mut average_position = Vector3::new(0.0, 0.0, 0.0);

    for player in players {
        average_position += player.position;
        average_velocity += player.velocity;
    }

    average_position /= players.len() as f32;
    average_velocity /= players.len() as f32;

    (average_velocity.sub(actor.velocity) + (average_position.sub(actor.position))) / 25.0
}

pub async fn actor_main(
    actor: Actor,
    tx: UnboundedSender<GameMessage>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut interval = time::interval(Duration::from_millis(100));
    loop {
        interval.tick().await;

        let (sender, receiver) = oneshot::channel::<(
            Actor,
            FrozenGameObject,
            Vec<FrozenGameObject>,
            Vec<FrozenGameObject>,
        )>();
        tx.send(GameMessage::Scan(actor.actor_id, sender))?;
        let (_, actor_obj, players, actors) = receiver.await?;

        let alignment = compute_alignment(&actor_obj, &actors);
        let cohesion = compute_cohesion(&actor_obj, &actors);
        let separation = compute_separation(&actor_obj, &actors);
        let attack = compute_attack(&actor_obj, &players);

        let dir = alignment + cohesion + separation + attack;

        // log::debug!("align: {:?} | cohe: {:?} | sep: {:?} -> {:?}", alignment, cohesion, separation, dir);

        if !(dir.x.is_nan() && dir.y.is_nan() && dir.z.is_nan()) {
            //log::debug!("dir: {:?}", dir);
            tx.send(GameMessage::ActorMove(
                actor.actor_id,
                dir.x as f32,
                dir.y as f32,
                dir.z as f32,
            ))?;
        }
    }
}
