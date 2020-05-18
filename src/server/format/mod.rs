mod decoder;
mod encoder;
mod json;

pub use decoder::Decoder;
pub use encoder::Encoder;

pub fn json_protocol_impls() -> (Box<dyn Encoder>, Box<dyn Decoder>) {
    (
        Box::new(json::JsonEncoder::new()),
        Box::new(json::JsonDecoder::new()),
    )
}

use super::*;
