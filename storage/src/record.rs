use crate::{Error, Result};

pub(crate) struct Record {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    pub tombstone: bool,
}

impl Record {
    pub fn new(key: Vec<u8>, value: Vec<u8>, tombstone: bool) -> Self {
        Self {
            key,
            value,
            tombstone,
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        todo!()
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_record_round_trips() {
        let original = Record::new(b"name".to_vec(), b"Taro".to_vec(), false);

        let bytes = original.encode().unwrap();
        let decoded = Record::decode(&bytes).unwrap();

        assert_eq!(decoded.key, b"name");
        assert_eq!(decoded.value, b"Taro");
        assert!(decoded.tombstone);
    }

    #[test]
    fn tombstone_round_trips() {}

    #[test]
    fn rejects_checksum_mismatch() {}

    #[test]
    fn rejects_oversized_value() {}
}
