use std::fmt::Formatter;

pub fn format_slotmap_key<T: slotmap::Key>(
    f: &mut Formatter,
    type_name: &str,
    key: T,
) -> std::fmt::Result {
    if !key.is_null() {
        let raw_id = key.data().as_ffi();
        // This depends on undefined SlotMap internals, but whatever. No big deal if it breaks.
        let version = raw_id >> 32;
        let index = (raw_id << 32) >> 32;
        write!(f, "{}#{}:{}", type_name, index, version)
    } else {
        write!(f, "{}#null", type_name)
    }
}
