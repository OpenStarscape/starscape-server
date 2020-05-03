use crate::connection::Encodable;
use crate::entity::Entity;
use crate::plumbing::{new_conduit_property, Conduit};
use crate::state::{EntityKey, PropertyKey, State};

struct BodyListConduit {}

impl Conduit for BodyListConduit {
    fn get_value(&self, state: &State) -> Result<Encodable, String> {
        let entities: Vec<EntityKey> = state.bodies.values().map(|body| body.entity).collect();
        Ok(entities.into())
    }

    fn set_value(&self, _state: &mut State, _value: ()) -> Result<(), String> {
        Err("read_only_property".into())
    }

    fn connect(&self, state: &State, property: PropertyKey) -> Result<(), String> {
        state.bodies.connect(property).map_err(|e| {
            eprintln!("Error: {}", e);
            "server_error".into()
        })
    }

    fn disconnect(&self, state: &State, property: PropertyKey) -> Result<(), String> {
        state.bodies.disconnect(property).map_err(|e| {
            eprintln!("Error: {}", e);
            "server_error".into()
        })
    }
}

pub fn create_god(state: &mut State) -> EntityKey {
    let entity = state.entities.insert(Entity::new());
    new_conduit_property(state, entity, "bodies", Box::new(BodyListConduit {}));
    entity
}
