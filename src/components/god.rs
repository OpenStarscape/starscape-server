use super::*;

pub fn create_god(state: &mut State) -> EntityKey {
    let entity = state.create_entity();
    state.install_property(
        entity,
        "bodies",
        Box::new(ComponentListConduit::<Body>::new()),
    );
    entity
}
