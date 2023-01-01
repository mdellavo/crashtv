use rand::Rng;
use nalgebra::Vector3;

use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize)]
pub enum ObjectType {
    Player,
    Enemy,
    Item,
}

#[derive(Clone, Debug, Serialize)]
pub struct GameObject {
    object_type: ObjectType,
    position: Vector3<f64>,
    velocity: Vector3<f64>,
}

impl GameObject {
    pub fn new(object_type: ObjectType) -> GameObject {
        GameObject {
            object_type,
            position: Vector3::new(0.0, 0.0, 0.0),
            velocity: Vector3::new(0.0, 0.0, 0.0),
        }
    }
}

pub struct GameArea {
    pub area_size: u32,
    pub objects: Vec<GameObject>,
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
            let mut item = GameObject::new(ObjectType::Item);
            item.position.x = rng.gen::<f64>() * self.area_size as f64;
            item.position.z = rng.gen::<f64>() * self.area_size as f64;
            self.objects.push(item);
        }
    }
}
