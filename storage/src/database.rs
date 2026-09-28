use std::{collections::HashMap, path::Path};

use crate::{Result, log::Log, record::Record};

pub struct Database {
    log: Log,
    index: HashMap<Vec<u8>, Vec<u8>>,
}

pub(crate) fn rebuild_index(records: Vec<Record>) -> HashMap<Vec<u8>, Vec<u8>> {
    let mut index = HashMap::new();

    for record in records {
        if record.tombstone {
            index.remove(&record.key);
        } else {
            index.insert(record.key, record.value);
        }
    }

    index
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let mut log = Log::open(path)?;
        let records = log.scan()?;
        let index = rebuild_index(records);

        Ok(Self { log, index })
    }

    pub fn get(&self, key: &[u8]) -> Option<&[u8]> {
        self.index.get(key).map(Vec::as_slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latest_value_wins() {
        let record = vec![
            Record::new(b"name".to_vec(), b"Taro".to_vec(), false),
            Record::new(b"name".to_vec(), b"Jiro".to_vec(), false),
        ];

        let index = rebuild_index(record);

        assert_eq!(index.get(b"name".as_slice()), Some(&b"Jiro".to_vec()));
    }

    #[test]
    fn tombstone_removes_key() {
        let records = vec![
            Record::new(b"name".to_vec(), b"Taro".to_vec(), false),
            Record::new(b"name".to_vec(), b"Jiro".to_vec(), true),
        ];
        let index = rebuild_index(records);
        assert_eq!(index.get(b"name".as_slice()), None);
    }
}
