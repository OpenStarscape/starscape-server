use mio::net::{TcpListener, TcpStream};
use mio::{Events, Poll, PollOpt, Ready, Registration, SetReadiness, Token};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{spawn, JoinHandle};

use super::*;

fn handle_connection(stream: TcpStream) {
    panic!("Tried to initiate connection");
}

fn run(quit_registration: Registration, should_quit: Arc<AtomicBool>) {
    const TOKEN: Token = Token(0);
    let addr: std::net::SocketAddr = "127.0.0.1:1212".parse().expect("Failed to parse addr");
    let listener = TcpListener::bind(&addr).expect("Failed to bind to socket");
    let poll = Poll::new().expect("Failed to create Poll");
    poll.register(&listener, TOKEN, Ready::readable(), PollOpt::edge())
        .unwrap();
    poll.register(
        &quit_registration,
        TOKEN,
        Ready::readable(),
        PollOpt::edge(),
    )
    .unwrap();
    let mut events = Events::with_capacity(1024);
    loop {
        poll.poll(&mut events, None).unwrap();
        if should_quit.load(Ordering::Relaxed) {
            break;
        }
        for event in events.iter() {
            match event.token() {
                TOKEN => match listener.accept() {
                    Ok((stream, _peer_addr)) => handle_connection(stream),
                    Err(e) => {
                        if e.kind() != std::io::ErrorKind::WouldBlock {
                            panic!("error connecting: {}", e);
                        }
                    }
                },
                token => panic!("unknown token {:?}", token),
            }
        }
    }
}

pub struct TcpServer {
    should_quit: Arc<AtomicBool>,
    quit_set_readiness: SetReadiness,
    thread: Option<JoinHandle<()>>,
}

impl TcpServer {
    pub fn new() -> Self {
        let (quit_registration, quit_set_readiness) = Registration::new2();
        let should_quit = Arc::new(AtomicBool::new(false));
        let thread = Some(spawn({
            let should_quit = should_quit.clone();
            || run(quit_registration, should_quit)
        }));
        Self {
            should_quit,
            quit_set_readiness,
            thread,
        }
    }
}

impl Drop for TcpServer {
    fn drop(&mut self) {
        self.should_quit.store(true, Ordering::Relaxed);
        self.quit_set_readiness.set_readiness(Ready::readable()).expect("failed to set readiness");
        self.thread.take().unwrap().join().expect("server panicked");
    }
}

impl Server for TcpServer {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::mpsc, thread, time::Duration};

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
            done_tx.send(()).expect("Unable to send completion signal");
            val
        });

        match done_rx.recv_timeout(d) {
            Ok(_) => handle.join().expect("Thread panicked"),
            Err(_) => panic!("Thread took too long"),
        }
    }

    #[test]
    fn can_start_and_stop() {
        panic_after(Duration::from_secs(1), || {
            let _server = TcpServer::new();
        });
    }
}
