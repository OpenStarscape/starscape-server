use super::*;

/// [Orbital Elements on Wikipedia](https://en.wikipedia.org/wiki/Orbital_elements) may be helpful
/// in understanding this struct
#[derive(Debug, Clone, Copy)]
pub struct OrbitData {
    /// Length of the semi-major axis (longest radius). Commonly a.
    pub semi_major: f64,
    /// Length of the semi-minor axis (shortest radius). Commonly b.
    pub semi_minor: f64,
    /// Tilt (in radians) of the orbital plane above the global X/Y plane. Ranges from 0 to π. Commonly i.
    pub inclination: f64,
    /// The ascending node is the point where the orbit crosses the parent's global Z position with a positive Z
    /// velocity. This property is the angle in radians of the ascending node compared to the parent on the global X/Y
    /// plane. Commonly Ω (idk wtf that is either).
    pub ascending_node: f64,
    /// The periapsis is the closest point on the orbit to the parent. This property is the angle in radians of the
    /// periapsis relative to the ascending node on the orbit plane. 0 is at the ascending node. π/2 means the periapsis
    /// is at the point on the orbit with the highest global Z value. Commonly ω.
    pub periapsis: f64,
    /// Some time at which the body was/will be at the periapsis
    pub base_time: f64,
    /// Time it takes for a full orbit to complete. Derivable from parent mass and G. Included here because it must be
    /// updated atomically with the rest of the orbit parameters.
    pub period_time: f64,
    /// The "gravity parent" of the body. Should always be the same as the dedicated property of
    /// that name. Duplicated here because it must be updated atomically with the rest of the orbit
    /// parameters.
    pub parent: EntityKey,
}

impl From<OrbitData> for Value {
    fn from(orbit: OrbitData) -> Self {
        if orbit.parent.is_null() {
            Value::Null
        } else {
            let array: Vec<Value> = vec![
                orbit.semi_major.into(),
                orbit.semi_minor.into(),
                orbit.inclination.into(),
                orbit.ascending_node.into(),
                orbit.periapsis.into(),
                orbit.base_time.into(),
                orbit.period_time.into(),
                orbit.parent.into(),
            ];
            array.into()
        }
    }
}

impl From<Value> for RequestResult<OrbitData> {
    fn from(value: Value) -> Self {
        match value {
            Value::Array(data) => {
                if data.len() == 8 {
                    let mut iter = data.into_iter();
                    Ok(OrbitData {
                        semi_major: RequestResult::<f64>::from(iter.next().unwrap())?,
                        semi_minor: RequestResult::<f64>::from(iter.next().unwrap())?,
                        inclination: RequestResult::<f64>::from(iter.next().unwrap())?,
                        ascending_node: RequestResult::<f64>::from(iter.next().unwrap())?,
                        periapsis: RequestResult::<f64>::from(iter.next().unwrap())?,
                        base_time: RequestResult::<f64>::from(iter.next().unwrap())?,
                        period_time: RequestResult::<f64>::from(iter.next().unwrap())?,
                        parent: RequestResult::<EntityKey>::from(iter.next().unwrap())?,
                    })
                } else {
                    Err(BadRequest(format!(
                        "orbit has {} elements instead of 8",
                        data.len()
                    )))
                }
            }
            _ => Err(BadRequest(format!("{:?} is not an array", value))),
        }
    }
}

/// A conduit that implements a body's orbit property
pub struct OrbitConduit {
    subscribers: SyncSubscriberList,
    body: EntityKey,
    cached_parent: Mutex<EntityKey>,
}

impl OrbitConduit {
    pub fn new(body: EntityKey) -> Self {
        Self {
            subscribers: SyncSubscriberList::new(),
            body,
            cached_parent: Mutex::new(EntityKey::null()),
        }
    }

    fn for_each_parent_subscribable<F: Fn(&dyn Subscribable)>(
        state: &State,
        parent: EntityKey,
        f: &F,
    ) -> RequestResult<()> {
        if !parent.is_null() {
            let parent_body = state.component::<Body>(parent)?;
            f(&parent_body.position);
            f(&parent_body.velocity);
            f(&parent_body.mass);
        }
        Ok(())
    }

    fn for_each_subscribable<F: Fn(&dyn Subscribable)>(
        &self,
        state: &State,
        f: &F,
    ) -> RequestResult<()> {
        let body = state.component::<Body>(self.body)?;
        Self::for_each_parent_subscribable(state, *self.cached_parent.lock().unwrap(), f)?;
        f(&body.gravity_parent);
        f(&body.position);
        f(&body.velocity);
        f(&body.mass);
        Ok(())
    }

    /// Ensures we are subscribed to the properties of the currently correct parent, and returns it
    fn update_parent(&self, state: &State) -> EntityKey {
        let parent = *state
            .component::<Body>(self.body)
            .expect("OrbitConduit body does not exist")
            .gravity_parent;
        let mut cached_parent = self.cached_parent.lock().unwrap();
        if parent != *cached_parent {
            let _ = Self::for_each_parent_subscribable(state, *cached_parent, &|s| {
                self.subscribers.unsubscribe_all(state, s);
            });
            *cached_parent = parent;
            let _ = Self::for_each_parent_subscribable(state, *cached_parent, &|s| {
                self.subscribers.subscribe_all(state, s);
            });
        }
        *cached_parent
    }
}

impl Conduit<Option<OrbitData>, ReadOnlyPropSetType> for OrbitConduit {
    fn output(&self, state: &State) -> RequestResult<Option<OrbitData>> {
        let parent = self.update_parent(state);
        let body = state.component::<Body>(self.body)?;
        if let Ok(parent_body) = state.component::<Body>(parent) {
            let gm = GRAVITATIONAL_CONSTANT * *parent_body.mass;
            let relitive_pos = *body.position - *parent_body.position;
            let relitive_vel = *body.velocity - *parent_body.velocity;
            let current_time = state.time();
            let r = relitive_pos.magnitude();
            let v = relitive_vel.magnitude();
            let up_unit = relitive_pos.cross(relitive_vel).normalize();
            let semi_major = r * gm / (2.0 * gm - r * v * v);
            // let specific_angular_momentum =
            //     r * relitive_vel.dot(up_unit.cross(relitive_pos).normalize());
            // also works:
            // let specific_angular_momentum = r
            //     * v
            //     * (relitive_vel.dot(relitive_pos)
            //         / (relitive_vel.magnitude() * relitive_pos.magnitude()))
            //     .acos()
            //     .sin();
            // let specific_orbital_energy = -gm / (2.0 * semi_major);
            let eccentricity_vec = (1.0 / gm)
                * ((v * v - (gm / r)) * relitive_pos
                    - (relitive_pos.dot(relitive_vel)) * relitive_vel);
            let eccentricity = eccentricity_vec.magnitude();
            let major_axis_unit = if ulps_eq!(eccentricity, 0.0) {
                relitive_vel.normalize()
            } else {
                eccentricity_vec / eccentricity
            };
            // also works:
            // let eccentricity =
            //     (1.0
            //         + (2.0
            //             * specific_orbital_energy
            //             * specific_angular_momentum
            //             * specific_angular_momentum)
            //             / (gm * gm))
            //         .sqrt();
            let inclination = Vector3::unit_z().dot(up_unit).acos();
            let ascending_node_unit = if ulps_eq!(up_unit, Vector3::unit_z()) {
                relitive_vel.normalize()
            } else {
                Vector3::unit_z().cross(up_unit).normalize()
            };
            let ascending_node = ascending_node_unit.y.atan2(ascending_node_unit.x);
            let semi_minor = semi_major * (1.0 - eccentricity * eccentricity).sqrt();
            let period_time = TAU * (semi_major * semi_major * semi_major / gm).sqrt();
            let minor_axis_unit = up_unit.cross(major_axis_unit).normalize();
            let periapsis = (-minor_axis_unit.dot(ascending_node_unit))
                .atan2(major_axis_unit.dot(ascending_node_unit));
            // assert!(
            //     ulps_eq!(major_axis_unit.dot(up_unit), 0.0),
            //     "eccentricity_vec: {:?}, up_unit: {:?}",
            //     eccentricity_vec,
            //     up_unit
            // );
            let orbital_plane_pos = Vector2::new(
                major_axis_unit.dot(relitive_pos),
                minor_axis_unit.dot(relitive_pos),
            );
            // assert!(
            //     ulps_eq!(orbital_plane_pos.magnitude(), relitive_pos.magnitude()),
            //     "orbital_plane_pos: {:?}, relitive_pos: {:?}",
            //     orbital_plane_pos,
            //     relitive_pos
            // );
            let center_to_focus = (semi_major * semi_major - semi_minor * semi_minor).sqrt();
            let eccentricic_anomoly = (orbital_plane_pos.y * semi_major / semi_minor)
                .atan2(orbital_plane_pos.x + center_to_focus);
            let mean_anomoly = eccentricic_anomoly - eccentricity * eccentricic_anomoly.sin();
            let base_time = current_time - mean_anomoly * period_time / TAU;
            if semi_major.is_finite()
                && semi_minor.is_finite()
                && inclination.is_finite()
                && ascending_node.is_finite()
                && periapsis.is_finite()
                && base_time.is_finite()
                && period_time.is_finite()
            {
                Ok(Some(OrbitData {
                    semi_major,
                    semi_minor,
                    inclination,
                    ascending_node,
                    periapsis,
                    base_time,
                    period_time,
                    parent,
                }))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    fn input(&self, _: &mut State, _: ReadOnlyPropSetType) -> RequestResult<()> {
        unreachable!()
    }
}

impl Subscribable for OrbitConduit {
    fn subscribe(&self, state: &State, subscriber: &Arc<dyn Subscriber>) -> RequestResult<()> {
        // If the parent isn't initialized, we could miss notifications if we don't set it up here
        self.update_parent(state);
        self.for_each_subscribable(state, &|s| {
            s.subscribe(state, subscriber)
                .or_log_error("subscribing to OrbitConduit");
        })?;
        self.subscribers.add(subscriber)?;
        Ok(())
    }

    fn unsubscribe(&self, state: &State, subscriber: &Weak<dyn Subscriber>) -> RequestResult<()> {
        // No need to update parent here, it reflects the currently subscribed to things which is
        // all that matters.
        self.for_each_subscribable(state, &|s| {
            s.unsubscribe(state, subscriber)
                .or_log_error("unsubscribing from OrbitConduit");
        })?;
        self.subscribers.remove(subscriber)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // more orbit-related tests are in ../test

    #[test]
    fn can_encode_orbit_data() {
        let e = mock_keys(1);
        let data = OrbitData {
            semi_major: 100.0,
            semi_minor: 50.0,
            inclination: 0.1,
            ascending_node: 0.2,
            periapsis: 0.3,
            base_time: 3.0,
            period_time: 5.0,
            parent: e[0],
        };
        let value: Value = data.into();
        let result = RequestResult::<(f64, f64, f64, f64, f64, f64, f64, EntityKey)>::from(value)
            .expect("failed to decode orbit data");
        assert_eq!(result, (100.0, 50.0, 0.1, 0.2, 0.3, 3.0, 5.0, e[0]));
    }

    #[test]
    fn can_decode_orbit_data() {
        let e = mock_keys(1);
        let value: Value = (100.0, 50.0, 0.1, 0.2, 0.3, 3.0, 5.0, e[0]).into();
        let result = RequestResult::<OrbitData>::from(value).expect("failed to decode orbit data");
        assert_eq!(result.semi_major, 100.0);
        assert_eq!(result.semi_minor, 50.0);
        assert_eq!(result.inclination, 0.1);
        assert_eq!(result.ascending_node, 0.2);
        assert_eq!(result.periapsis, 0.3);
        assert_eq!(result.base_time, 3.0);
        assert_eq!(result.period_time, 5.0);
        assert_eq!(result.parent, e[0]);
    }
}
