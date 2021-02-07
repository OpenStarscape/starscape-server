use std::error::Error;

/// Splits a stream of bytes into datagrams
/// Assums a specific byte is always a delimiter
pub struct DatagramSplitter {
    pending_data: Vec<u8>,
    delimiter: u8,
    max_buffer: usize,
}

impl DatagramSplitter {
    pub fn new(delimiter: u8, max_buffer: usize) -> Self {
        Self {
            pending_data: Vec::new(),
            delimiter,
            max_buffer,
        }
    }

    /// Splits the given data into datagrams
    /// Saves any leftover bytes to be the start of the next datagram
    pub fn data(&mut self, data: Vec<u8>) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
        let delimiter = self.delimiter;
        let mut datagrams = data.split(|b| *b == delimiter);
        let mut first = self.pending_data.split_off(0);
        let first_of_data = datagrams.next().unwrap_or(&[]);
        if first.len() + first_of_data.len() > self.max_buffer {
            self.pending_data = vec![];
            return Err("datagram too long".into());
        }
        first.extend(first_of_data);
        let result: Result<Vec<Vec<u8>>, Box<dyn Error>> = std::iter::once(Ok(first))
            .chain(datagrams.map(|d| {
                if d.len() > self.max_buffer {
                    Err("datagram too long".into())
                } else {
                    Ok(d.to_owned())
                }
            }))
            .collect();
        match result {
            Ok(mut datagrams) => {
                self.pending_data = datagrams.pop().unwrap();
                Ok(datagrams.into_iter().filter(|d| !d.is_empty()).collect())
            }
            Err(e) => {
                self.pending_data = vec![];
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod decoder_tests {
    use super::*;

    /// Asserts each call with the strings returns the vecs
    fn assert_splits_to(io: Vec<(&str, Vec<&str>)>) {
        let mut splitter = DatagramSplitter::new(b'|', usize::MAX);
        let mut result = Vec::new();
        for packet in &io {
            result.push(splitter.data(packet.0.as_bytes().to_owned()).unwrap());
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

    #[test]
    fn start_with_delimiter_is_ignored() {
        assert_splits_to(vec![("|", vec![])]);
    }

    #[test]
    fn ignores_empty_datagram() {
        assert_splits_to(vec![("abc||", vec!["abc"])]);
    }

    #[test]
    fn ignores_empty_datagrams_across_multiple_calls() {
        assert_splits_to(vec![("abc|", vec!["abc"]), ("|xyz|||", vec!["xyz"])]);
    }

    #[test]
    fn does_not_error_if_each_packet_small_enough() {
        let mut splitter = DatagramSplitter::new(b'|', 4);
        assert!(splitter.data("abc|".as_bytes().to_owned()).is_ok());
        assert!(splitter.data("abc|xyz|i".as_bytes().to_owned()).is_ok());
        assert!(splitter.data("|ab".as_bytes().to_owned()).is_ok());
    }

    #[test]
    fn erros_with_too_much_data() {
        let mut splitter = DatagramSplitter::new(b'|', 4);
        assert!(splitter.data("ab|ab".as_bytes().to_owned()).is_ok());
        assert!(splitter.data("xyz".as_bytes().to_owned()).is_err());
    }
}
