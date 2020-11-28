use super::*;

mod json_decoder;
mod json_encoder;

pub use json_decoder::JsonDecoder;
pub use json_encoder::JsonEncoder;

pub fn json_protocol_impls() -> (Box<dyn Encoder>, Box<dyn Decoder>) {
    (Box::new(JsonEncoder::new()), Box::new(JsonDecoder::new()))
}
