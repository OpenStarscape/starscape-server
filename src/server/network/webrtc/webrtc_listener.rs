use super::*;

pub struct WebrtcListener {}

impl WebrtcListener {
    pub fn new(new_session_tx: Sender<Box<dyn SessionBuilder>>) -> Result<Self, Box<dyn Error>> {
        Err("WebrtcListener::new() not implemented".into())
    }
}

impl Debug for WebrtcListener {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "WebrtcListener") // Not fully implemented
    }
}

impl Listener for WebrtcListener {}
