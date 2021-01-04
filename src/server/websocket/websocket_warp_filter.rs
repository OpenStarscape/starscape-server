use super::*;

/// Returns a warp::Filter that, when added to a Warp HTTP server, initiates WebSocket connections
pub fn websocket_warp_filter(new_session_tx: Sender<Box<dyn SessionBuilder>>) -> GenericFilter {
    // Everything captured by the warp filter needs to be clonable and sync
    let new_session_tx = Arc::new(Mutex::new(new_session_tx));
    warp::path("websocket")
        // The `ws()` filter will prepare the Websocket handshake.
        .and(warp::ws())
        .map(move |ws: warp::ws::Ws| {
            let new_session_tx = new_session_tx.clone();
            // And then our closure will be called when it completes.
            Box::new(ws.on_upgrade(move |websocket| {
                if let Err(e) = new_session_tx
                    .lock()
                    .unwrap()
                    .send(Box::new(WebsocketSessionBuilder::new(websocket)))
                {
                    warn!("creating WebSocket session: {}", e);
                }
                futures::future::ready(())
            })) as Box<dyn warp::Reply>
        })
        .boxed()
}
