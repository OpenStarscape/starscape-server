use super::*;
use serde::de::Deserialize;
use serde_json::Value;
use std::convert::{TryFrom, TryInto};

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
        ctx: &dyn DecodeCtx,
        datagram: &serde_json::map::Map<String, Value>,
    ) -> Result<EntityProperty, Box<dyn Error>> {
        let obj = datagram
            .get("object")
            .ok_or("request does not have an object ID")?
            .as_u64()
            .ok_or("object ID not an unsigned int")?;
        let entity = ctx.entity_for(obj)?;
        let prop = datagram
            .get("property")
            .ok_or("request does not have a property")?
            .as_str()
            .ok_or("property not a string")?
            .to_owned();
        Ok((entity, prop))
    }

    fn wrap_property_request(
        &self,
        ctx: &dyn DecodeCtx,
        datagram: &serde_json::map::Map<String, Value>,
        property_request: PropertyRequest,
    ) -> Result<RequestType, Box<dyn Error>> {
        Ok(RequestType::Property(
            Self::decode_obj_prop(ctx, &datagram)?,
            property_request,
        ))
    }

    fn decode_datagram(
        &self,
        ctx: &dyn DecodeCtx,
        bytes: &[u8],
    ) -> Result<RequestType, Box<dyn Error>> {
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
            "set" => self.wrap_property_request(
                ctx,
                &datagram,
                PropertyRequest::Set(
                    datagram
                        .get("value")
                        .ok_or("set request does not have a value")?
                        .clone()
                        .try_into()?,
                ),
            )?,
            "get" => self.wrap_property_request(ctx, &datagram, PropertyRequest::Get)?,
            "subscribe" => {
                self.wrap_property_request(ctx, &datagram, PropertyRequest::Subscribe)?
            }
            "unsubscribe" => {
                self.wrap_property_request(ctx, &datagram, PropertyRequest::Unsubscribe)?
            }
            _ => return Err("request has invalid mtype".into()),
        })
    }
}

impl Decoder for JsonDecoder {
    fn decode(
        &mut self,
        ctx: &dyn DecodeCtx,
        bytes: Vec<u8>,
    ) -> Result<Vec<RequestType>, Box<dyn Error>> {
        let mut requests = Vec::new();
        for datagram in self.splitter.data(bytes) {
            requests.push(self.decode_datagram(ctx, &datagram)?);
        }
        Ok(requests)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use PropertyRequest::*;
    use RequestType::*;

    struct MockDecodeCtx {
        e: Vec<EntityKey>,
    }

    impl DecodeCtx for MockDecodeCtx {
        fn entity_for(&self, obj: ObjectId) -> Result<EntityKey, Box<dyn Error>> {
            if obj < self.e.len() as u64 {
                Ok(self.e[obj as usize])
            } else {
                Err(format!("invalid test object {}", obj).into())
            }
        }
    }

    fn assert_results_in_request(e: &Vec<EntityKey>, json: &str, request: RequestType) {
        let mut decoder = JsonDecoder::new();
        let ctx = MockDecodeCtx { e: e.clone() };
        let result = decoder
            .decode(&ctx, json.as_bytes().to_owned())
            .expect("failed to decode");
        assert_eq!(result, vec![request]);
    }

    fn assert_results_in_error(json: &str, msg: &str) {
        let mut decoder = JsonDecoder::new();
        let e = mock_keys(12);
        let ctx = MockDecodeCtx { e };
        match decoder.decode(&ctx, json.as_bytes().to_owned()) {
            Ok(output) => panic!("should have errored, instead gave: {:?}", output),
            Err(e) if !format!("{}", e).contains(msg) => {
                panic!("{:?} does not contain {:?}", e, msg)
            }
            _ => (),
        }
    }

    #[test]
    fn basic_get_request() {
        let e = mock_keys(12);
        assert_results_in_request(
            &e,
            "{ \
                \"mtype\": \"get\", \
                \"object\": 6, \
                \"property\": \"foobar\" \
            }\n",
            Property((e[6], "foobar".to_owned()), Get),
        );
    }

    #[test]
    fn basic_set_request() {
        let e = mock_keys(12);
        assert_results_in_request(
            &e,
            "{ \
                \"mtype\": \"set\", \
                \"object\": 9, \
                \"property\": \"xyz\", \
				\"value\": null \
            }\n",
            Property((e[9], "xyz".to_owned()), Set(Decodable::Null)),
        );
    }

    #[test]
    fn basic_subscribe_request() {
        let e = mock_keys(12);
        assert_results_in_request(
            &e,
            "{ \
                \"mtype\": \"subscribe\", \
                \"object\": 2, \
                \"property\": \"abc\" \
            }\n",
            Property((e[2], "abc".to_owned()), Subscribe),
        );
    }

    #[test]
    fn basic_unsubscribe_request() {
        let e = mock_keys(12);
        assert_results_in_request(
            &e,
            "{ \
                \"mtype\": \"unsubscribe\", \
                \"object\": 11, \
                \"property\": \"abc\" \
            }\n",
            Property((e[11], "abc".to_owned()), Unsubscribe),
        );
    }

    #[test]
    fn can_process_multiple_requests_split_up_cleanly() {
        let json = vec![
            "{ \
                \"mtype\": \"get\", \
                \"object\": 2, \
                \"property\": \"foobar\" \
            }\n",
            "{\
                \"mtype\": \"set\", \
                \"object\": 8, \
                \"property\": \"abc\", \
				\"value\": 12 \
            }\n",
            "{ \
				\"mtype\": \"subscribe\", \
                \"object\": 11, \
                \"property\": \"xyz\" \
			}\n",
        ];
        let mut decoder = JsonDecoder::new();
        let mut result = Vec::new();
        let e = mock_keys(12);
        let ctx = MockDecodeCtx { e: e.clone() };
        for json in json {
            result.extend(
                decoder
                    .decode(&ctx, json.as_bytes().to_owned())
                    .expect("failed to decode"),
            );
        }
        assert_eq!(
            result,
            vec![
                Property((e[2], "foobar".to_owned()), Get),
                Property((e[8], "abc".to_owned()), Set(Decodable::Integer(12))),
                Property((e[11], "xyz".to_owned()), Subscribe)
            ]
        );
    }

    #[test]
    fn can_process_multiple_requests_at_once() {
        let json = "{ \
                \"mtype\": \"get\", \
                \"object\": 3, \
                \"property\": \"foobar\" \
            }\n \
			{ \
                \"mtype\": \"set\", \
                \"object\": 5, \
                \"property\": \"abc\", \
				\"value\": 12 \
            }\n \
			{ \
				\"mtype\": \"subscribe\", \
                \"object\": 7, \
                \"property\": \"xyz\" \
			}\n";
        let mut decoder = JsonDecoder::new();
        let e = mock_keys(12);
        let ctx = MockDecodeCtx { e: e.clone() };
        let result = decoder
            .decode(&ctx, json.as_bytes().to_owned())
            .expect("failed to decode");
        assert_eq!(
            result,
            vec![
                Property((e[3], "foobar".to_owned()), Get),
                Property((e[5], "abc".to_owned()), Set(Decodable::Integer(12))),
                Property((e[7], "xyz".to_owned()), Subscribe)
            ]
        );
    }

    #[test]
    fn can_process_multiple_requests_split_up_dirtily() {
        let json = vec![
            "{ \
                \"mtype\": \"get\", \
                \"objec",
            "t\": 9, \
                \"property\": \"foobar\" \
            }\n \
			{ \
                \"mtype\": \"set\", \
                \"object\": 2, \
                \"property\": \"abc\", \
				\"value\": 12 \
            }\n \
            { \
				\"mtype\": \"subscribe\", \
                \"object\": 1,",
            "\"property\": \"xyz\" \
			}\n",
        ];
        let mut decoder = JsonDecoder::new();
        let mut result = Vec::new();
        let e = mock_keys(12);
        let ctx = MockDecodeCtx { e: e.clone() };
        for json in json {
            result.extend(
                decoder
                    .decode(&ctx, json.as_bytes().to_owned())
                    .expect("failed to decode"),
            );
        }
        assert_eq!(
            result,
            vec![
                Property((e[9], "foobar".to_owned()), Get),
                Property((e[2], "abc".to_owned()), Set(Decodable::Integer(12))),
                Property((e[1], "xyz".to_owned()), Subscribe)
            ]
        );
    }

    #[test]
    fn errors_without_mtype() {
        assert_results_in_error(
            "{ \
                \"object\": 4, \
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
                \"object\": 3, \
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
                \"object\": 8 \
            }\n",
            "does not have a property",
        );
    }

    #[test]
    fn set_errors_with_no_value() {
        assert_results_in_error(
            "{ \
                \"mtype\": \"set\", \
                \"object\": 6, \
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
                \"object\": 5, \
				\"property\": \"foobar\", \
				\"value\": {} \
            }\n",
            "map not implemented",
        );
    }
}
