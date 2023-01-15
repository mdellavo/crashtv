use crate::game::GameObject;

#[derive(Clone, Debug, Serialize)]
pub struct StateUpdate {
    pub object_id: u32,
    pub area_size: u32,
    pub objects: Vec<GameObject>,
}

