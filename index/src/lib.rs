use storage::heap_file::RecordId;

struct BPlusTree {}

struct LeafNode {
    key: u8,
    recode_id: RecordId,
}

impl BPlusTree {
    fn new() -> Self {
        BPlusTree {}
    }
    fn insert(&self, record: Vec<u8>, record_id: RecordId) {
        todo!()
    }
    fn get(&self,)
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
