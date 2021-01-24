use super::*;

/// Doesn't work for everything, but can be useful in tests
pub fn attempt_any_to_string(any: &dyn Any) -> String {
    if let Some(s) = any.downcast_ref::<&str>() {
        format!("&str{{ {:?} }}", s)
    } else if let Some(s) = any.downcast_ref::<String>() {
        format!("String{{ {:?} }}", s)
    } else if let Some(b) = any.downcast_ref::<Box<dyn Any>>() {
        format!("Box{{ {} }}", attempt_any_to_string(&**b))
    } else if let Some(b) = any.downcast_ref::<Box<dyn Any + Send>>() {
        format!("Box{{ {} }}", attempt_any_to_string(&**b))
    } else if let Some(b) = any.downcast_ref::<Box<dyn Any + Send + Sync>>() {
        format!("Box{{ {} }}", attempt_any_to_string(&**b))
    } else {
        format!("non-string Any value with {:?}", any.type_id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn works_on_str() {
        assert_eq!(attempt_any_to_string(&"foo"), "&str{ \"foo\" }");
    }

    #[test]
    fn works_on_string() {
        assert_eq!(
            attempt_any_to_string(&("foo").to_string()),
            "String{ \"foo\" }"
        );
    }

    #[test]
    fn works_on_boxed_any() {
        let boxed_any: Box<dyn Any> = Box::new("foo");
        let boxed_send_any: Box<dyn Any + Send> = Box::new("foo");
        let boxed_send_sync_any: Box<dyn Any + Send + Sync> = Box::new("foo");
        let expected = "Box{ &str{ \"foo\" } }";
        assert_eq!(attempt_any_to_string(&boxed_any), expected);
        assert_eq!(attempt_any_to_string(&boxed_send_any), expected);
        assert_eq!(attempt_any_to_string(&boxed_send_sync_any), expected);
    }
}
