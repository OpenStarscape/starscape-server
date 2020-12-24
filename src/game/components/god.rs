use super::*;

pub fn install_god(state: &mut State) {
    ComponentListConduit::<Body>::new().install(state, state.root_entity(), "bodies");
}
