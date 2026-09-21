use crate::{Error, Result};
use crc32fast::hash;

pub(crate) struct Record {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    pub tombstone: bool,
}

const HEADER_SIZE: usize = 13;
const MAX_KEY_SIZE: usize = 1024;
const MAX_VALUE_SIZE: usize = 1024 * 1024;

impl Record {
    pub fn new(key: Vec<u8>, value: Vec<u8>, tombstone: bool) -> Self {
        Self {
            key,
            value,
            tombstone,
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        Self::check_sizes(self.key.len(), self.value.len())?;

        let key_len = self.key.len() as u32;
        let value_len = self.value.len() as u32;

        let mut body = Vec::new();

        body.extend_from_slice(&key_len.to_le_bytes());
        body.extend_from_slice(&value_len.to_le_bytes());
        body.push(u8::from(self.tombstone));
        body.extend_from_slice(&self.key);
        body.extend_from_slice(&self.value);

        let checksum = hash(&body);

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&checksum.to_le_bytes());
        bytes.extend_from_slice(&body);

        Ok(bytes)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < HEADER_SIZE {
            return Err(Error::TruncatedRecord {
                expected: HEADER_SIZE,
                actual: bytes.len(),
            });
        }

        // チェックサムの検証
        let expected_checksum = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let actual_checksum = hash(&bytes[4..]);
        if expected_checksum != actual_checksum {
            return Err(Error::ChecksumMismatch {
                expected: expected_checksum,
                actual: actual_checksum,
            });
        }

        // key_lenとvalue_lenを取得
        let key_len = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
        let value_len = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;

        Self::check_sizes(key_len, value_len)?;

        let expected_size = HEADER_SIZE + key_len + value_len;
        if bytes.len() < expected_size {
            return Err(Error::TruncatedRecord {
                expected: expected_size,
                actual: bytes.len(),
            });
        }

        let tombstone = match bytes[12] {
            0 => false,
            1 => true,
            value => return Err(Error::InvalidTombstone(value)),
        };

        let key_start = HEADER_SIZE;
        let key_end = key_start + key_len;
        let value_end = key_end + value_len;

        // keyとvalueの範囲をチェック
        if bytes.len() < value_end {
            return Err(Error::TruncatedRecord {
                expected: value_end,
                actual: bytes.len(),
            });
        }

        let key = bytes[key_start..key_end].to_vec();
        let value = bytes[key_end..value_end].to_vec();

        Ok(Self::new(key, value, tombstone))
    }

    fn check_sizes(key_len: usize, value_len: usize) -> Result<()> {
        if key_len > MAX_KEY_SIZE {
            return Err(Error::KeyTooLarge {
                actual: key_len,
                max: MAX_KEY_SIZE,
            });
        }

        if value_len > MAX_VALUE_SIZE {
            return Err(Error::ValueTooLarge {
                actual: value_len,
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
        bytes[12] = 2;

        // encodeで計算されたチェックサムを再計算して書き換える
        let checksum = hash(&bytes[4..]);
        bytes[0..4].copy_from_slice(&checksum.to_le_bytes());

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
    fn rejects_truncated_record() {
        let result = Record::decode(&[]);
        assert!(matches!(
            result,
            Err(Error::TruncatedRecord {
                expected: 13,
                actual: 0
            })
        ))
    }

    #[test]
    fn refects_oversized_key() {
        let oversized_key = vec![0u8; MAX_KEY_SIZE + 1]; // 1025 bytes

        let record = Record::new(oversized_key, b"value".to_vec(), false);
        let result = record.encode();

        assert!(matches!(
            result,
            Err(Error::KeyTooLarge {
                actual: 1025,
                max: 1024
            })
        ));
    }

    #[test]
    fn decode_rejects_oversized_key() {
        let key_len = (MAX_KEY_SIZE + 1) as u32;
        let value_len = 0u32;

        let mut body = Vec::new();
        body.extend_from_slice(&key_len.to_le_bytes());
        body.extend_from_slice(&value_len.to_le_bytes());
        body.push(0); // tombstone

        let checksum = hash(&body);

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&checksum.to_le_bytes());
        bytes.extend_from_slice(&body);

        let result = Record::decode(&bytes);

        assert!(matches!(
            result,
            Err(Error::KeyTooLarge {
                actual: 1025,
                max: 1024,
            })
        ));
    }
}
