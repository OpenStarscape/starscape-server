use super::*;

pub struct MockEventHandler(pub RefCell<Vec<(ConnectionKey, Event)>>);

impl MockEventHandler {
    pub fn new() -> Self {
        Self(RefCell::new(Vec::new()))
    }
}

impl EventHandler for MockEventHandler {
    fn event(&self, _handler: &dyn RequestHandler, connection: ConnectionKey, event: Event) {
        self.0.borrow_mut().push((connection, event));
    }
}
