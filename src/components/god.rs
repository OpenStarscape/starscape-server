use std::sync::{Arc, Weak};

use crate::{
    plumbing::{new_conduit_property, Conduit, NotificationSink},
    server::{Decodable, Encodable},
    state::{EntityKey, State},
};

#[derive(Clone)]
struct BodyListConduit {}

impl Conduit for BodyListConduit {
    fn get_value(&self, state: &State) -> Result<Encodable, String> {
        let entities: Vec<EntityKey> = state.bodies.values().map(|body| body.entity).collect();
        Ok(entities.into())
    }

    fn set_value(&self, _state: &mut State, _value: &Decodable) -> Result<(), String> {
        Err("read_only_property".into())
    }

    fn subscribe(
        &self,
        state: &State,
        subscriber: &Arc<dyn NotificationSink>,
    ) -> Result<(), String> {
        state.bodies.subscribe(subscriber).map_err(|e| {
            eprintln!("Error: {}", e);
            "server_error".into()
        })
    }

    fn unsubscribe(
        &self,
        state: &State,
        subscriber: &Weak<dyn NotificationSink>,
    ) -> Result<(), String> {
        state.bodies.unsubscribe(subscriber).map_err(|e| {
            eprintln!("Error: {}", e);
            "server_error".into()
        })
    }
}

pub fn create_god(state: &mut State) -> EntityKey {
    let entity = state.entities.new_entity();
    new_conduit_property(state, entity, "bodies", Box::new(BodyListConduit {}));
    entity
}
