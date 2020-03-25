use slotmap::DenseSlotMap;

use super::body::{Body, GravityWell};

new_key_type! {
    pub struct BodyKey;
    pub struct GravityWellKey;
}

pub struct State {
    pub bodies: DenseSlotMap<BodyKey, Body>,
    pub gravity_wells: DenseSlotMap<GravityWellKey, GravityWell>,
}

impl State {
    pub fn new() -> State {
        State {
            bodies: DenseSlotMap::with_key(),
            gravity_wells: DenseSlotMap::with_key(),
        }
    }
}
