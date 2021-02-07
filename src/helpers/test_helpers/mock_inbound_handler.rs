use super::*;

#[derive(Debug, PartialEq, Clone)]
pub enum MockInbound {
    Data(Vec<u8>),
    Close,
}

#[derive(Clone)]
pub struct MockInboundHandler(Arc<Mutex<Vec<MockInbound>>>);

impl MockInboundHandler {
    pub fn new() -> Self {
        MockInboundHandler(Arc::new(Mutex::new(vec![])))
    }

    pub fn get(&self) -> Vec<MockInbound> {
        self.0.lock().unwrap().clone()
    }
}

impl InboundBundleHandler for MockInboundHandler {
    fn handle(&mut self, data: &[u8]) {
        self.0
            .lock()
            .expect("failed to lock handler mutex")
            .push(MockInbound::Data(data.to_vec()));
    }

    fn close(&mut self) {
        self.0
            .lock()
            .expect("failed to lock handler mutex")
            .push(MockInbound::Close);
    }
}
