use rand::Rng;
use nalgebra::Vector3;

use std::sync::Arc;

pub trait GameObject {

}

pub struct Player {
    position: Vector3<f64>,
    velocity: Vector3<f64>,
}

impl Player {
    pub fn new() -> Player {
        Player {
            position: Vector3::new(0.0, 0.0, 0.0),
            velocity: Vector3::new(0.0, 0.0, 0.0),
        }
    }
}

impl GameObject for Player {}

pub struct Item {
    position: Vector3<f64>,
    velocity: Vector3<f64>,
}

impl Item {
    pub fn new() -> Item {
        Item {
            position: Vector3::new(0.0, 0.0, 0.0),
            velocity: Vector3::new(0.0, 0.0, 0.0),
        }
    }
}

impl GameObject for Item {}

pub struct Enemy {
    position: Vector3<f64>,
    velocity: Vector3<f64>,
}

impl Enemy {
    pub fn new() -> Enemy {
        Enemy {
            position: Vector3::new(0.0, 0.0, 0.0),
            velocity: Vector3::new(0.0, 0.0, 0.0),
        }
    }
}

impl GameObject for Enemy {}

pub struct GameArea {
    area_size: u32,
    objects: Vec<Arc<dyn GameObject>>,
}

impl GameArea {
    pub fn new(area_size: u32) -> GameArea {
        GameArea {
            area_size,
            objects: Vec::new(),
        }
    }

    pub fn populate(&mut self, num_items: u32) {
        let mut rng = rand::thread_rng();

        for _n in 0..num_items {
            let mut item = Item::new();
            item.position.x = rng.gen::<f64>() * self.area_size as f64;
            item.position.z = rng.gen::<f64>() * self.area_size as f64;
            self.objects.push(Arc::new(item));
        }
    }
}

unsafe impl Send for GameArea {}
unsafe impl Sync for GameArea {}
