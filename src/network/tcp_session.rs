use mio::net::TcpStream;
use std::{
    error::Error,
    fmt::{Debug, Formatter},
    io::{ErrorKind::WouldBlock, Read},
    net::SocketAddr,
};

use super::*;

fn try_to_read_data(stream: &mut TcpStream) -> Result<(), Box<dyn Error>> {
    let mut buffer = [0 as u8; 1024];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                // Successful read of zero bytes means connection is closed
                return Err("Connection closed!".into());
            }
            Ok(len) => {
                let text = std::str::from_utf8(&buffer[0..len])?;
                println!("Read bytes from TCP stream: {}", text);
                // Keep looping until we get a WouldBlock or other error…
            }
            Err(ref e) if e.kind() == WouldBlock => return Ok(()),
            Err(e) => return Err(e.into()),
        }
    }
}

pub struct TcpSession {
    peer_addr: SocketAddr,
    _mio_poll_thread: Box<dyn Drop + Send>,
}

impl TcpSession {
    pub fn new(stream: TcpStream, peer_addr: SocketAddr) -> Result<Self, Box<dyn Error>> {
        let thread = new_mio_poll_thread(stream, |listener| try_to_read_data(listener))?;
        Ok(Self {
            peer_addr,
            _mio_poll_thread: thread,
        })
    }
}

impl Debug for TcpSession {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "TcpSession connected to {:?}", self.peer_addr)
    }
}

impl Session for TcpSession {}
