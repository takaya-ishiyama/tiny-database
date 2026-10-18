#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserted_value_can_be_found_by_key() {
        let mut tree = BPlusTree::new();

        tree.insert(b"name".to_vec(), record_id);
        let found = tree.get(b"name");

        assert_eq!(found, Some(record_id));
    }
}
