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
        ActionConduit::new(move |state, (position, velocity)| {
            let ship = create_ship(state, position, velocity);
            state.component_mut::<God>(entity)?.ship_created.fire(ship);
            Ok(())
        })
        .install_action(state, entity, "create_ship");
        state.install_component(entity, self);
        ComponentListConduit::<Body>::new().install_property(state, entity, "bodies");
    }
}
