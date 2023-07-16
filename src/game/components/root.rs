use super::*;

pub struct Root {
    pub error: Signal<String>,
    pub time: Element<f64>,
    pub physics_ticks_per_network_tick: Element<u64>,
    pub physics_tick_delta: Element<f64>,
    pub min_roundtrip_time: Element<f64>,
    pub pause_at: Element<Option<f64>>,
    paused: Signal<f64>,
    ship_created: Signal<Id<Body>>,
    max_connections: Element<u64>,
    current_connections: Element<u64>,
}

impl Default for Root {
    fn default() -> Self {
        Self {
            error: Signal::new(),
            time: Element::new(0.0),
            physics_ticks_per_network_tick: Element::new(4),
            physics_tick_delta: Element::new(0.05),
            min_roundtrip_time: Element::new(0.1),
            pause_at: Element::new(None),
            paused: Signal::new(),
            ship_created: Signal::new(),
            max_connections: Element::new(0),
            current_connections: Element::new(0),
        }
    }
}

impl Root {
    /// Installs the root entity, must only be called once per state
    pub fn install(state: &mut State) {
        let error_signal = state.root.error.conduit(&state.notif_queue);
        let paused_signal = state.root.paused.conduit(&state.notif_queue);
        let ship_created_signal = state.root.ship_created.conduit(&state.notif_queue);

        let obj = state.object_mut(state.root()).unwrap();

        obj.add_signal(
            "error",
            error_signal.map_output(|_, iter| Ok(iter.into_iter().map(Into::into).collect())),
        );

        obj.add_action(
            "reset",
            ActionConduit::new(|state, ()| {
                let bodies: Vec<Id<Body>> = state.iter().map(|(id, _)| id).collect();
                for body in bodies {
                    state.remove(body)?;
                }
                Ok(())
            })
            .map_into(),
        );

        obj.add_property("time", ROConduit::new_into(|state| Ok(&state.root.time)));

        obj.add_property(
            "physics_ticks_per_network_tick",
            RWConduit::new(
                |state| Ok(&state.root.physics_ticks_per_network_tick),
                |state| Ok(&mut state.root.physics_ticks_per_network_tick),
            )
            .map_input(|state, ticks| {
                // TODO: use set_physics_ticks_per_network_tick()
                if ticks == 0 && *state.root.physics_ticks_per_network_tick > 0 {
                    state.root.paused.fire(*state.root.time);
                }
                Ok((ticks, Ok(())))
            })
            .map_into(),
        );

        obj.add_property(
            "physics_tick_delta",
            RWConduit::new(
                |state| Ok(&state.root.physics_tick_delta),
                |state| Ok(&mut state.root.physics_tick_delta),
            )
            .map_input(|_, delta: f64| {
                if delta > 0.0 && delta.is_finite() {
                    Ok((delta, Ok(())))
                } else {
                    Err(BadRequest("must be >0 and finite".into()))
                }
            })
            .map_into(),
        );

        obj.add_property(
            "min_roundtrip_time",
            RWConduit::new_into(
                |state| Ok(&state.root.min_roundtrip_time),
                |state| Ok(&mut state.root.min_roundtrip_time),
            ),
        );

        obj.add_property(
            "pause_at",
            RWConduit::new_into(
                |state| Ok(&state.root.pause_at),
                |state| Ok(&mut state.root.pause_at),
            ),
        );

        obj.add_signal(
            "paused",
            paused_signal.map_output(|_, iter| Ok(iter.into_iter().map(Into::into).collect())),
        );

        obj.add_property(
            "max_conn_count",
            RWConduit::new_into(
                |state| Ok(&state.root.max_connections),
                |state| Ok(&mut state.root.max_connections),
            ),
        );

        obj.add_property(
            "conn_count",
            RWConduit::new_into(
                |state| Ok(&state.root.current_connections),
                |state| Ok(&mut state.root.current_connections),
            ),
        );

        obj.add_signal(
            "ship_created",
            ship_created_signal
                .map_output(|_, iter| Ok(iter.into_iter().map(Into::into).collect())),
        );

        obj.add_action(
            "create_ship",
            ActionConduit::new(|state, (position, velocity)| {
                let ship = create_ship(state, position, velocity);
                state.root.ship_created.fire(ship);
                Ok(())
            })
            .map_into(),
        );

        obj.add_action(
            "create_celestial",
            ActionConduit::new(|state, mut props: HashMap<String, Value>| {
                let mut body = Body::new();
                if let Some(n) = props.remove("name") {
                    body = body.with_name(RequestResult::<String>::from(n)?);
                }
                if let Some(c) = props.remove("color") {
                    body = body.with_color(RequestResult::<ColorRGB>::from(c)?);
                }
                if let Some(p) = props.remove("position") {
                    body = body.with_position(RequestResult::<Point3<f64>>::from(p)?);
                }
                if let Some(v) = props.remove("velocity") {
                    body = body.with_velocity(RequestResult::<Vector3<f64>>::from(v)?);
                }
                if let Some(r) = props.remove("radius") {
                    body = body.with_shape(Shape::from_radius(RequestResult::<f64>::from(r)?)?);
                }
                if let Some(r) = props.remove("mass") {
                    body = body.with_mass(RequestResult::<f64>::from(r)?);
                }
                if !props.is_empty() {
                    return Err(BadRequest(format!("invalid properties: {:?}", props)));
                }
                body.install(state);
                Ok(())
            })
            .map_into(),
        );

        obj.add_property("bodies", ComponentListConduit::<Body>::new().map_into());
    }

    pub fn set_physics_ticks_per_network_tick(&mut self, ticks: u64) {
        if ticks == 0 && *self.physics_ticks_per_network_tick > 0 {
            self.paused.fire(*self.time);
        }
        self.physics_ticks_per_network_tick.set(ticks);
    }
}
