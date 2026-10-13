use storage::{
    Error,
    slotted_page::{SlotId, SlottedPage},
};

fn next_random(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1);
    *state
}

#[test]
fn random_operations_match_reference_model() {
    let mut page = SlottedPage::new();
    let mut model: Vec<Option<Vec<u8>>> = Vec::new();

    // A fixed seed makes a failing operation sequence reproducible.
    let mut random_state = 0x5EED_u64;
    let mut insert_count = 0;
    let mut delete_count = 0;
    let mut compact_count = 0;

    for _ in 0..2_000 {
        match (next_random(&mut random_state) >> 32) % 4 {
            0 => {
                let length = ((next_random(&mut random_state) >> 32) % 64 + 1) as usize;
                let record: Vec<u8> = (0..length)
                    .map(|_| (next_random(&mut random_state) >> 32) as u8)
                    .collect();

                match page.insert(&record) {
                    Ok(slot_id) => {
                        assert_eq!(slot_id as usize, model.len());
                        model.push(Some(record));
                        insert_count += 1;
                    }
                    Err(Error::PageFull) => {}
                    Err(error) => panic!("unexpected insert error: {error}"),
                }
            }
            1 => {
                if !model.is_empty() {
                    let index = (next_random(&mut random_state) >> 32) as usize % model.len();
                    let slot_id = index as SlotId;

                    page.delete(&slot_id);
                    model[index] = None;
                    delete_count += 1;
                }
            }
            2 => {
                page.compact().unwrap();
                compact_count += 1;
            }
            _ => {
                let index = if model.is_empty() {
                    0
                } else {
                    (next_random(&mut random_state) >> 32) as usize % (model.len() + 1)
                };
                let slot_id = index as SlotId;
                let expected = model.get(index).and_then(|record| record.as_deref());

                assert_eq!(page.get(&slot_id), expected);
            }
        }

        for (index, expected) in model.iter().enumerate() {
            let slot_id = index as SlotId;
            assert_eq!(
                page.get(&slot_id),
                expected.as_deref(),
                "slot {slot_id} differs from the reference model"
            );
        }
    }

    assert!(insert_count > 0);
    assert!(delete_count > 0);
    assert!(compact_count > 0);
}
