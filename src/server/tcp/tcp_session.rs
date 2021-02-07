use super::*;
use ::mio::net::TcpStream;
use std::io::{ErrorKind::WouldBlock, Read, Write};

fn try_to_read_data(
    stream: &mut TcpStream,
    handler: &mut dyn InboundBundleHandler,
) -> Result<(), Box<dyn Error>> {
    let mut buffer = [0; 1024];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                // Successful read of zero bytes means connection is closed
                handler.close();
            }
            Ok(len) => {
                handler.handle(&buffer[0..len]);
                // Keep looping until we get a WouldBlock or other error…
            }
            Err(ref e) if e.kind() == WouldBlock => return Ok(()),
            Err(e) => return Err(e.into()),
        }
    }
}

#[derive(Debug)]
pub struct TcpSessionBuilder {
    stream: TcpStream,
}

impl TcpSessionBuilder {
    pub fn new(stream: TcpStream) -> Self {
        Self { stream }
    }
}

impl SessionBuilder for TcpSessionBuilder {
    fn build(
        self: Box<Self>,
        handler: Box<dyn InboundBundleHandler>,
    ) -> Result<Box<dyn Session>, Box<dyn Error>> {
        let handler = Arc::new(Mutex::new(handler));
        let poll_thread_handler = handler.clone();
        let thread = new_mio_poll_thread(self.stream.try_clone()?, move |listener| {
            // This could probably be done without a lock every message but who cares
            let mut locked_handler = poll_thread_handler.lock().unwrap();
            try_to_read_data(listener, &mut **locked_handler)
        })?;
        Ok(Box::new(TcpSession {
            stream: self.stream,
            handler,
            mio_poll_thread: Some(thread),
        }))
    }
}

struct TcpSession {
    stream: TcpStream,
    /// Note that the mutex remains locked by the poll thread for as long as it's alive
    handler: Arc<Mutex<Box<dyn InboundBundleHandler>>>,
    mio_poll_thread: Option<Box<dyn Drop + Send>>,
}

impl Debug for TcpSession {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "TcpSession connected to {:?}", self.stream.peer_addr())
    }
}

impl Session for TcpSession {
    fn yeet_bundle(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        self.stream.write_all(data)?;
        Ok(())
    }

    fn max_packet_len(&self) -> usize {
        std::usize::MAX
    }

    fn close(&mut self) {
        self.mio_poll_thread = None;
        self.stream
            .shutdown(std::net::Shutdown::Both)
            .or_log_warn("shutting down TCP stream");
        match self.handler.lock() {
            Ok(mut handler) => handler.close(),
            Err(e) => error!("failed to close connection, could not lock handler: {}", e),
        }
    }
}
