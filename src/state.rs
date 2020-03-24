use slotmap::DenseSlotMap;

use super::body::{Body, GravityWell};
use super::object::Object;

new_key_type! {
    pub struct BodyKey;
    pub struct GravityWellKey;
    pub struct ObjectKey;
}

pub struct State {
    pub bodies: DenseSlotMap<BodyKey, Box<dyn Body>>,
    pub gravity_wells: DenseSlotMap<GravityWellKey, Box<dyn GravityWell>>,
    pub objects: DenseSlotMap<ObjectKey, Box<dyn Object>>,
}

impl State {
    pub fn new() -> State {
        State {
            bodies: DenseSlotMap::with_key(),
            gravity_wells: DenseSlotMap::with_key(),
            objects: DenseSlotMap::with_key(),
        }
    }
}
