extern crate serde;

#[macro_use]
extern crate serde_derive;
extern crate rmp_serde as rmps;

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use rmps::{Deserializer, Serializer};

pub mod game;

use game::GameObject;

#[derive(Debug, Deserialize, Serialize)]
struct StateUpdate {
    area_size: u32,
    objects: Vec<GameObject>
}
