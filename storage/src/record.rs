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
        let key_len = self.key.len() as u32;
        let value_len = self.value.len() as u32;

        let mut bytes = Vec::new();

        bytes.extend_from_slice(&key_len.to_le_bytes());
        bytes.extend_from_slice(&value_len.to_le_bytes());
        bytes.push(u8::from(self.tombstone));
        bytes.extend_from_slice(&self.key);
        bytes.extend_from_slice(&self.value);

        Ok(bytes)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let key_len = u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize;

        let value_len = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;

        let tombstone = bytes[8] != 0;

        let key_start = 9;
        let key_end = key_start + key_len;
        let value_end = key_end + value_len;

        let key = bytes[key_start..key_end].to_vec();
        let value = bytes[key_end..value_end].to_vec();

        Ok(Self::new(key, value, tombstone))
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
        assert!(!decoded.tombstone);
    }

    #[test]
    fn tombstone_round_trips() {
        let original = Record::new(b"name".to_vec(), Vec::new(), true);
        let bytes = original.encode().unwrap();
        let decoded = Record::decode(&bytes).unwrap();

        assert_eq!(decoded.key, b"name");
        assert!(decoded.value.is_empty());
        assert!(decoded.tombstone);
    }

    #[test]
    fn rejects_checksum_mismatch() {}

    #[test]
    fn rejects_oversized_value() {}
}
