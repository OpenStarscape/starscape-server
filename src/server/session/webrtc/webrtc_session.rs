use super::*;

#[derive(Debug)]
pub struct WebrtcSessionBuilder {}

impl WebrtcSessionBuilder {
    pub fn new() -> Self {
        Self {}
    }
}

impl SessionBuilder for WebrtcSessionBuilder {
    fn build(
        self: Box<Self>,
        mut handle_incoming_data: Box<dyn FnMut(&[u8]) + Send>,
    ) -> Result<Box<dyn Session>, Box<dyn Error>> {
        Err("WebrtcSessionBuilder::build() not implemented".into())
    }
}

struct WebrtcSession {}

impl Debug for WebrtcSession {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "WebrtcSession") // Not fully implemented
    }
}

impl Session for WebrtcSession {
    fn send(&mut self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        Err("WebrtcSession::send() not implemented".into())
    }
}
