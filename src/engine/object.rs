use super::*;

type ConduitBuilder = Box<dyn Fn(ConnectionKey) -> RequestResult<Box<dyn Conduit<Value, Value>>>>;

pub struct Object {
    id: GenericId,
    destroyed: Signal<()>,
    conduit_builders: HashMap<&'static str, ConduitBuilder>,
}

impl Object {
    pub fn new(id: GenericId) -> Self {
        Self {
            id,
            destroyed: Signal::new(),
            conduit_builders: HashMap::new(),
        }
    }

    pub fn add_conduit(&mut self, name: &'static str, builder: ConduitBuilder) {
        use std::collections::hash_map::Entry;
        match self.conduit_builders.entry(name) {
            Entry::Vacant(entry) => {
                entry.insert(builder);
            }
            Entry::Occupied(_) => {
                error!("conduit {} added to object multiple times", name);
            }
        }
    }

    pub fn add_property<C>(&mut self, name: &'static str, conduit: C)
    where
        C: Conduit<Value, Value> + 'static,
    {
        let caching = CachingConduit::new(conduit);
        let id = self.id; // TODO drop in Rust 2021
        self.add_conduit(
            name,
            Box::new(move |connection| {
                Ok(PropertyConduit::new(connection, id, name, caching.clone()))
            }),
        );
    }

    pub fn add_signal<C>(&mut self, name: &'static str, conduit: C)
    where
        C: Conduit<Vec<Value>, SignalsDontTakeInputSilly> + 'static,
    {
        let conduit = Arc::new(conduit) as Arc<dyn Conduit<Vec<Value>, SignalsDontTakeInputSilly>>;
        let id = self.id; // TODO drop in Rust 2021
        self.add_conduit(
            name,
            Box::new(move |connection| {
                Ok(SignalConduit::new(connection, id, name, conduit.clone()))
            }),
        );
    }

    pub fn add_action<C>(&mut self, name: &'static str, conduit: C)
    where
        C: Conduit<ActionsDontProduceOutputSilly, Value> + 'static,
    {
        let conduit =
            Arc::new(conduit.map_output(|_| unreachable!())) as Arc<dyn Conduit<Value, Value>>;
        let id = self.id; // TODO drop in Rust 2021
        self.add_conduit(
            name,
            Box::new(move |connection| {
                Ok(PropertyConduit::new(connection, id, name, conduit.clone()))
            }),
        );
    }

    /// Get the property of the given name
    pub fn conduit(
        &self,
        connection: ConnectionKey,
        name: &str,
    ) -> RequestResult<Box<dyn Conduit<Value, Value>>> {
        self.conduit_builders
            .get(name)
            .map(|builder| builder(connection))
            .unwrap_or_else(|| Err(BadName(self.id, name.to_string())))
    }

    pub fn destroyed_signal(
        &self,
        notif_queue: &NotifQueue,
    ) -> impl Conduit<Vec<()>, SignalsDontTakeInputSilly> {
        self.destroyed.conduit(notif_queue)
    }

    pub fn finalize(&mut self, _state: &mut State) {
        self.destroyed.fire(());
    }
}

// TODO: test
