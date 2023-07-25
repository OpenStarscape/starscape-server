use std::any::type_name;

pub fn short_type_name<T>() -> &'static str {
    let name = type_name::<T>();
    let end = name.find('<').unwrap_or(name.len());
    let start = name[..end].rfind(':').map(|i| i + 1).unwrap_or(0);
    &name[start..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_short_type_name() {
        struct Foo;
        assert_ne!(type_name::<Foo>(), "Foo");
        assert_eq!(short_type_name::<Foo>(), "Foo");
    }

    #[test]
    fn returns_short_type_name_unit_type() {
        assert_eq!(short_type_name::<()>(), "()");
    }

    #[test]
    fn returns_short_type_name_vec() {
        struct Foo;
        assert_eq!(short_type_name::<Vec<Foo>>(), "Vec");
    }
}
