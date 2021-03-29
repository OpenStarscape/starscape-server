use super::*;

pub struct God {
    pub time: Element<f64>,
    ship_created: Signal<EntityKey>,
    max_connections: Element<u64>,
    current_connections: Element<u64>,
}

impl Default for God {
    fn default() -> Self {
        Self {
            time: Element::new(0.0),
            ship_created: Signal::new(),
            max_connections: Element::new(0),
            current_connections: Element::new(0),
        }
    }
}

impl God {
    /// Installs the god as the root entity, must only be called once per state
    pub fn install(mut self, state: &mut State) {
        let entity = state.root_entity();

        self.ship_created
            .conduit(&state.notif_queue)
            .install_signal(state, entity, "ship_created");
        ActionConduit::new(move |state, (position, velocity)| {
            let ship = create_ship(state, position, velocity);
            state.component_mut::<God>(entity)?.ship_created.fire(ship);
            Ok(())
        })
        .install_action(state, entity, "create_ship");

        ROConduit::new(move |state| Ok(&state.component::<God>(entity)?.time))
            .install_property(state, entity, "time");

        RWConduit::new(
            move |state| Ok(&state.component::<God>(entity)?.max_connections),
            move |state, value| {
                Ok(state
                    .component_mut::<God>(entity)?
                    .max_connections
                    .set(value))
            },
        )
        .install_property(state, entity, "max_conn_count");

        RWConduit::new(
            move |state| Ok(&state.component::<God>(entity)?.current_connections),
            move |state, value| {
                Ok(state
                    .component_mut::<God>(entity)?
                    .current_connections
                    .set(value))
            },
        )
        .install_property(state, entity, "conn_count");

        ComponentListConduit::<Body>::new().install_property(state, entity, "bodies");

        state.install_component(entity, self);
    }
}
