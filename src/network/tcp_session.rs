use mio::net::TcpStream;
use std::{
    error::Error,
    fmt::{Debug, Formatter},
    io::{ErrorKind::WouldBlock, Read, Write},
};

use super::*;

fn try_to_read_data(
    stream: &mut TcpStream,
    handle_incoming_data: &mut dyn FnMut(&[u8]) -> (),
) -> Result<(), Box<dyn Error>> {
    let mut buffer = [0 as u8; 1024];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                // Successful read of zero bytes means connection is closed
                return Err("Connection closed!".into());
            }
            Ok(len) => {
                handle_incoming_data(&buffer[0..len]);
                // Keep looping until we get a WouldBlock or other error…
            }
            Err(ref e) if e.kind() == WouldBlock => return Ok(()),
            Err(e) => return Err(e.into()),
        }
    }
}

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
        mut handle_incoming_data: Box<dyn FnMut(&[u8]) -> () + Send>,
    ) -> Result<Box<dyn Session>, Box<dyn Error>> {
        let thread = new_mio_poll_thread(self.stream.try_clone()?, move |listener| {
            try_to_read_data(listener, &mut *handle_incoming_data)
        })?;
        Ok(Box::new(TcpSession {
            stream: self.stream,
            _mio_poll_thread: thread,
        }))
    }
}

struct TcpSession {
    stream: TcpStream,
    _mio_poll_thread: Box<dyn Drop + Send>,
}

impl Debug for TcpSession {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "TcpSession connected to {:?}", self.stream.peer_addr())
    }
}

impl Session for TcpSession {
    fn send(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        self.stream.write_all(data)?;
        Ok(())
    }
}
