use mio::net::{TcpListener, TcpStream};
use std::{
    error::Error,
    io::ErrorKind::AddrInUse,
    net::{IpAddr, SocketAddr},
    sync::mpsc::Sender,
};

use super::*;

fn handle_connection(stream: TcpStream) -> Result<(), Box<dyn Error>> {
    let _mio_poll_thread = new_mio_poll_thread(stream, |_| Ok(()))?;
    Ok(())
}

fn try_to_accept_connection(listener: &mut TcpListener) -> Result<(), Box<dyn Error>> {
    match listener.accept() {
        Ok((stream, _peer_addr)) => handle_connection(stream),
        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(()),
        Err(e) => Err(e.into()),
    }
}

pub struct TcpServer {
    address: SocketAddr,
    mio_poll_thread: Box<dyn Drop>,
}

impl TcpServer {
    pub fn new(
        new_session_tx: Sender<Box<dyn Session>>,
        requested_addr: Option<IpAddr>,
        requested_port: Option<u16>,
    ) -> Result<Self, Box<dyn Error>> {
        let addr = requested_addr.unwrap_or("::1".parse()?);
        for i in 0..20 {
            let port = requested_port.unwrap_or(55_000 + i * 10);
            let socket_addr = SocketAddr::new(addr, port);
            match TcpListener::bind(&socket_addr) {
                Ok(listener) => {
                    let mio_poll_thread = new_mio_poll_thread(listener, try_to_accept_connection)?;
                    return Ok(Self {
                        address: socket_addr,
                        mio_poll_thread,
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

impl Server for TcpServer {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::run_with_timeout;
    use std::{net::Ipv6Addr, sync::mpsc::channel, thread, time::Duration};

    const SHORT_TIME: Duration = Duration::from_millis(20);
    const LOOPBACK: Option<IpAddr> = Some(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)));

    #[test]
    fn can_start_and_stop_immediately() {
        run_with_timeout(|| {
            let (tx, _rx) = channel();
            let _server = TcpServer::new(tx, LOOPBACK, None).expect("failed to create TCP server");
        });
    }

    #[test]
    fn can_start_and_stop_with_pause() {
        let (tx, _rx) = channel();
        run_with_timeout(move || {
            let _server = TcpServer::new(tx, LOOPBACK, None).expect("failed to create TCP server");
            thread::sleep(SHORT_TIME);
        });
    }

    #[test]
    fn does_not_create_session_by_default() {
        run_with_timeout(|| {
            let (tx, rx) = channel();
            let _server = TcpServer::new(tx, LOOPBACK, None).expect("failed to create TCP server");
            thread::sleep(SHORT_TIME);
            let sessions: Vec<Box<dyn Session>> = rx.try_iter().collect();
            assert_eq!(sessions.len(), 0);
        });
    }
}
