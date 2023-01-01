
use serde::ser::{Serialize, Serializer, SerializeStruct};

use crate::game::GameObject;

#[derive(Clone, Debug)]
pub struct StateUpdate {
    pub area_size: u32,
    pub objects: Vec<GameObject>,
}

impl Serialize for StateUpdate {
   fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
       let mut state = serializer.serialize_struct("StateUpdate", 2)?;
       state.serialize_field("area_size", &self.area_size)?;
       state.serialize_field("objects", &self.objects)?;
       state.end()
    }
}
