use super::*;
use std::net::{SocketAddr, TcpListener};

struct KnownSocket {
    addr: SocketAddr,
    is_locked: AtomicBool,
}

pub struct ReservedSocket(Arc<KnownSocket>);

impl Deref for ReservedSocket {
    type Target = SocketAddr;

    fn deref(&self) -> &Self::Target {
        &self.0.addr
    }
}

impl Drop for ReservedSocket {
    fn drop(&mut self) {
        self.0.is_locked.store(false, SeqCst);
    }
}

struct Sockets {
    known: Vec<Arc<KnownSocket>>,
    port: u16,
}

lazy_static::lazy_static! {
    static ref SOCKETS: Mutex<Sockets> = Mutex::new(Sockets{
        known: Vec::new(),
        port: 51111, // Anything over 49152 works
    });
}

/// Returns a socket address that was not in use when checked and will not be used by anything else in this process
/// until after the ReservedSocket is dropped. Manages a global pool of socket addresses for efficiency.
pub fn provision_socket() -> ReservedSocket {
    let mut sockets = SOCKETS.lock().unwrap();
    // First, search for an already known socket that is not currently locked
    for socket in &sockets.known {
        // This is safe because is_locked may go from true to false at any time, but only goes false to true while
        // SOCKETS is locked.
        if !socket.is_locked.load(SeqCst) {
            socket.is_locked.store(true, SeqCst);
            return ReservedSocket(socket.clone());
        }
    }
    // Failing that, find a new socket
    let ip = "::1".parse().unwrap();
    loop {
        sockets.port += 1;
        if sockets.port >= 65535 {
            panic!("provision_socket() could not find a free socket");
        }
        let addr = SocketAddr::new(ip, sockets.port);
        match TcpListener::bind(addr) {
            Ok(_) => {
                // Cool, this socket is free
                let socket = Arc::new(KnownSocket {
                    addr,
                    is_locked: AtomicBool::new(true),
                });
                sockets.known.push(socket.clone());
                return ReservedSocket(socket);
            }
            _ => (),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_provision_socket() {
        let _ = provision_socket();
    }

    #[test]
    fn multiple_calls_results_in_multiple_sockets() {
        let a = provision_socket();
        let b = provision_socket();
        assert_ne!(*a, *b);
    }

    #[test]
    fn does_not_provision_new_socket_if_not_needed() {
        for _ in 0..100 {
            let _ = provision_socket();
        }
        // This is a global pool, so this test is banking on the assumption that >50 sockets will never be needed at once
        assert!(SOCKETS.lock().unwrap().known.len() < 50);
    }
}
