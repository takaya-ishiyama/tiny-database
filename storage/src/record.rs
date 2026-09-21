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
        self.check_max_size()?;

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
        const HEADER_SIZE: usize = 9;

        if bytes.len() < HEADER_SIZE {
            return Err(Error::TruncatedRecord {
                expected: HEADER_SIZE,
                actual: bytes.len(),
            });
        }

        let key_len = u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize;

        let value_len = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;

        let expected_size = HEADER_SIZE + key_len + value_len;
        if bytes.len() < expected_size {
            return Err(Error::TruncatedRecord {
                expected: expected_size,
                actual: bytes.len(),
            });
        }

        let tombstone = match bytes[8] {
            0 => false,
            1 => true,
            value => return Err(Error::InvalidTombstone(value)),
        };

        let key_start = 9;
        let key_end = key_start + key_len;
        let value_end = key_end + value_len;

        let key = bytes[key_start..key_end].to_vec();
        let value = bytes[key_end..value_end].to_vec();

        Ok(Self::new(key, value, tombstone))
    }

    fn check_max_size(&self) -> Result<()> {
        const MAX_KEY_SIZE: usize = 1024;
        const MAX_VALUE_SIZE: usize = 1024 * 1024;

        if self.key.len() > MAX_KEY_SIZE {
            return Err(Error::KeyTooLarge {
                actual: self.key.len(),
                max: MAX_KEY_SIZE,
            });
        }

        if self.value.len() > MAX_VALUE_SIZE {
            return Err(Error::ValueTooLarge {
                actual: self.value.len(),
                max: MAX_VALUE_SIZE,
            });
        }
        Ok(())
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
    fn rejects_checksum_mismatch() {
        let record = Record::new(b"name".to_vec(), b"Taro".to_vec(), false);
        let mut bytes = record.encode().unwrap();

        // valueの最後の1バイトを意図的に壊す
        let last = bytes.len() - 1;
        bytes[last] ^= 0xFF;

        let result = Record::decode(&bytes);
        assert!(matches!(result, Err(Error::ChecksumMismatch { .. })));
    }

    #[test]
    fn rejects_check_invalid_tombstone() {
        let record = Record::new(b"name".to_vec(), b"Taro".to_vec(), false);
        let mut bytes = record.encode().unwrap();

        // tombstoneが入っている9番目のバイトを不正な値に書き換える
        bytes[8] = 2;
        let result = Record::decode(&bytes);
        assert!(matches!(result, Err(Error::InvalidTombstone(2))));
    }

    #[test]
    fn rejects_oversized_value() {
        let orversized_value = vec![0u8; 1024 * 1024 * 10]; // 10 MB

        let record = Record::new(b"key".to_vec(), orversized_value, false);
        let result = record.encode();
        assert!(matches!(
            result,
            Err(Error::ValueTooLarge {
                actual: 10_485_760,
                max: 1_048_576
            })
        ));
    }

    #[test]
    fn rejects_trancated_record() {
        let result = Record::decode(&[]);
        assert!(matches!(
            result,
            Err(Error::TruncatedRecord {
                expected: 9,
                actual: 0
            })
        ))
    }
}
