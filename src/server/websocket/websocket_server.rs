use super::*;

pub struct WebsocketServer {}

impl WebsocketServer {
    pub fn new(
        new_session_tx: Sender<Box<dyn SessionBuilder>>,
    ) -> Result<(GenericFilter, Self), Box<dyn Error>> {
        Ok((websocket_warp_filter(new_session_tx), Self {}))
    }

    // TODO: keep track of connections and gracefully close all on shutdown
}

impl Debug for WebsocketServer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "WebsocketServer")
    }
}

impl ServerComponent for WebsocketServer {}
