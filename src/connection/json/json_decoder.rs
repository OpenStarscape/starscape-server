use super::*;
use serde::de::Deserialize;
use serde_json::Value;

pub struct JsonDecoder {
    splitter: DatagramSplitter,
}

impl JsonDecoder {
    pub fn new() -> Self {
        Self {
            splitter: DatagramSplitter::new(b'\n'),
        }
    }

    /// For disambiguation purposes, some types are wrapped in an array. This function handles them.
    fn decode_wrapper_array(
        &self,
        ctx: &dyn DecodeCtx,
        array: &[Value],
    ) -> Result<Decoded, String> {
        match array.len() {
            3 => {
                let component = |value: &Value| {
                    value
                        .as_f64()
                        .ok_or_else(|| format!("{} is an invalid vector component", value))
                };
                Ok(Decoded::Vector(Vector3::new(
                    component(&array[0])?,
                    component(&array[1])?,
                    component(&array[2])?,
                )))
            }
            1 => {
                if let Some(obj_id) = array[0].as_i64() {
                    // An array-wrapped int is an object ID
                    ctx.entity_for(obj_id as u64).map(Decoded::Entity)
                } else if let Some(array) = array[0].as_array() {
                    // An array-wrapped array is an actual array
                    let result: Result<Vec<_>, _> = array
                        .iter()
                        .map(|value| self.decode_value(ctx, value))
                        .collect();
                    Ok(Decoded::Array(result?))
                } else {
                    Err(format!(
                        "{} is an array-wrapped value, but not an object ID",
                        array[0]
                    ))
                }
            }
            len => Err(format!(
                "non-wrapped array with length {} is not valid",
                len
            )),
        }
    }

    /// Ideally we would implement some strange Deserialize trait for minimal copying, but aint
    /// nobody got time for that.
    fn decode_value(&self, ctx: &dyn DecodeCtx, value: &Value) -> Result<Decoded, String> {
        match value {
            Value::Null => Ok(Decoded::Null),
            Value::Bool(_) => Err("decoding bool not implemented".to_string()),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(Decoded::Integer(i))
                } else if let Some(f) = n.as_f64() {
                    Ok(Decoded::Scalar(f))
                } else {
                    Err(format!("{} is an invalid number", value))
                }
            }
            Value::String(text) => Ok(Decoded::Text(text.to_string())),
            Value::Array(array) => self.decode_wrapper_array(ctx, array),
            Value::Object(_) => Err("decoding map not implemented".to_string()),
        }
    }

    fn decode_obj(
        ctx: &dyn DecodeCtx,
        datagram: &serde_json::map::Map<String, Value>,
    ) -> Result<EntityKey, Box<dyn Error>> {
        let obj = datagram
            .get("object")
            .ok_or("request does not have an object ID")?
            .as_u64()
            .ok_or("object ID not an unsigned int")?;
        ctx.entity_for(obj).map_err(Into::into)
    }

    fn decode_name(
        datagram: &serde_json::map::Map<String, Value>,
    ) -> Result<String, Box<dyn Error>> {
        Ok(datagram
            .get("property")
            .ok_or("request does not have a property")?
            .as_str()
            .ok_or("property not a string")?
            .to_string())
    }

    fn decode_datagram(
        &self,
        ctx: &dyn DecodeCtx,
        bytes: &[u8],
    ) -> Result<Request, Box<dyn Error>> {
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
            "fire" => Request::fire(
                Self::decode_obj(ctx, &datagram)?,
                Self::decode_name(&datagram)?,
                self.decode_value(
                    ctx,
                    datagram
                        .get("value")
                        .ok_or("fire request does not have a value")?,
                )?,
            ),
            "set" => Request::set(
                Self::decode_obj(ctx, &datagram)?,
                Self::decode_name(&datagram)?,
                self.decode_value(
                    ctx,
                    datagram
                        .get("value")
                        .ok_or("set request does not have a value")?,
                )?,
            ),
            "get" => Request::get(
                Self::decode_obj(ctx, &datagram)?,
                Self::decode_name(&datagram)?,
            ),
            "subscribe" => Request::subscribe(
                Self::decode_obj(ctx, &datagram)?,
                Self::decode_name(&datagram)?,
            ),
            "unsubscribe" => Request::unsubscribe(
                Self::decode_obj(ctx, &datagram)?,
                Self::decode_name(&datagram)?,
            ),
            _ => return Err("request has invalid mtype".into()),
        })
    }
}

impl Decoder for JsonDecoder {
    fn decode(
        &mut self,
        ctx: &dyn DecodeCtx,
        bytes: Vec<u8>,
    ) -> Result<Vec<Request>, Box<dyn Error>> {
        let mut requests = Vec::new();
        for datagram in self.splitter.data(bytes) {
            requests.push(self.decode_datagram(ctx, &datagram)?);
        }
        Ok(requests)
    }
}

#[cfg(test)]
struct MockDecodeCtx {
    e: Vec<EntityKey>,
}

#[cfg(test)]
impl MockDecodeCtx {
    fn new(count: u32) -> Self {
        Self {
            e: mock_keys(count),
        }
    }
}

#[cfg(test)]
impl std::ops::Index<usize> for MockDecodeCtx {
    type Output = EntityKey;
    fn index(&self, i: usize) -> &EntityKey {
        &self.e[i]
    }
}

#[cfg(test)]
impl DecodeCtx for MockDecodeCtx {
    fn entity_for(&self, obj: ObjectId) -> Result<EntityKey, String> {
        self.e
            .get(obj as usize)
            .cloned()
            .ok_or_else(|| "invalid obj".to_string())
    }
}

#[cfg(test)]
mod decode_tests {
    use super::*;
    use Encodable::*;

    struct TerrifiedDecodeCtx;

    impl DecodeCtx for TerrifiedDecodeCtx {
        fn entity_for(&self, _obj: ObjectId) -> Result<EntityKey, String> {
            panic!("should not have been called")
        }
    }

    fn decode(ctx: &dyn DecodeCtx, json: &str) -> Result<Decoded, Box<dyn Error>> {
        let decoder = JsonDecoder::new();
        let mut deserializer = serde_json::Deserializer::from_slice(json.as_bytes());
        let value = Value::deserialize(&mut deserializer).expect("failed to deserialize");
        Ok(decoder.decode_value(ctx, &value)?)
    }

    fn assert_decodes_to_with_ctx(ctx: &dyn DecodeCtx, json: &str, expected: Decoded) {
        let actual = decode(ctx, json).expect("failed to turn into decodable");
        assert_eq!(actual, expected);
    }

    fn assert_decodes_to(json: &str, expected: Decoded) {
        assert_decodes_to_with_ctx(&TerrifiedDecodeCtx, json, expected)
    }

    fn assert_results_in_error_with_ctx(ctx: &dyn DecodeCtx, json: &str, msg: &str) {
        match decode(ctx, json) {
            Ok(output) => panic!("should have errored, instead gave: {:?}", output),
            Err(e) if !format!("{}", e).contains(msg) => {
                panic!("{:?} does not contain {:?}", e, msg)
            }
            _ => (),
        }
    }

    fn assert_results_in_error(json: &str, msg: &str) {
        assert_results_in_error_with_ctx(&TerrifiedDecodeCtx, json, msg)
    }

    #[test]
    fn integer() {
        assert_decodes_to("-583", Integer(-583));
    }

    #[test]
    fn scalar() {
        assert_decodes_to("784.25", Scalar(784.25));
    }

    #[test]
    fn scalar_even_when_decimal_is_zero() {
        assert_decodes_to("784.0", Scalar(784.0));
    }

    #[test]
    fn text() {
        assert_decodes_to("\"hello\\n\"", Text("hello\n".to_string()));
    }

    #[test]
    fn null() {
        assert_decodes_to("null", Null);
    }

    #[test]
    fn vector() {
        assert_decodes_to("[-12, 0.0, 2.5]", Vector(Vector3::new(-12.0, 0.0, 2.5)));
    }

    #[test]
    fn array() {
        assert_decodes_to(
            "[[[1, 1, 1], 74, -0.5]]",
            Array(vec![
                Vector(Vector3::new(1.0, 1.0, 1.0)),
                Integer(74),
                Scalar(-0.5),
            ]),
        );
    }

    #[test]
    fn entity() {
        let ctx = MockDecodeCtx::new(12);
        assert_decodes_to_with_ctx(&ctx, "[7]", Entity(ctx[7]));
    }

    #[test]
    fn array_with_entities() {
        let ctx = MockDecodeCtx::new(12);
        assert_decodes_to_with_ctx(
            &ctx,
            "[[[4], 7, [2]]]",
            Array(vec![Entity(ctx[4]), Integer(7), Entity(ctx[2])]),
        );
    }

    #[test]
    fn array_size_two_is_error() {
        assert_results_in_error("[1, 2]", "length 2");
    }

    #[test]
    fn array_size_zero_is_error() {
        assert_results_in_error("[]", "length 0");
    }

    #[test]
    fn vector_of_obj_ids_is_error() {
        assert_results_in_error("[[1], [2], [3]]", "invalid vector component");
    }

    #[test]
    fn array_wrapped_scalar_is_error() {
        assert_results_in_error("[7.1]", "not an object ID");
    }

    #[test]
    fn array_wrapped_scalar_is_error_even_when_decimal_is_zero() {
        assert_results_in_error("[7.0]", "not an object ID");
    }

    #[test]
    fn unknown_object_is_error() {
        assert_results_in_error_with_ctx(&MockDecodeCtx::new(12), "[88]", "invalid obj");
    }
}

#[cfg(test)]
mod message_tests {
    use super::*;

    fn assert_results_in_request(ctx: &dyn DecodeCtx, json: &str, request: Request) {
        let mut decoder = JsonDecoder::new();
        let result = decoder
            .decode(ctx, json.as_bytes().to_owned())
            .expect("failed to decode");
        assert_eq!(result, vec![request]);
    }

    fn assert_results_in_error(json: &str, msg: &str) {
        let mut decoder = JsonDecoder::new();
        let ctx = MockDecodeCtx::new(12);
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
        let e = MockDecodeCtx::new(12);
        assert_results_in_request(
            &e,
            "{ \
                \"mtype\": \"get\", \
                \"object\": 6, \
                \"property\": \"foobar\" \
            }\n",
            Request::get(e[6], "foobar".to_owned()),
        );
    }

    #[test]
    fn basic_set_request() {
        let e = MockDecodeCtx::new(12);
        assert_results_in_request(
            &e,
            "{ \
                \"mtype\": \"set\", \
                \"object\": 9, \
                \"property\": \"xyz\", \
				\"value\": null \
            }\n",
            Request::set(e[9], "xyz".to_owned(), Decoded::Null),
        );
    }

    #[test]
    fn basic_fire_request() {
        let e = MockDecodeCtx::new(12);
        assert_results_in_request(
            &e,
            "{ \
                \"mtype\": \"fire\", \
                \"object\": 9, \
                \"property\": \"xyz\", \
				\"value\": 12 \
            }\n",
            Request::fire(e[9], "xyz".to_owned(), Decoded::Integer(12)),
        );
    }

    #[test]
    fn basic_subscribe_request() {
        let e = MockDecodeCtx::new(12);
        assert_results_in_request(
            &e,
            "{ \
                \"mtype\": \"subscribe\", \
                \"object\": 2, \
                \"property\": \"abc\" \
            }\n",
            Request::subscribe(e[2], "abc".to_owned()),
        );
    }

    #[test]
    fn basic_unsubscribe_request() {
        let e = MockDecodeCtx::new(12);
        assert_results_in_request(
            &e,
            "{ \
                \"mtype\": \"unsubscribe\", \
                \"object\": 11, \
                \"property\": \"abc\" \
            }\n",
            Request::unsubscribe(e[11], "abc".to_owned()),
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
        let e = MockDecodeCtx::new(12);
        for json in json {
            result.extend(
                decoder
                    .decode(&e, json.as_bytes().to_owned())
                    .expect("failed to decode"),
            );
        }
        assert_eq!(
            result,
            vec![
                Request::get(e[2], "foobar".to_owned()),
                Request::set(e[8], "abc".to_owned(), Decoded::Integer(12)),
                Request::subscribe(e[11], "xyz".to_owned())
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
        let e = MockDecodeCtx::new(12);
        let result = decoder
            .decode(&e, json.as_bytes().to_owned())
            .expect("failed to decode");
        assert_eq!(
            result,
            vec![
                Request::get(e[3], "foobar".to_owned()),
                Request::set(e[5], "abc".to_owned(), Decoded::Integer(12)),
                Request::subscribe(e[7], "xyz".to_owned())
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
        let e = MockDecodeCtx::new(12);
        for json in json {
            result.extend(
                decoder
                    .decode(&e, json.as_bytes().to_owned())
                    .expect("failed to decode"),
            );
        }
        assert_eq!(
            result,
            vec![
                Request::get(e[9], "foobar".to_owned()),
                Request::set(e[2], "abc".to_owned(), Decoded::Integer(12)),
                Request::subscribe(e[1], "xyz".to_owned())
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
