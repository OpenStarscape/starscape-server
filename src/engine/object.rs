use super::*;

type ConduitBuilder = Box<dyn Fn(ConnectionKey) -> RequestResult<Box<dyn Conduit<Value, Value>>>>;

pub struct Object {
    type_name: &'static str,
    destroyed: Signal<()>,
    conduit_builders: HashMap<&'static str, ConduitBuilder>,
}

impl Object {
    pub fn new<T>(type_name: &'static str) -> Self
    where
        State: HasCollection<T>,
    {
        Self {
            type_name,
            destroyed: Signal::new(),
            conduit_builders: HashMap::new(),
        }
    }

    pub fn add_property<C>(&mut self, name: &'static str, conduit: C)
    where
        C: Conduit<Value, Value> + 'static,
    {
        let caching = CachingConduit::new(conduit);
        use std::collections::hash_map::Entry;
        match self.conduit_builders.entry(name) {
            Entry::Vacant(entry) => {
                entry.insert(Box::new(move |connection| {
                    Ok(PropertyConduit::new(
                        connection,
                        EntityKey::null(), // TODO
                        name,
                        caching.clone(),
                    ))
                }));
            }
            Entry::Occupied(_) => {
                error!("conduit {} added to object multiple times", name,);
            }
        }
    }

    /// Get the property of the given name
    pub fn conduit(
        &self,
        connection: ConnectionKey,
        name: &str,
    ) -> Option<RequestResult<Box<dyn Conduit<Value, Value>>>> {
        self.conduit_builders
            .get(name)
            .map(|builder| builder(connection))
    }

    pub fn destroyed_signal(
        &self,
        notif_queue: &NotifQueue,
    ) -> impl Conduit<Vec<()>, SignalsDontTakeInputSilly> {
        self.destroyed.conduit(notif_queue)
    }

    pub fn finalize(&mut self, state: &mut State) {
        self.destroyed.fire(());
    }
}

// TODO: test
