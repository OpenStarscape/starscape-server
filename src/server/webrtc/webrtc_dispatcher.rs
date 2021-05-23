use super::*;

/// Because session setup is basically just "when we start getting data we assume there's a session",
/// sessions start getting data before a connection has been set up for them. For this reason, we
/// need to buffer the received data and send it to the connection when it's ready.
enum DispatchTarget {
    /// This is used before the connection is set up
    Buffer(Vec<Vec<u8>>),
    /// This means too much data has been received and we gave up waiting for the connection. Value
    /// is the number of packets.
    OverflowedBuffer(usize),
    /// This is used after a real connection has been set up
    Handler(Box<dyn InboundBundleHandler>),
}

impl DispatchTarget {
    fn new() -> Self {
        Self::Buffer(Vec::new())
    }

    /// Handle data sent from a client
    fn dispatch(&mut self, data: &[u8]) {
        match self {
            Self::Buffer(bundle_list) => {
                bundle_list.push(data.to_vec());
                if bundle_list.len() > 20 {
                    // We've gotten too many packets with nothing to properly handle them, so we
                    // give up buffering to prevent excess memory usage. If someone does try to
                    // build a session later set_handler() will error.
                    *self = Self::OverflowedBuffer(bundle_list.len());
                }
            }
            Self::OverflowedBuffer(count) => *count += 1,
            Self::Handler(handler) => handler.handle(data),
        }
    }

    /// This *should* only be called once when the session is built and given a handler. It sends
    /// all buffered data to the handler before returning. Returns Err if called multiple times or
    /// the buffer overflowed
    fn set_handler(
        &mut self,
        mut handler: Box<dyn InboundBundleHandler>,
    ) -> Result<(), Box<dyn Error>> {
        match self {
            Self::Buffer(bundles) => {
                for bundle in bundles {
                    handler.handle(bundle);
                }
                *self = Self::Handler(handler);
                Ok(())
            }
            Self::OverflowedBuffer(count) => Err(format!(
                "received {} packets before handler was set, and gave up buffering them",
                count
            )
            .into()),
            Self::Handler(_) => Err("handler dispatch target set multiple times".into()),
        }
    }
}

impl Drop for DispatchTarget {
    fn drop(&mut self) {
        if let Self::Handler(handler) = self {
            handler.close();
        }
        // if we're a self::Buffer we will fail to build the connection if we try to (which is fine)
    }
}

struct DispatcherInner {
    session_map: HashMap<SocketAddr, DispatchTarget>,
    new_session_tx: Sender<Box<dyn SessionBuilder>>,
    outbound_tx: tokio::sync::mpsc::Sender<(SocketAddr, WebrtcMessage)>,
}

/// Dispatches inbound data to the correct session based on source address
#[derive(Clone)]
pub struct WebrtcDispatcher(Arc<Mutex<DispatcherInner>>);

#[derive(Debug, PartialEq)]
pub enum WebrtcMessage {
    Data(Vec<u8>),
    Close,
}

impl WebrtcDispatcher {
    pub fn new(
        new_session_tx: Sender<Box<dyn SessionBuilder>>,
        outbound_tx: tokio::sync::mpsc::Sender<(SocketAddr, WebrtcMessage)>,
    ) -> Self {
        Self(Arc::new(Mutex::new(DispatcherInner {
            session_map: HashMap::new(),
            new_session_tx,
            outbound_tx,
        })))
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<DispatcherInner>, Box<dyn Error>> {
        self.0
            .lock()
            .map_err(|e| format!("failed to lock mutex: {}", e).into())
    }

    pub fn set_inbound_handler(
        &self,
        addr: &SocketAddr,
        handler: Box<dyn InboundBundleHandler>,
    ) -> Result<(), Box<dyn Error>> {
        let mut locked = self.lock()?;
        locked
            .session_map
            .get_mut(addr)
            .ok_or(format!("can not set handler for unknown address {}", addr))?
            .set_handler(handler)
    }

    pub fn dispatch_inbound(&self, addr: &SocketAddr, data: &[u8]) {
        match self.lock() {
            Ok(mut locked) => match locked.session_map.get_mut(addr) {
                Some(target) => target.dispatch(data),
                None => {
                    let mut target = DispatchTarget::new();
                    target.dispatch(data);
                    locked.session_map.insert(*addr, target);
                    let session =
                        WebrtcSession::new(self.clone(), *addr, locked.outbound_tx.clone());
                    if let Err(e) = locked.new_session_tx.send(Box::new(session)) {
                        error!("failed to send WebRTC session builder: {}", e);
                    }
                }
            },
            Err(e) => error!("failed to lock WebRTC dispatcher: {}", e),
        }
    }

    pub fn close_session(&self, addr: &SocketAddr) {
        match self.lock() {
            Ok(mut locked) => match locked.session_map.remove(addr) {
                Some(_) => (),
                None => error!("failed to close unknown WebRTC session {}", addr),
            },
            Err(e) => error!("failed to lock WebRTC dispatcher: {}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_addr(port: u16) -> SocketAddr {
        SocketAddr::new("::1".parse().unwrap(), port)
    }

    fn test_data(value: u8) -> Vec<u8> {
        vec![value, value, value]
    }

    #[allow(clippy::type_complexity)]
    fn new_test() -> (
        Receiver<Box<dyn SessionBuilder>>,
        tokio::sync::mpsc::Receiver<(SocketAddr, WebrtcMessage)>,
        WebrtcDispatcher,
    ) {
        let (new_session_tx, new_session_rx) = channel();
        let (outbound_tx, outbound_rx) = tokio::sync::mpsc::channel(10);
        let dispatcher = WebrtcDispatcher::new(new_session_tx, outbound_tx);
        (new_session_rx, outbound_rx, dispatcher)
    }

    #[test]
    fn creates_session_on_new_address() {
        let (new_session, _, dispatcher) = new_test();
        dispatcher.dispatch_inbound(&test_addr(1), &test_data(1));
        let builder = new_session
            .recv_timeout(Duration::from_secs(1))
            .expect("no session builder");
        builder
            .build(Box::new(MockInboundHandler::new()))
            .expect("failed to build session");
    }

    #[test]
    fn creates_multiple_sessions_for_multiple_addresses() {
        let (new_session, _, dispatcher) = new_test();
        dispatcher.dispatch_inbound(&test_addr(1), &test_data(1));
        dispatcher.dispatch_inbound(&test_addr(2), &test_data(2));
        let builder = new_session
            .recv_timeout(Duration::from_secs(1))
            .expect("no session builder");
        let _ = new_session
            .recv_timeout(Duration::from_secs(1))
            .expect("no session builder");
        builder
            .build(Box::new(MockInboundHandler::new()))
            .expect("failed to build session");
        dispatcher.dispatch_inbound(&test_addr(3), &test_data(3));
        let _ = new_session
            .recv_timeout(Duration::from_secs(1))
            .expect("no session builder");
    }

    #[test]
    fn dispatches_initial_packet_given_before_session_created() {
        let (new_session, _, dispatcher) = new_test();
        dispatcher.dispatch_inbound(&test_addr(1), &test_data(1));
        let builder = new_session
            .recv_timeout(Duration::from_secs(1))
            .expect("no session builder");
        let inbound = MockInboundHandler::new();
        let _ = builder.build(Box::new(inbound.clone()));
        assert_eq!(inbound.get(), vec![MockInbound::Data(test_data(1))]);
    }

    #[test]
    fn dispatches_multiple_packets_before_session_created() {
        let (new_session, _, dispatcher) = new_test();
        dispatcher.dispatch_inbound(&test_addr(1), &test_data(1));
        dispatcher.dispatch_inbound(&test_addr(1), &test_data(2));
        dispatcher.dispatch_inbound(&test_addr(1), &test_data(3));
        let builder = new_session
            .recv_timeout(Duration::from_secs(1))
            .expect("no session builder");
        let inbound = MockInboundHandler::new();
        let _ = builder.build(Box::new(inbound.clone()));
        assert_eq!(
            inbound.get(),
            vec![
                MockInbound::Data(test_data(1)),
                MockInbound::Data(test_data(2)),
                MockInbound::Data(test_data(3))
            ]
        );
    }

    #[test]
    fn fails_if_too_many_packets_sent_before_session_created() {
        let (new_session, _, dispatcher) = new_test();
        let count = 105;
        for i in 0..count {
            dispatcher.dispatch_inbound(&test_addr(1), &test_data(i));
        }
        let builder = new_session
            .recv_timeout(Duration::from_secs(1))
            .expect("no session builder");
        match builder.build(Box::new(MockInboundHandler::new())) {
            Ok(_) => panic!("was able to build session despite too many bundles being sent before"),
            Err(e) => {
                // doesn't really matter, but error should contain the number of packets
                if !format!("{}", e).contains(&format!("{}", count)) {
                    panic!("{:?} does not contain {}", e, count);
                }
            }
        }
    }

    #[test]
    fn dispatches_data_after_session_created() {
        let (new_session, _, dispatcher) = new_test();
        dispatcher.dispatch_inbound(&test_addr(1), &test_data(1));
        let builder = new_session
            .recv_timeout(Duration::from_secs(1))
            .expect("no session builder");
        let inbound = MockInboundHandler::new();
        let _ = builder.build(Box::new(inbound.clone()));
        dispatcher.dispatch_inbound(&test_addr(1), &test_data(2));
        dispatcher.dispatch_inbound(&test_addr(1), &test_data(3));
        assert_eq!(
            inbound.get(),
            vec![
                MockInbound::Data(test_data(1)),
                MockInbound::Data(test_data(2)),
                MockInbound::Data(test_data(3))
            ]
        );
    }

    #[test]
    fn created_session_can_send_bundle() {
        let (new_session, mut outbound_rx, dispatcher) = new_test();
        dispatcher.dispatch_inbound(&test_addr(1), &test_data(1));
        let builder = new_session
            .recv_timeout(Duration::from_secs(1))
            .expect("no session builder");
        let mut session = builder
            .build(Box::new(MockInboundHandler::new()))
            .expect("failed to build session");
        session
            .yeet_bundle(&test_data(2))
            .expect("failed to yeet bundle");
        let (addr, bundle) = run_with_timeout(move || block_on(outbound_rx.recv()))
            .expect("failed to receive bundle");
        assert_eq!(addr, test_addr(1));
        assert_eq!(bundle, WebrtcMessage::Data(test_data(2)));
    }

    #[test]
    fn can_be_closed() {
        let (new_session, _, dispatcher) = new_test();
        dispatcher.dispatch_inbound(&test_addr(1), &test_data(1));
        let builder = new_session
            .recv_timeout(Duration::from_secs(1))
            .expect("no session builder");
        let inbound = MockInboundHandler::new();
        let _ = builder.build(Box::new(inbound.clone()));
        dispatcher.close_session(&test_addr(1));
        assert_eq!(
            inbound.get(),
            vec![MockInbound::Data(test_data(1)), MockInbound::Close]
        );
    }
}
