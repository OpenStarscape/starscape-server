use super::*;
use std::io::ErrorKind::WouldBlock;

fn try_to_accept_connections(
    listener: &::mio::net::TcpListener,
    new_session_tx: &Sender<Box<dyn SessionBuilder>>,
) -> Result<(), Box<dyn Error>> {
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                let session = TcpSessionBuilder::new(stream);
                if let Err(e) = new_session_tx.send(Box::new(session)) {
                    error!("failed to send TCP session: {}", e);
                }
                // Keep looping until we get a WouldBlock or other error…
            }
            Err(ref e) if e.kind() == WouldBlock => return Ok(()),
            Err(e) => return Err(e.into()),
        }
    }
}

pub struct TcpListener {
    address: SocketAddr,
    _mio_poll_thread: Box<dyn Drop>,
}

impl TcpListener {
    pub fn new(
        new_session_tx: Sender<Box<dyn SessionBuilder>>,
        addr: SocketAddr,
    ) -> Result<Self, Box<dyn Error>> {
        let listener = ::mio::net::TcpListener::bind(&addr)?;
        let thread = new_mio_poll_thread(listener, move |listener| {
            try_to_accept_connections(listener, &new_session_tx)
        })?;
        Ok(Self {
            address: addr,
            _mio_poll_thread: thread,
        })
    }
}

impl Debug for TcpListener {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "TcpListener on {:?}", self.address)
    }
}

impl ServerComponent for TcpListener {}

#[cfg(test)]
mod tests {
    use super::*;
    use ::mio::net::TcpStream;
    use std::{
        io::{Read, Write},
        thread,
        time::Duration,
    };

    const SHORT_TIME: Duration = Duration::from_millis(20);

    fn build(tx: Sender<Box<dyn SessionBuilder>>) -> (ReservedSocket, TcpListener) {
        let socket = provision_socket();
        match TcpListener::new(tx.clone(), *socket) {
            Ok(listener) => (socket, listener),
            Err(e) => panic!("failed to create TcpListener: {}", e),
        }
    }

    #[test]
    fn can_start_and_stop_immediately() {
        run_with_timeout(|| {
            let (tx, _rx) = channel();
            let (_socket, _listener) = build(tx);
        });
    }

    #[test]
    fn can_start_and_stop_with_pause() {
        let (tx, _rx) = channel();
        run_with_timeout(move || {
            let (_socket, _listener) = build(tx);
            thread::sleep(SHORT_TIME);
        });
    }

    #[test]
    fn does_not_create_session_by_default() {
        let (tx, rx) = channel();
        run_with_timeout(|| {
            let (_socket, _listener) = build(tx);
            thread::sleep(SHORT_TIME);
        });
        let sessions: Vec<Box<dyn SessionBuilder>> = rx.try_iter().collect();
        assert_eq!(sessions.len(), 0);
    }

    #[test]
    fn ceates_session_on_connection() {
        let (tx, rx) = channel();
        run_with_timeout(|| {
            let (_socket, listener) = build(tx);
            let _client = TcpStream::connect(&listener.address).expect("failed to connect");
            thread::sleep(SHORT_TIME);
        });
        let sessions: Vec<Box<dyn SessionBuilder>> = rx.try_iter().collect();
        assert_eq!(sessions.len(), 1);
    }

    #[test]
    fn can_create_multiple_sessions() {
        let (tx, rx) = channel();
        run_with_timeout(|| {
            let (socket, _listener) = build(tx);
            let _client_a = TcpStream::connect(&*socket).expect("failed to connect");
            let _client_b = TcpStream::connect(&*socket).expect("failed to connect");
            let _client_c = TcpStream::connect(&*socket).expect("failed to connect");
            thread::sleep(SHORT_TIME);
            let _client_d = TcpStream::connect(&*socket).expect("failed to connect");
            thread::sleep(SHORT_TIME);
        });
        let sessions: Vec<Box<dyn SessionBuilder>> = rx.try_iter().collect();
        assert_eq!(sessions.len(), 4);
    }

    #[test]
    fn can_build_session() {
        run_with_timeout(|| {
            let (tx, rx) = channel();
            let (socket, _listener) = build(tx);
            let _client = TcpStream::connect(&*socket).expect("failed to connect");
            thread::sleep(SHORT_TIME);
            let builder = rx.try_recv().unwrap();
            let handler = MockInboundHandler::new();
            let _session = builder.build(Box::new(handler)).unwrap();
        });
    }

    #[test]
    fn can_send_data_client_to_server() {
        run_with_timeout(|| {
            let (tx, rx) = channel();
            let (socket, _listener) = build(tx);
            let mut client = TcpStream::connect(&*socket).expect("failed to connect");
            thread::sleep(SHORT_TIME);
            let builder = rx.try_recv().unwrap();
            let handler = MockInboundHandler::new();
            let _session = builder.build(Box::new(handler.clone())).unwrap();
            client.write_all(&[75]).unwrap();
            thread::sleep(SHORT_TIME);
            assert_eq!(handler.get(), vec![MockInbound::Data(vec![75])]);
        });
    }

    #[test]
    fn can_send_data_server_to_client() {
        run_with_timeout(|| {
            let (tx, rx) = channel();
            let (socket, _listener) = build(tx);
            let mut client = TcpStream::connect(&*socket).expect("failed to connect");
            thread::sleep(SHORT_TIME);
            let builder = rx.try_recv().unwrap();
            let handler = MockInboundHandler::new();
            let mut session = builder.build(Box::new(handler.clone())).unwrap();
            session.yeet_bundle(&[82]).unwrap();
            thread::sleep(SHORT_TIME);
            let mut buffer = [0; 1];
            client.read_exact(&mut buffer).unwrap();
            assert_eq!(buffer, [82]);
        });
    }

    #[test]
    fn can_shut_down_while_client_still_active() {
        run_with_timeout(move || {
            let _client;
            {
                let (tx, rx) = channel();
                let (socket, _listener) = build(tx);
                _client = TcpStream::connect(&*socket).expect("failed to connect");
                thread::sleep(SHORT_TIME);
                let builder = rx.try_recv().unwrap();
                let handler = MockInboundHandler::new();
                let _session = builder.build(Box::new(handler)).unwrap();
            }
        });
    }
}
