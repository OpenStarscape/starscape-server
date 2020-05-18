use serde::de::Deserialize;
use serde_json::Value;
use std::{
    convert::{TryFrom, TryInto},
    error::Error,
};

use super::*;

impl TryFrom<serde_json::Value> for Decodable {
    type Error = &'static str;
    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        match value {
            Value::Null => Ok(Decodable::Null),
            Value::Bool(_) => Err("decoding bool not implemented"),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(Decodable::Integer(i))
                } else if let Some(f) = n.as_f64() {
                    Ok(Decodable::Scaler(f))
                } else {
                    Err("bad number")
                }
            }
            Value::String(_) => Err("decoding string not implemented"),
            Value::Array(array) => {
                let result: Result<Vec<Decodable>, Self::Error> =
                    array.into_iter().map(TryFrom::try_from).collect();
                Ok(Decodable::List(result?))
            }
            Value::Object(_) => Err("decoding map not implemented"),
        }
    }
}

pub struct JsonDecoder {
    splitter: DatagramSplitter,
}

impl JsonDecoder {
    pub fn new() -> Self {
        Self {
            splitter: DatagramSplitter::new(b'\n'),
        }
    }

    fn decode_obj_prop(
        datagram: &serde_json::map::Map<String, Value>,
    ) -> Result<ObjectProperty, Box<dyn Error>> {
        let obj = datagram
            .get("object")
            .ok_or("request does not have an object ID")?
            .as_u64()
            .ok_or("object ID not an unsigned int")?;
        let prop = datagram
            .get("property")
            .ok_or("request does not have a property")?
            .as_str()
            .ok_or("property not a string")?
            .to_owned();
        Ok((obj, prop))
    }

    fn decode_datagram(&self, bytes: &[u8]) -> Result<RequestData, Box<dyn Error>> {
        // serde doesn't handle internally tagged enums terribly well
        // (https://github.com/serde-rs/serde/issues/1495)
        // and this is unlikely to be a bottleneck so easier to just deserialize into a Value
        // rather than implementing complicated visitor shit
        let mut deserializer = serde_json::Deserializer::from_slice(bytes);
        let value = Value::deserialize(&mut deserializer)?;
        let datagram = value.as_object().ok_or("request is not a JSON object")?;
        let mtype = datagram
            .get("mtype")
            .ok_or("request does not have an mtype field")?
            .as_str()
            .ok_or("request type is not a string")?;
        Ok(match mtype {
            "set" => RequestData::Set(
                Self::decode_obj_prop(&datagram)?,
                datagram
                    .get("value")
                    .ok_or("set request does not have a value")?
                    .clone()
                    .try_into()?,
            ),
            "get" => RequestData::Get(Self::decode_obj_prop(&datagram)?),
            "subscribe" => RequestData::Subscribe(Self::decode_obj_prop(&datagram)?),
            "unsubscribe" => RequestData::Unsubscribe(Self::decode_obj_prop(&datagram)?),
            _ => return Err("request has invalid mtype".into()),
        })
    }
}

impl Decoder for JsonDecoder {
    fn decode(&mut self, bytes: Vec<u8>) -> Result<Vec<RequestData>, Box<dyn Error>> {
        let mut requests = Vec::new();
        for datagram in self.splitter.data(bytes) {
            requests.push(self.decode_datagram(&datagram)?);
        }
        Ok(requests)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use RequestData::*;

    fn assert_results_in_request(json: &str, request: RequestData) {
        let mut decoder = JsonDecoder::new();
        let result = decoder
            .decode(json.as_bytes().to_owned())
            .expect("failed to decode");
        assert_eq!(result, vec![request]);
    }

    fn assert_results_in_error(json: &str, msg: &str) {
        let mut decoder = JsonDecoder::new();
        match decoder.decode(json.as_bytes().to_owned()) {
            Ok(output) => panic!("should have errored, instead gave: {:?}", output),
            Err(e) if !format!("{}", e).contains(msg) => {
                panic!("{:?} does not contain {:?}", e, msg)
            }
            _ => (),
        }
    }

    #[test]
    fn basic_get_request() {
        assert_results_in_request(
            "{ \
                \"mtype\": \"get\", \
                \"object\": 42, \
                \"property\": \"foobar\" \
            }\n",
            Get((42, "foobar".to_owned())),
        );
    }

    #[test]
    fn basic_set_request() {
        assert_results_in_request(
            "{ \
                \"mtype\": \"set\", \
                \"object\": 17, \
                \"property\": \"xyz\", \
				\"value\": null \
            }\n",
            Set((17, "xyz".to_owned()), Decodable::Null),
        );
    }

    #[test]
    fn basic_subscribe_request() {
        assert_results_in_request(
            "{ \
                \"mtype\": \"subscribe\", \
                \"object\": 38, \
                \"property\": \"abc\" \
            }\n",
            Subscribe((38, "abc".to_owned())),
        );
    }

    #[test]
    fn basic_unsubscribe_request() {
        assert_results_in_request(
            "{ \
                \"mtype\": \"unsubscribe\", \
                \"object\": 38, \
                \"property\": \"abc\" \
            }\n",
            Unsubscribe((38, "abc".to_owned())),
        );
    }

    #[test]
    fn can_process_multiple_requests_split_up_cleanly() {
        let json = vec![
            "{ \
                \"mtype\": \"get\", \
                \"object\": 42, \
                \"property\": \"foobar\" \
            }\n",
            "{\
                \"mtype\": \"set\", \
                \"object\": 102, \
                \"property\": \"abc\", \
				\"value\": 12 \
            }\n",
            "{ \
				\"mtype\": \"subscribe\", \
                \"object\": 17, \
                \"property\": \"xyz\" \
			}\n",
        ];
        let mut decoder = JsonDecoder::new();
        let mut result = Vec::new();
        for json in json {
            result.extend(
                decoder
                    .decode(json.as_bytes().to_owned())
                    .expect("failed to decode"),
            );
        }
        assert_eq!(
            result,
            vec![
                Get((42, "foobar".to_owned())),
                Set((102, "abc".to_owned()), Decodable::Integer(12)),
                Subscribe((17, "xyz".to_owned()))
            ]
        );
    }

    #[test]
    fn can_process_multiple_requests_at_once() {
        let json = "{ \
                \"mtype\": \"get\", \
                \"object\": 42, \
                \"property\": \"foobar\" \
            }\n \
			{ \
                \"mtype\": \"set\", \
                \"object\": 102, \
                \"property\": \"abc\", \
				\"value\": 12 \
            }\n \
			{ \
				\"mtype\": \"subscribe\", \
                \"object\": 17, \
                \"property\": \"xyz\" \
			}\n";
        let mut decoder = JsonDecoder::new();
        let result = decoder
            .decode(json.as_bytes().to_owned())
            .expect("failed to decode");
        assert_eq!(
            result,
            vec![
                Get((42, "foobar".to_owned())),
                Set((102, "abc".to_owned()), Decodable::Integer(12)),
                Subscribe((17, "xyz".to_owned()))
            ]
        );
    }

    #[test]
    fn can_process_multiple_requests_split_up_dirtily() {
        let json = vec![
            "{ \
                \"mtype\": \"get\", \
                \"objec",
            "t\": 42, \
                \"property\": \"foobar\" \
            }\n \
			{ \
                \"mtype\": \"set\", \
                \"object\": 102, \
                \"property\": \"abc\", \
				\"value\": 12 \
            }\n \
            { \
				\"mtype\": \"subscribe\", \
                \"object\": 17,",
            "\"property\": \"xyz\" \
			}\n",
        ];
        let mut decoder = JsonDecoder::new();
        let mut result = Vec::new();
        for json in json {
            result.extend(
                decoder
                    .decode(json.as_bytes().to_owned())
                    .expect("failed to decode"),
            );
        }
        assert_eq!(
            result,
            vec![
                Get((42, "foobar".to_owned())),
                Set((102, "abc".to_owned()), Decodable::Integer(12)),
                Subscribe((17, "xyz".to_owned()))
            ]
        );
    }

    #[test]
    fn errors_without_mtype() {
        assert_results_in_error(
            "{ \
                \"object\": 38, \
                \"property\": \"abc\" \
            }\n",
            "does not have an mtype",
        );
    }

    #[test]
    fn errors_with_invalid_mtype() {
        assert_results_in_error(
            "{ \
				\"mtype\": \"get_\", \
                \"object\": 38, \
                \"property\": \"abc\" \
            }\n",
            "invalid mtype",
        );
    }

    #[test]
    fn errors_with_no_object() {
        assert_results_in_error(
            "{ \
                \"mtype\": \"get\", \
                \"property\": \"foobar\" \
            }\n",
            "does not have an object ID",
        );
    }

    #[test]
    fn errors_with_no_property() {
        assert_results_in_error(
            "{ \
                \"mtype\": \"get\", \
                \"object\": 42 \
            }\n",
            "does not have a property",
        );
    }

    #[test]
    fn set_errors_with_no_value() {
        assert_results_in_error(
            "{ \
                \"mtype\": \"set\", \
                \"object\": 42, \
				\"property\": \"foobar\" \
            }\n",
            "does not have a value",
        );
    }

    #[test]
    fn set_errors_with_invalid_value() {
        assert_results_in_error(
            "{ \
                \"mtype\": \"set\", \
                \"object\": 42, \
				\"property\": \"foobar\", \
				\"value\": {} \
            }\n",
            "map not implemented",
        );
    }
}
