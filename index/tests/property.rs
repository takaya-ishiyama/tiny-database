use std::collections::BTreeMap;

use index::BPlusTree;
use storage::heap_file::RecordId;

fn next_random(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1);
    *state
}

#[test]
fn random_insertions_match_btree_map() {
    let mut tree = BPlusTree::new();
    let mut model = BTreeMap::new();
    let mut random_state = 0x5EED_u64;

    for insertion_index in 0..1_000 {
        let key = next_random(&mut random_state).to_be_bytes().to_vec();
        let record_id = RecordId {
            page_id: insertion_index,
            slot_id: 0,
        };

        tree.insert(key.clone(), record_id);
        model.insert(key, record_id);
    }

    for (key, expected_record_id) in model {
        assert_eq!(tree.get(&key), Some(expected_record_id));
    }
}

#[test]
fn random_ranges_match_btree_map() {
    let mut tree = BPlusTree::new();
    let mut model = BTreeMap::new();
    let mut random_state = 0x5EED_u64;

    for insertion_index in 0..1_000 {
        let key = next_random(&mut random_state).to_be_bytes().to_vec();
        let record_id = RecordId {
            page_id: insertion_index,
            slot_id: 0,
        };

        tree.insert(key.clone(), record_id);
        model.insert(key, record_id);
    }

    for _ in 0..100 {
        let first = next_random(&mut random_state);
        let second = next_random(&mut random_state);
        let start = first.min(second).to_be_bytes().to_vec();
        let end = first.max(second).to_be_bytes().to_vec();

        let expected: Vec<(Vec<u8>, RecordId)> = model
            .range(start.clone()..end.clone())
            .map(|(key, record_id)| (key.clone(), *record_id))
            .collect();

        assert_eq!(tree.range(&start, &end), expected);
    }
}
