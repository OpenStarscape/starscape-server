use super::*;

/// Returns a list of stotmap of the given length, should only be called once per test for each key
/// type (else you'll get duplicate keys)
pub fn mock_keys<T: Key>(number: u32) -> Vec<T> {
    let mut map = slotmap::DenseSlotMap::with_key();
    (0..number).map(|_| map.insert(())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_keys_all_different() {
        let k: Vec<slotmap::DefaultKey> = mock_keys(3);
        assert_eq!(k.len(), 3);
        assert_ne!(k[0], k[1]);
        assert_ne!(k[0], k[2]);
        assert_ne!(k[1], k[2]);
    }
}
