use std::sync::atomic::{AtomicI32, Ordering};
use std::collections::HashMap;

use rand::Rng;
use nalgebra::Vector3;

use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub enum ObjectType {
    Actor,
    Item,
}

static GAME_OBJECT_COUNTER: AtomicI32 = AtomicI32::new(1);

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

pub struct GameArea {
    pub area_size: u32,
    pub objects: HashMap<u32, GameObject>,
}

impl GameArea {
    pub fn new(area_size: u32) -> GameArea {
        GameArea {
            area_size,
            objects: HashMap::new(),
        }
    }

    pub fn add_object(&mut self, object_type: ObjectType) -> &mut GameObject {
        let obj = GameObject::new(object_type);
        let key = obj.object_id;
        self.objects.insert(obj.object_id, obj);
        return self.objects.get_mut(&key).unwrap();
    }

    pub fn get_object(&mut self, object_id: u32) -> Option<&mut GameObject> {
        return self.objects.get_mut(&object_id);
    }

    pub fn remove_object(&mut self, object_id: u32) {
        self.objects.remove(&object_id);
    }

    pub fn populate(&mut self, num_items: u32) {
        let mut rng = rand::thread_rng();

        let size = self.area_size;
        for _n in 0..num_items {
            let item = self.add_object(ObjectType::Item);
            item.position.x = rng.gen::<f64>() * size as f64;
            item.position.z = rng.gen::<f64>() * size as f64;
        }
    }
}
