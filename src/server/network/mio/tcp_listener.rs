use super::*;
use std::{
    io::ErrorKind::{AddrInUse, WouldBlock},
    net::{IpAddr, SocketAddr},
    sync::mpsc::Sender,
};

fn try_to_accept_connections(
    listener: &::mio::net::TcpListener,
    new_session_tx: &Sender<Box<dyn SessionBuilder>>,
) -> Result<(), Box<dyn Error>> {
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                let session = TcpSessionBuilder::new(stream);
                if let Err(e) = new_session_tx.send(Box::new(session)) {
                    eprintln!("Failed to send TCP session: {}", e);
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
        requested_addr: Option<IpAddr>,
        requested_port: Option<u16>,
    ) -> Result<Self, Box<dyn Error>> {
        let addr = requested_addr.unwrap_or("::1".parse()?);
        for i in 0..20 {
            let port = requested_port.unwrap_or(55_000 + i * 10);
            let socket_addr = SocketAddr::new(addr, port);
            match ::mio::net::TcpListener::bind(&socket_addr) {
                Ok(listener) => {
                    let thread = new_mio_poll_thread(listener, move |listener| {
                        try_to_accept_connections(listener, &new_session_tx)
                    })?;
                    eprintln!("Created TCP listener on {}", socket_addr);
                    return Ok(Self {
                        address: socket_addr,
                        _mio_poll_thread: thread,
                    });
                }
                Err(e) if e.kind() == AddrInUse => {
                    eprintln!("{} in use", socket_addr);
                }
                Err(e) => return Err(e.into()),
            }
        }
        match requested_port {
            Some(port) => Err(format!("address/port {}:{} not available", addr, port).into()),
            None => Err("could not find available port".into()),
        }
    }
}

impl Debug for TcpListener {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "TcpListener on {:?}", self.address)
    }
}

impl Listener for TcpListener {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::run_with_timeout;
    use ::mio::net::TcpStream;
    use std::{net::Ipv6Addr, sync::mpsc::channel, thread, time::Duration};

    const SHORT_TIME: Duration = Duration::from_millis(20);
    const LOOPBACK: Option<IpAddr> = Some(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)));

    fn build(tx: Sender<Box<dyn SessionBuilder>>) -> TcpListener {
        TcpListener::new(tx, LOOPBACK, None).expect("failed to create TCP listener")
    }

    #[test]
    fn can_start_and_stop_immediately() {
        run_with_timeout(|| {
            let (tx, _rx) = channel();
            let _listener = build(tx);
        });
    }

    #[test]
    fn can_start_and_stop_with_pause() {
        let (tx, _rx) = channel();
        run_with_timeout(move || {
            let _listener = build(tx);
            thread::sleep(SHORT_TIME);
        });
    }

    #[test]
    fn does_not_create_session_by_default() {
        let (tx, rx) = channel();
        run_with_timeout(|| {
            let _listener = build(tx);
            thread::sleep(SHORT_TIME);
        });
        let sessions: Vec<Box<dyn SessionBuilder>> = rx.try_iter().collect();
        assert_eq!(sessions.len(), 0);
    }

    #[test]
    fn ceates_session_on_connection() {
        let (tx, rx) = channel();
        run_with_timeout(|| {
            let listener = build(tx);
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
            let listener = build(tx);
            let _client_a = TcpStream::connect(&listener.address).expect("failed to connect");
            let _client_b = TcpStream::connect(&listener.address).expect("failed to connect");
            let _client_c = TcpStream::connect(&listener.address).expect("failed to connect");
            thread::sleep(SHORT_TIME);
            let _client_d = TcpStream::connect(&listener.address).expect("failed to connect");
            thread::sleep(SHORT_TIME);
        });
        let sessions: Vec<Box<dyn SessionBuilder>> = rx.try_iter().collect();
        assert_eq!(sessions.len(), 4);
    }
}
