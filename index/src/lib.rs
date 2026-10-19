use storage::heap_file::RecordId;

pub struct BPlusTree {
    root: LeafNode,
}

struct LeafNode {
    entries: Vec<Entry>,
}

struct Entry {
    key: Vec<u8>,
    record_id: RecordId,
}

impl BPlusTree {
    pub fn new() -> Self {
        BPlusTree {
            root: LeafNode::new(),
        }
    }
    pub fn insert(&mut self, key: Vec<u8>, record_id: RecordId) {
        self.root.insert(key, record_id);
    }
    pub fn get(&self, key: &[u8]) -> Option<RecordId> {
        self.root.get(key)
    }
}
impl Default for BPlusTree {
    fn default() -> Self {
        Self::new()
    }
}

impl LeafNode {
    fn new() -> Self {
        LeafNode { entries: vec![] }
    }
    fn insert(&mut self, key: Vec<u8>, record_id: RecordId) {
        match self.entries.binary_search_by(|entry| entry.key.cmp(&key)) {
            Ok(index) => {
                self.entries[index].record_id = record_id;
            }
            Err(index) => {
                let entry = Entry { key, record_id };
                self.entries.insert(index, entry);
            }
        };
    }
    fn get(&self, key: &[u8]) -> Option<RecordId> {
        self.entries
            .iter()
            .find(|entry| entry.key.as_slice() == key)
            .map(|entry| entry.record_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserted_value_can_be_found_by_key() {
        let mut tree = BPlusTree::new();

        let record_id = RecordId {
            page_id: 0,
            slot_id: 0,
        };

        tree.insert(b"name".to_vec(), record_id);
        let found = tree.get(b"name");

        assert_eq!(found, Some(record_id));
    }
    #[test]
    fn multiple_keys_inserted_out_of_order_can_be_found() {
        let mut tree = BPlusTree::new();

        let name_id = RecordId {
            page_id: 0,
            slot_id: 0,
        };
        let age_id = RecordId {
            page_id: 0,
            slot_id: 1,
        };
        let zip_id = RecordId {
            page_id: 0,
            slot_id: 2,
        };

        tree.insert(b"name".to_vec(), name_id);
        tree.insert(b"zip".to_vec(), zip_id);
        tree.insert(b"age".to_vec(), age_id);

        assert_eq!(tree.get(b"name"), Some(name_id));
        assert_eq!(tree.get(b"age"), Some(age_id));
        assert_eq!(tree.get(b"zip"), Some(zip_id));
        assert_eq!(tree.get(b"missing"), None);
    }
    #[test]
    fn entries_are_kept_in_key_order() {
        let mut tree = BPlusTree::new();

        let record_id = RecordId {
            page_id: 0,
            slot_id: 0,
        };

        tree.insert(b"name".to_vec(), record_id);
        tree.insert(b"zip".to_vec(), record_id);
        tree.insert(b"age".to_vec(), record_id);

        let keys: Vec<&[u8]> = tree
            .root
            .entries
            .iter()
            .map(|entry| entry.key.as_slice())
            .collect();

        assert_eq!(
            keys,
            vec![b"age".as_slice(), b"name".as_slice(), b"zip".as_slice(),]
        );
    }
    #[test]
    fn inserting_existing_key_replaces_record_id() {
        let mut tree = BPlusTree::new();

        let record_id_1 = RecordId {
            page_id: 0,
            slot_id: 0,
        };
        let record_id_2 = RecordId {
            page_id: 0,
            slot_id: 1,
        };

        tree.insert(b"name".to_vec(), record_id_1);
        tree.insert(b"name".to_vec(), record_id_2);

        let keys: Vec<&[u8]> = tree
            .root
            .entries
            .iter()
            .map(|entry| entry.key.as_slice())
            .collect();

        assert_eq!(keys, vec![b"name".as_slice()]);
        assert_eq!(tree.get(b"name"), Some(record_id_2));
    }
}
