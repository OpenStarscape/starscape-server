use mio::net::{TcpListener, TcpStream};
use mio::{Events, Poll, PollOpt, Ready, Registration, SetReadiness, Token};
use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread::{spawn, JoinHandle};

use super::*;

fn handle_connection(stream: TcpStream) -> Result<(), Box<dyn Error>> {
    let _mio_poll_thread = new_mio_poll_thread(stream, |_| Ok(()))?;
    Ok(())
}

fn try_to_accept_connection(listener: &TcpListener) -> Result<(), Box<dyn Error>> {
    match listener.accept() {
        Ok((stream, _peer_addr)) => handle_connection(stream),
        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(()),
        Err(e) => Err(e.into()),
    }
}

pub struct TcpServer {
    mio_poll_thread: Box<dyn Drop>,
}

impl TcpServer {
    pub fn new(new_session_tx: Sender<Box<dyn Session>>) -> Result<Self, Box<dyn Error>> {
        let addr: std::net::SocketAddr = "127.0.0.1:1212".parse()?;
        let listener = TcpListener::bind(&addr)?;
        let mio_poll_thread = new_mio_poll_thread(listener, try_to_accept_connection)?;
        Ok(Self { mio_poll_thread })
    }
}

impl Server for TcpServer {}

#[cfg(test)]
mod tests {
    use super::*;
	use crate::util::run_with_timeout;
    use std::sync::mpsc::channel;
    use std::{thread, time::Duration};

    const SHORT_TIME: Duration = Duration::from_millis(20);

    #[test]
    fn can_start_and_stop_immediately() {
        run_with_timeout(|| {
            let (tx, _rx) = channel();
            let _server = TcpServer::new(tx).expect("failed to create TCP server");
        });
    }

    #[test]
    fn can_start_and_stop_with_pause() {
        let (tx, _rx) = channel();
        run_with_timeout(move || {
            let _server = TcpServer::new(tx).expect("failed to create TCP server");
            thread::sleep(SHORT_TIME);
        });
    }

    /*
    #[test]
    fn does_not_create_session_by_default() {
        run_with_timeout(LONG_TIME, || {
            let (tx, rx) = channel();
            let _server = TcpServer::new(tx).expect("failed to create TCP server");
            thread::sleep(SHORT_TIME);
            let sessions: Vec<Box<dyn Session>> = rx.try_iter().collect();
            assert_eq!(sessions.len(), 0);
        });
    }
    */
}
