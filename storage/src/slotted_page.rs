use crate::{Error, PAGE_SIZE, Result};

pub type SlotId = u16;

const HEADER_SIZE: usize = 4;
const SLOT_SIZE: usize = 4;

pub struct SlottedPage {
    bytes: [u8; PAGE_SIZE],
}

impl SlottedPage {
    pub fn new() -> Self {
        let mut bytes = [0u8; PAGE_SIZE];
        // slot_count
        bytes[0..2].copy_from_slice(&0u16.to_le_bytes());
        // free_end
        bytes[2..4].copy_from_slice(&(PAGE_SIZE as u16).to_le_bytes());
        Self { bytes }
    }
    pub fn insert(&mut self, record: &[u8]) -> Result<SlotId> {
        let slot_count = u16::from_le_bytes(self.bytes[0..2].try_into().unwrap());
        let free_end = u16::from_le_bytes(self.bytes[2..4].try_into().unwrap()) as usize;
        let slot_id = slot_count;

        let record_start = free_end - record.len();

        self.bytes[record_start..free_end].copy_from_slice(record);

        let slot_start = HEADER_SIZE + slot_id as usize * SLOT_SIZE;

        // 64ビット環境でusize::to_le_bytes()は8バイトを返すが保存先は2バイトしか確保していないのでpanicになる
        let offset = record_start as u16;
        let length = record.len() as u16;
        self.bytes[slot_start..slot_start + 2].copy_from_slice(&offset.to_le_bytes());
        self.bytes[slot_start + 2..slot_start + 4].copy_from_slice(&length.to_le_bytes());

        let slot_count = slot_count + 1;
        let new_free_end = record_start as u16;
        self.bytes[0..2].copy_from_slice(&slot_count.to_le_bytes());
        self.bytes[2..4].copy_from_slice(&new_free_end.to_le_bytes());

        Ok(slot_id)
    }
    pub fn get(&self, slot_id: SlotId) -> Option<&[u8]> {
        let slot_count = u16::from_le_bytes(self.bytes[0..2].try_into().unwrap());
        if slot_id >= slot_count {
            return None;
        }
        let slot_start = HEADER_SIZE + slot_id as usize * SLOT_SIZE;
        let offset =
            u16::from_le_bytes(self.bytes[slot_start..slot_start + 2].try_into().unwrap()) as usize;
        let length = u16::from_le_bytes(
            self.bytes[slot_start + 2..slot_start + 4]
                .try_into()
                .unwrap(),
        ) as usize;
        Some(&self.bytes[offset..offset + length])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserted_record_can_be_retrieved_by_slot_id() {
        let mut page = SlottedPage::new();
        let record = b"hello";

        let slot_id = page.insert(record).unwrap();
        let loaded = page.get(slot_id);

        assert_eq!(slot_id, 0);
        assert_eq!(loaded, Some(&record[..]));
    }
    #[test]
    fn returns_none_for_invalid_slot_id() {
        let mut page = SlottedPage::new();
        let record = b"hello";
        let slot_id = page.insert(record).unwrap();

        let invalid_slot_id = slot_id + 1;
        let loaded = page.get(invalid_slot_id);

        assert_eq!(loaded, None);
    }
}
