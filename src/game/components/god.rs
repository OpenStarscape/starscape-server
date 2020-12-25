use super::*;

pub struct God {
    ship_created: EventElement<EntityKey>,
}

impl Default for God {
    fn default() -> Self {
        Self {
            ship_created: EventElement::new(),
        }
    }
}

impl God {
    /// Installs the god as the root entity, must only be called once per state
    pub fn install(mut self, state: &mut State) {
        let entity = state.root_entity();
        self.ship_created
            .conduit(&state.notif_queue)
            .install_event(state, entity, "ship_created");
        state.install_component(entity, self);
        ComponentListConduit::<Body>::new().install_property(state, entity, "bodies");
    }
}
