use super::*;

/// Receives data from the session layer (on the session's thread), decodes it into requests and
/// sends those off to be processed by the session on the main thead.
pub struct BundleHandler {
    connection_key: ConnectionKey,
    decoder: Box<dyn Decoder>,
    decode_ctx: Arc<dyn DecodeCtx>,
    request_tx: Sender<Request>,
}

impl BundleHandler {
    pub fn new(
        connection_key: ConnectionKey,
        decoder: Box<dyn Decoder>,
        decode_ctx: Arc<dyn DecodeCtx>,
        request_tx: Sender<Request>,
    ) -> Self {
        Self {
            connection_key,
            decoder,
            decode_ctx,
            request_tx,
        }
    }
}

impl InboundBundleHandler for BundleHandler {
    fn handle(&mut self, data: &[u8]) {
        match self
            .decoder
            .decode(self.decode_ctx.as_ref(), data.to_owned())
        {
            Ok(requests) => {
                requests.into_iter().for_each(|request| {
                    if let Err(e) = self.request_tx.send(request) {
                        warn!("failed to handle data for {:?}: {}", self.connection_key, e);
                    }
                });
            }
            Err(e) => {
                warn!(
                    "can't decode inbound bundle: {} on {:?}",
                    e, self.connection_key
                );
            }
        }
    }

    fn close(&mut self) {
        if let Err(e) = self.request_tx.send(Request::Close) {
            warn!("failed to close {:?}: {}", self.connection_key, e);
        }
    }
}
