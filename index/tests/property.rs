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

#[test]
fn random_operations_match_btree_map() {
    let mut tree = BPlusTree::new();
    let mut model = BTreeMap::new();
    let mut random_state = 0xD3E7E_u64;

    for operation_index in 0..10_000 {
        let operation = next_random(&mut random_state) % 3;
        let key_number = next_random(&mut random_state) % 256;
        let key = key_number.to_be_bytes().to_vec();

        match operation {
            0 => {
                let record_id = RecordId {
                    page_id: operation_index,
                    slot_id: 0,
                };

                tree.insert(key.clone(), record_id);
                model.insert(key, record_id);
            }
            1 => {
                assert_eq!(tree.delete(&key), model.remove(&key).is_some());
            }
            2 => {
                assert_eq!(tree.get(&key), model.get(&key).copied());
            }
            _ => unreachable!(),
        }
    }

    for key_number in 0_u64..256 {
        let key = key_number.to_be_bytes();
        assert_eq!(tree.get(&key), model.get(key.as_slice()).copied());
    }
}
