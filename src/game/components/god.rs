use super::*;

pub fn create_god(state: &mut State) -> EntityKey {
    let entity = state.create_entity();
    ComponentListConduit::<Body>::new().install(state, entity, "bodies");
    entity
}
