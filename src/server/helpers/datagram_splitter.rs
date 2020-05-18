/// Splits a stream of bytes into datagrams
/// Assums a specific byte is always a delimiter
pub struct DatagramSplitter {
    pending_data: Vec<u8>,
    delimiter: u8,
}

impl DatagramSplitter {
    pub fn new(delimiter: u8) -> Self {
        Self {
            pending_data: Vec::new(),
            delimiter,
        }
    }

    /// Splits the given data into datagrams
    /// Saves any leftover bytes to be the start of the next datagram
    pub fn data(&mut self, data: Vec<u8>) -> Vec<Vec<u8>> {
        let delimiter = self.delimiter;
        let mut datagrams = data.split(|b| *b == delimiter);
        let mut first = self.pending_data.split_off(0);
        first.extend(datagrams.next().unwrap_or(&[]));
        let mut result: Vec<Vec<u8>> = std::iter::once(first)
            .chain(datagrams.map(|d| d.to_owned()))
            .collect();
        self.pending_data = result.pop().unwrap();
        result
    }
}

#[cfg(test)]
mod decoder_tests {
    use super::*;

    /// Asserts each call with the strings returns the vecs
    fn assert_splits_to(io: Vec<(&str, Vec<&str>)>) {
        let mut splitter = DatagramSplitter::new(b'|');
        let mut result = Vec::new();
        for packet in &io {
            result.push(splitter.data(packet.0.as_bytes().to_owned()));
        }
        let result_strs: Vec<Vec<&str>> = result
            .iter()
            .map(|v| {
                v.iter()
                    .map(|s| std::str::from_utf8(s).expect("failed to convert to UTF-8 string"))
                    .collect()
            })
            .collect();
        let expected: Vec<Vec<&str>> = io.iter().map(|packet| packet.1.clone()).collect();
        assert_eq!(result_strs, expected);
    }

    #[test]
    fn single_datagram() {
        assert_splits_to(vec![("abc|", vec!["abc"])]);
    }

    #[test]
    fn start_with_delimiter_makes_empty() {
        assert_splits_to(vec![("|", vec![""])]);
    }

    #[test]
    fn single_datagram_with_dirty_end() {
        assert_splits_to(vec![("abc|xy", vec!["abc"])]);
    }

    #[test]
    fn dirty_start() {
        assert_splits_to(vec![("abc", vec![])]);
    }

    #[test]
    fn multiple_datagrams_in_one_call() {
        assert_splits_to(vec![("abc|xyz|", vec!["abc", "xyz"])]);
    }

    #[test]
    fn datagram_split_across_multiple_calls() {
        assert_splits_to(vec![("quer", vec![]), ("ty|", vec!["querty"])]);
    }

    #[test]
    fn multiple_split_up_datagrams() {
        assert_splits_to(vec![
            ("ab", vec![]),
            ("c|xyz|q", vec!["abc", "xyz"]),
            ("uerty|", vec!["querty"]),
        ]);
    }

    #[test]
    fn handles_empty_call() {
        assert_splits_to(vec![
            ("abc|xy", vec!["abc"]),
            ("", vec![]),
            ("z|", vec!["xyz"]),
        ]);
    }
}
