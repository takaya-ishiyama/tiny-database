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
        let entry = Entry { key, record_id };
        self.entries.push(entry);
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
}
