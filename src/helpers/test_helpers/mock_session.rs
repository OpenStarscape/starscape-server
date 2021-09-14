use super::*;

#[derive(Debug)]
struct MockSessionInner {
    pub bundles: Vec<Vec<u8>>,
    pub should_error: bool,
    pub is_closed: bool,
}

#[derive(Debug, Clone)]
pub struct MockSession(Arc<Mutex<MockSessionInner>>);

impl MockSession {
    pub fn new(should_error: bool) -> Self {
        Self(Arc::new(Mutex::new(MockSessionInner {
            bundles: Vec::new(),
            should_error,
            is_closed: false,
        })))
    }

    pub fn assert_bundles_eq(&self, expected: Vec<String>) {
        let actual: Vec<String> = self
            .0
            .lock()
            .unwrap()
            .bundles
            .iter()
            .map(|b| std::str::from_utf8(b).expect("non-utf8 bundle").to_string())
            .collect();
        assert_eq!(actual, expected);
    }

    pub fn is_closed(&self) -> bool {
        self.0.lock().unwrap().is_closed
    }
}

impl Session for MockSession {
    fn send_data(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        let mut lock = self.0.lock().unwrap();
        if lock.is_closed {
            panic!("sent bundle after MockSession closed");
        }
        lock.bundles.push(data.to_vec());
        if lock.should_error {
            Err("MockSession error".into())
        } else {
            Ok(())
        }
    }

    fn max_packet_len(&self) -> usize {
        usize::MAX
    }

    fn close(&mut self) {
        let mut lock = self.0.lock().unwrap();
        if lock.is_closed {
            panic!("MockSession closed multiple times");
        }
        lock.is_closed = true;
    }
}
