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
    use std::sync::mpsc::channel;
    use std::{sync::mpsc, thread, time::Duration};

    const LONG_TIME: Duration = Duration::from_secs(1);
    const SHORT_TIME: Duration = Duration::from_millis(20);

    /// stolen from https://github.com/rust-lang/rfcs/issues/2798#issuecomment-552949300
    fn panic_after<T, F>(d: Duration, f: F) -> T
    where
        T: Send + 'static,
        F: FnOnce() -> T,
        F: Send + 'static,
    {
        let (done_tx, done_rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            let val = f();
            done_tx.send(()).expect("unable to send completion signal");
            val
        });

        match done_rx.recv_timeout(d) {
            Err(mpsc::RecvTimeoutError::Timeout) => panic!("thread timed out"),
            _ => match handle.join() {
                Ok(result) => result,
                Err(any) => panic!("thread panicked: {:?}", any),
            },
        }
    }

    #[test]
    fn can_start_and_stop_immediately() {
        panic_after(LONG_TIME, || {
            let (tx, _rx) = channel();
            let _server = TcpServer::new(tx).expect("failed to create TCP server");
        });
    }

    #[test]
    fn can_start_and_stop_with_pause() {
        let (tx, _rx) = channel();
        panic_after(LONG_TIME, move || {
            let _server = TcpServer::new(tx).expect("failed to create TCP server");
            thread::sleep(SHORT_TIME);
        });
    }

    #[test]
    fn does_not_create_session_by_default() {
        panic_after(LONG_TIME, || {
            let (tx, rx) = channel();
            let _server = TcpServer::new(tx).expect("failed to create TCP server");
            thread::sleep(SHORT_TIME);
            let sessions: Vec<Box<dyn Session>> = rx.try_iter().collect();
            assert_eq!(sessions.len(), 0);
        });
    }
}
