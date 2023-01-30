use std::sync::atomic::{AtomicI32, Ordering};

use nalgebra::Vector3;

use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::oneshot;
use tokio::time::{self, Duration};

use crate::game::{GameObject, GameMessage};

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

pub async fn actor_main(actor: Actor, tx: UnboundedSender<GameMessage>) -> Result<(), Box<dyn std::error::Error>> {
    let mut interval = time::interval(Duration::from_millis(100));
    loop {
        interval.tick().await;

        let (sender, receiver) = oneshot::channel::<(Actor, GameObject, Vec<GameObject>, Vec<GameObject>)>();
        tx.send(GameMessage::Scan(actor.actor_id, sender))?;
        let (_, actor_obj, players, actors) = receiver.await?;

        let mut dir = Vector3::new(0.0, 0.0, 0.0);
        if let Some(player) = players.iter().nth(0) {
            dir = (player.position - actor_obj.position).normalize() * 2.0;
        }

        // let (sender, receiver) = oneshot::channel::<(Actor, GameObject, Vec<GameObject>)>();
        // tx.send(GameMessage::Scan(actor.actor_id, ObjectType::Actor, sender))?;
        // let (_, actor_obj, actors) = receiver.await?;

        if dir.magnitude() > 0.0 {
            tx.send(GameMessage::ActorMove(actor.actor_id, dir.x as f32, dir.y as f32, dir.z as f32))?
        }
    }
}
