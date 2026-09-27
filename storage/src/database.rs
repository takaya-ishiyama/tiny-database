use std::collections::HashMap;

use crate::record::Record;

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
