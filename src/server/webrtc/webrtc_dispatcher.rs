use super::*;

struct DispatcherInner {
    session_map: HashMap<SocketAddr, InboundBundleHandler>,
    new_session_tx: Sender<Box<dyn SessionBuilder>>,
    bundle_tx: Sender<(SocketAddr, Vec<u8>)>,
}

#[derive(Clone)]
pub struct WebrtcDispatcher(Arc<Mutex<DispatcherInner>>);

impl WebrtcDispatcher {
    pub fn new(
        new_session_tx: Sender<Box<dyn SessionBuilder>>,
        bundle_tx: Sender<(SocketAddr, Vec<u8>)>,
    ) -> Self {
        Self(Arc::new(Mutex::new(DispatcherInner {
            session_map: HashMap::new(),
            new_session_tx,
            bundle_tx,
        })))
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<DispatcherInner>, Box<dyn Error>> {
        self.0
            .lock()
            .map_err(|e| format!("failed to lock mutex: {}", e).into())
    }

    pub fn set_inbound_handler(
        &self,
        addr: SocketAddr,
        mut handler: InboundBundleHandler,
    ) -> Result<(), Box<dyn Error>> {
        let mut locked = self.lock()?;
        locked.session_map.insert(addr, handler);
        Ok(())
    }

    pub fn dispatch_inbound(&self, addr: SocketAddr, data: &[u8]) {
        match self.lock() {
            Ok(mut locked) => match locked.session_map.get_mut(&addr) {
                Some(handler) => handler(data),
                None => {
                    let pending = Arc::new(Mutex::new(vec![data.to_vec()]));
                    locked.session_map.insert(
                        addr,
                        Box::new(move |data: &[u8]| {
                            pending
                                .lock()
                                .expect("failed to lock pending data mutex")
                                .push(data.to_vec())
                        }),
                    );
                    let session_builder =
                        WebrtcSession::new(self.clone(), addr, locked.bundle_tx.clone());
                    if let Err(e) = locked.new_session_tx.send(Box::new(session_builder)) {
                        error!("failed to send WebRTC session builder: {}", e);
                    }
                }
            },
            Err(e) => error!("failed to lock WebRTC dispatcher: {}", e),
        }
    }
}
