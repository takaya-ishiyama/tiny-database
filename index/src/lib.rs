use storage::heap_file::RecordId;

const MAX_LEAF_ENTRIES: usize = 3;

enum Node {
    Leaf(LeafNode),
    Internal(InternalNode),
}

pub struct BPlusTree {
    root: Node,
}

struct LeafNode {
    entries: Vec<Entry>,
}

pub struct InternalNode {
    keys: Vec<Vec<u8>>,
    children: Vec<Node>,
}

struct Entry {
    key: Vec<u8>,
    record_id: RecordId,
}

impl BPlusTree {
    pub fn new() -> Self {
        BPlusTree {
            root: Node::Leaf(LeafNode::new()),
        }
    }
    pub fn insert(&mut self, key: Vec<u8>, record_id: RecordId) {
        let should_split = match &mut self.root {
            Node::Leaf(leaf) => {
                leaf.insert(key, record_id);
                leaf.entries.len() > MAX_LEAF_ENTRIES
            }
            Node::Internal(internal) => {
                internal.insert(key, record_id);
                return;
            }
        };
        if should_split {
            let old_root = std::mem::replace(&mut self.root, Node::Leaf(LeafNode::new()));

            let Node::Leaf(mut left_leaf) = old_root else {
                unreachable!();
            };

            let (separator_key, right_leaf) = left_leaf.split();

            self.root = Node::Internal(InternalNode {
                keys: vec![separator_key],
                children: vec![Node::Leaf(left_leaf), Node::Leaf(right_leaf)],
            })
        }
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

impl Node {
    fn get(&self, key: &[u8]) -> Option<RecordId> {
        match self {
            Node::Leaf(leaf) => leaf.get(key),
            Node::Internal(internal) => internal.get(key),
        }
    }
}

impl InternalNode {
    fn insert(&mut self, key: Vec<u8>, record_id: RecordId) {
        let child_index = self
            .keys
            .partition_point(|separator| separator.as_slice() <= &key);
        match &mut self.children[child_index] {
            Node::Leaf(leaf) => {
                leaf.insert(key, record_id);
                if leaf.entries.len() <= MAX_LEAF_ENTRIES {
                    return;
                }
                let (separator_key, right_leaf) = leaf.split();
                self.keys.insert(child_index, separator_key);
                self.children
                    .insert(child_index + 1, Node::Leaf(right_leaf));
            }
            Node::Internal(internal) => internal.insert(key, record_id),
        }
    }
    fn get(&self, key: &[u8]) -> Option<RecordId> {
        let child_index = self
            .keys
            .partition_point(|separator| separator.as_slice() <= key);
        self.children[child_index].get(key)
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
        match self
            .entries
            .binary_search_by(|entry| entry.key.as_slice().cmp(key))
        {
            Ok(index) => Some(self.entries[index].record_id),
            Err(_) => None,
        }
    }
    fn split(&mut self) -> (Vec<u8>, LeafNode) {
        let split_point = self.entries.len() / 2;
        let right_entries = self.entries.split_off(split_point);

        let separator_key = right_entries[0].key.clone();
        let right_leaf = LeafNode {
            entries: right_entries,
        };

        (separator_key, right_leaf)
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

        let Node::Leaf(leaf) = &tree.root else {
            panic!("root should be a leaf");
        };

        let keys: Vec<&[u8]> = leaf
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

        let Node::Leaf(leaf) = &tree.root else {
            panic!("root should be a leaf");
        };

        let keys: Vec<&[u8]> = leaf
            .entries
            .iter()
            .map(|entry| entry.key.as_slice())
            .collect();

        assert_eq!(keys, vec![b"name".as_slice()]);
        assert_eq!(tree.get(b"name"), Some(record_id_2));
    }
    #[test]
    fn leaf_split_divides_entries_in_key_order() {
        let mut leaf = LeafNode::new();

        let record_id = RecordId {
            page_id: 0,
            slot_id: 0,
        };

        leaf.insert(b"name".to_vec(), record_id);
        leaf.insert(b"birth".to_vec(), record_id);
        leaf.insert(b"age".to_vec(), record_id);
        leaf.insert(b"address".to_vec(), record_id);

        let (_separator_key, right) = leaf.split();

        let left_keys: Vec<&[u8]> = leaf
            .entries
            .iter()
            .map(|entry| entry.key.as_slice())
            .collect();

        let right_keys: Vec<&[u8]> = right
            .entries
            .iter()
            .map(|entry| entry.key.as_slice())
            .collect();

        assert_eq!(left_keys, vec![b"address".as_slice(), b"age".as_slice()]);
        assert_eq!(right_keys, vec![b"birth".as_slice(), b"name".as_slice()]);
    }
    #[test]
    fn inserting_past_leaf_capacity_splits_root() {
        let mut tree = BPlusTree::new();

        let address_id = RecordId {
            page_id: 0,
            slot_id: 0,
        };
        let age_id = RecordId {
            page_id: 0,
            slot_id: 1,
        };
        let birth_id = RecordId {
            page_id: 0,
            slot_id: 2,
        };
        let name_id = RecordId {
            page_id: 0,
            slot_id: 3,
        };

        tree.insert(b"name".to_vec(), name_id);
        tree.insert(b"birth".to_vec(), birth_id);
        tree.insert(b"age".to_vec(), age_id);
        tree.insert(b"address".to_vec(), address_id);

        let Node::Internal(root) = &tree.root else {
            panic!("root should become an internal node after the leaf splits");
        };

        let separator_keys: Vec<&[u8]> = root.keys.iter().map(|key| key.as_slice()).collect();
        assert_eq!(separator_keys, vec![b"birth".as_slice()]);
        assert_eq!(root.children.len(), 2);

        let Node::Leaf(left) = &root.children[0] else {
            panic!("left child should be a leaf");
        };
        let Node::Leaf(right) = &root.children[1] else {
            panic!("right child should be a leaf");
        };

        let left_keys: Vec<&[u8]> = left
            .entries
            .iter()
            .map(|entry| entry.key.as_slice())
            .collect();
        let right_keys: Vec<&[u8]> = right
            .entries
            .iter()
            .map(|entry| entry.key.as_slice())
            .collect();

        assert_eq!(left_keys, vec![b"address".as_slice(), b"age".as_slice()]);
        assert_eq!(right_keys, vec![b"birth".as_slice(), b"name".as_slice()]);

        assert_eq!(tree.get(b"address"), Some(address_id));
        assert_eq!(tree.get(b"age"), Some(age_id));
        assert_eq!(tree.get(b"birth"), Some(birth_id));
        assert_eq!(tree.get(b"name"), Some(name_id));
    }
}
