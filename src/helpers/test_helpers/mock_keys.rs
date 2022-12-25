use super::*;

pub fn mock_keys<T: Key>(number: u32) -> Vec<T> {
    let mut map = slotmap::SlotMap::with_key();
    (0..number).map(|_| map.insert(())).collect()
}

/// Returns a list of IDs of the given length, should only be called once per test for each key
/// type (else you'll get duplicate keys)
pub fn mock_ids<T>(number: u32) -> Vec<Id<T>> {
    let mut a = slotmap::SlotMap::with_key();
    let mut b = slotmap::SlotMap::with_key();
    (0..number)
        .map(|_| Id::new(a.insert(()), b.insert(())))
        .collect()
}

pub fn mock_generic_ids(number: u32) -> Vec<GenericId> {
    mock_ids::<()>(number).into_iter().map(Into::into).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_keys_all_different() {
        let k = mock_ids::<()>(3);
        assert_eq!(k.len(), 3);
        assert_ne!(k[0], k[1]);
        assert_ne!(k[0], k[2]);
        assert_ne!(k[1], k[2]);
    }
}
