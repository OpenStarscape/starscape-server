use super::*;

pub struct Root {
    pub time: Element<f64>,
    ship_created: Signal<Id<Body>>,
    max_connections: Element<u64>,
    current_connections: Element<u64>,
}

impl Default for Root {
    fn default() -> Self {
        Self {
            time: Element::new(0.0),
            ship_created: Signal::new(),
            max_connections: Element::new(0),
            current_connections: Element::new(0),
        }
    }
}

impl Root {
    /// Installs the root entity, must only be called once per state
    pub fn install(state: &mut State) {
        let ship_created_signal = state.root.ship_created.conduit(&state.notif_queue);

        let obj = state.object_mut(state.root()).unwrap();

        obj.add_signal(
            "ship_created",
            ship_created_signal.map_output(|iter| Ok(iter.into_iter().map(Into::into).collect())),
        );
        obj.add_action(
            "create_ship",
            ActionConduit::new(|state, (position, velocity)| {
                let ship = create_ship(state, position, velocity);
                state.root.ship_created.fire(ship);
                Ok(())
            })
            .map_input(Into::into),
        );

        obj.add_property(
            "time",
            ROConduit::new(|state| Ok(&state.root.time)).map_into::<Value, Value>(),
        );

        obj.add_property(
            "max_conn_count",
            RWConduit::new(
                |state| Ok(&state.root.max_connections),
                |state, value| Ok(state.root.max_connections.set(value)),
            )
            .map_into::<Value, Value>(),
        );

        obj.add_property(
            "conn_count",
            RWConduit::new(
                |state| Ok(&state.root.current_connections),
                |state, value| Ok(state.root.current_connections.set(value)),
            )
            .map_into::<Value, Value>(),
        );

        obj.add_property(
            "bodies",
            ComponentListConduit::<Body>::new().map_into::<Value, Value>(),
        );
    }
}
