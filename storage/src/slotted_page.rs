use crate::{Error, PAGE_SIZE, Result};
pub type SlotId = u16;

const HEADER_SIZE: usize = 4;
const SLOT_SIZE: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
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

        let new_slot_end = HEADER_SIZE + (slot_count as usize + 1) * SLOT_SIZE;

        let Some(record_start) = free_end.checked_sub(record.len()) else {
            return Err(Error::PageFull);
        };

        if record_start < new_slot_end {
            return Err(Error::PageFull);
        }

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
    pub fn get(&self, slot_id: &SlotId) -> Option<&[u8]> {
        if self.is_invalid_slot_id(slot_id) {
            return None;
        }
        let slot_start = HEADER_SIZE + *slot_id as usize * SLOT_SIZE;
        let offset =
            u16::from_le_bytes(self.bytes[slot_start..slot_start + 2].try_into().unwrap()) as usize;
        let length = u16::from_le_bytes(
            self.bytes[slot_start + 2..slot_start + 4]
                .try_into()
                .unwrap(),
        ) as usize;
        if offset == 0 && length == 0 {
            return None;
        }
        Some(&self.bytes[offset..offset + length])
    }
    pub fn into_bytes(self) -> [u8; PAGE_SIZE] {
        self.bytes
    }
    pub fn from_bytes(bytes: [u8; PAGE_SIZE]) -> Result<Self> {
        let slot_count = u16::from_le_bytes(bytes[0..2].try_into().unwrap()) as usize;
        let free_end = u16::from_le_bytes(bytes[2..4].try_into().unwrap()) as usize;

        let slot_directory_end = HEADER_SIZE + slot_count * SLOT_SIZE;

        if PAGE_SIZE < slot_directory_end {
            return Err(Error::CorruptedPage("slot count exceeds page boundary"));
        }
        if free_end > PAGE_SIZE {
            return Err(Error::CorruptedPage("free_end exceeds page boundary"));
        }
        if slot_directory_end > free_end {
            return Err(Error::CorruptedPage("slot directory overlaps record area"));
        }

        for slot_id in 0..slot_count {
            let slot_start = HEADER_SIZE + slot_id * SLOT_SIZE;
            let offset =
                u16::from_le_bytes(bytes[slot_start..slot_start + 2].try_into().unwrap()) as usize;
            let length =
                u16::from_le_bytes(bytes[slot_start + 2..slot_start + 4].try_into().unwrap())
                    as usize;

            if offset == 0 && length == 0 {
                continue;
            }
            if offset < free_end {
                return Err(Error::CorruptedPage("record starts before free_end"));
            }

            let Some(record_end) = offset.checked_add(length) else {
                return Err(Error::CorruptedPage("record offset overflow"));
            };
            if record_end > PAGE_SIZE {
                return Err(Error::CorruptedPage("record exceeds page boundary"));
            }
        }

        Ok(Self { bytes })
    }
    pub fn delete(&mut self, slot_id: &SlotId) -> bool {
        if self.is_invalid_slot_id(slot_id) {
            return false;
        }
        let slot_start = HEADER_SIZE + *slot_id as usize * SLOT_SIZE;
        /* 削除したレコードのRIDを仕様させないためslotの4バイトを0埋めしている
        slotの4バイトは永続的に残ってしまう問題が残っている
        レコード本体はcompactで消している
        */
        self.bytes[slot_start..slot_start + SLOT_SIZE].fill(0);
        true
    }
    pub fn compact(&mut self) -> Result<()> {
        let slot_count = u16::from_le_bytes(self.bytes[0..2].try_into().unwrap());
        let mut free_end = PAGE_SIZE;
        let mut new_page = [0u8; PAGE_SIZE];

        for i in 0..slot_count {
            let slot_start = HEADER_SIZE + i as usize * SLOT_SIZE;
            let offset =
                u16::from_le_bytes(self.bytes[slot_start..slot_start + 2].try_into().unwrap())
                    as usize;
            let length = u16::from_le_bytes(
                self.bytes[slot_start + 2..slot_start + 4]
                    .try_into()
                    .unwrap(),
            ) as usize;

            if offset == 0 && length == 0 {
                continue;
            }

            let record = &self.bytes[offset..offset + length];
            let Some(record_start) = free_end.checked_sub(record.len()) else {
                return Err(Error::PageFull);
            };

            let new_offset = record_start as u16;
            let new_length = length as u16;

            new_page[record_start..free_end].copy_from_slice(record);

            new_page[slot_start..slot_start + 2].copy_from_slice(&new_offset.to_le_bytes());
            new_page[slot_start + 2..slot_start + 4].copy_from_slice(&new_length.to_le_bytes());

            free_end = record_start;
        }

        new_page[0..2].copy_from_slice(&slot_count.to_le_bytes());
        new_page[2..4].copy_from_slice(&(free_end as u16).to_le_bytes());

        self.bytes = new_page;
        Ok(())
    }

    fn is_invalid_slot_id(&self, slot_id: &SlotId) -> bool {
        let slot_count = u16::from_le_bytes(self.bytes[0..2].try_into().unwrap());
        *slot_id >= slot_count
    }
}

impl Default for SlottedPage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::pager::Pager;

    use super::*;

    #[test]
    fn inserted_record_can_be_retrieved_by_slot_id() {
        let mut page = SlottedPage::new();
        let record = b"hello";

        let slot_id = page.insert(record).unwrap();
        let loaded = page.get(&slot_id);

        assert_eq!(slot_id, 0);
        assert_eq!(loaded, Some(&record[..]));
    }
    #[test]
    fn returns_none_for_invalid_slot_id() {
        let mut page = SlottedPage::new();
        let record = b"hello";
        let slot_id = page.insert(record).unwrap();

        let invalid_slot_id = slot_id + 1;
        let loaded = page.get(&invalid_slot_id);

        assert_eq!(loaded, None);
    }
    #[test]
    fn multiple_records_can_be_retrieved_by_their_slot_ids() {
        let mut page = SlottedPage::new();
        let record_1 = b"first";
        let record_2 = b"second";
        let slot_id_1 = page.insert(record_1).unwrap();
        let slot_id_2 = page.insert(record_2).unwrap();

        let loaded_1 = page.get(&slot_id_1).unwrap();
        let loaded_2 = page.get(&slot_id_2).unwrap();

        assert_eq!(slot_id_1, 0);
        assert_eq!(slot_id_2, 1);
        assert_eq!(loaded_1, record_1);
        assert_eq!(loaded_2, record_2);
    }
    #[test]
    fn returns_error_when_record_does_not_fit() {
        let mut page = SlottedPage::new();
        let record = vec![0u8; PAGE_SIZE];
        let result = page.insert(&record);
        assert!(matches!(result, Err(Error::PageFull)))
    }
    #[test]
    fn largest_record_that_fits_can_be_inserted() {
        let mut page = SlottedPage::new();
        let largest = PAGE_SIZE - HEADER_SIZE - SLOT_SIZE; // 4088
        let record = vec![0u8; largest];
        let slot_id = page.insert(&record).unwrap();
        let loaded = page.get(&slot_id);

        assert_eq!(slot_id, 0);
        assert_eq!(loaded, Some(&record[..]));
    }
    #[test]
    fn record_one_byte_too_large_returns_page_full() {
        let mut page = SlottedPage::new();
        let largest = PAGE_SIZE - HEADER_SIZE - SLOT_SIZE; // 4088
        let too_large = largest + 1; // 4089
        let record = vec![0u8; too_large];
        let result = page.insert(&record);
        assert!(matches!(result, Err(Error::PageFull)))
    }
    #[test]
    fn returns_page_full_when_multiple_records_fill_the_page() {
        let mut page = SlottedPage::new();
        for _ in 0..PAGE_SIZE {
            let record = [0u8];
            let result = page.insert(&record);
            if let Err(Error::PageFull) = result {
                assert!(result.is_err());
                return;
            }
        }
        panic!("its over")
    }
    #[test]
    fn page_survives_round_trip_through_bytes() {
        let mut page = SlottedPage::new();

        let record_1 = b"first";
        let record_2 = b"second";

        let slot_id_1 = page.insert(record_1).unwrap();
        let slot_id_2 = page.insert(record_2).unwrap();

        let bytes = page.into_bytes();

        let restored = SlottedPage::from_bytes(bytes).unwrap();

        assert_eq!(restored.get(&slot_id_1), Some(&record_1[..]));
        assert_eq!(restored.get(&slot_id_2), Some(&record_2[..]));
    }
    #[test]
    fn slotted_page_survives_pager_round_trip() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("database.pages");
        let mut pager = Pager::open(&path).unwrap();
        let page_id = pager.allocate_page().unwrap();

        let mut page = SlottedPage::new();
        let record = b"hello";
        let slot_id = page.insert(record).unwrap();

        let bytes = page.into_bytes();

        pager.write_page(page_id, &bytes).unwrap();
        drop(pager);
        let mut pager = Pager::open(&path).unwrap();

        let bytes = pager.read_page(page_id).unwrap();

        let restored = SlottedPage::from_bytes(bytes).unwrap();
        let restored_record = restored.get(&slot_id);

        assert_eq!(Some(&record[..]), restored_record);
    }
    #[test]
    fn deleted_record_cannot_be_retrieved() {
        let mut page = SlottedPage::new();
        let slot_id = page.insert(b"hello").unwrap();

        assert!(page.delete(&slot_id));
        assert_eq!(page.get(&slot_id), None);
    }
    #[test]
    fn deleting_one_record_does_not_affect_other_records() {
        let mut page = SlottedPage::new();

        let record_1 = b"first";
        let record_2 = b"second";

        let slot_id_1 = page.insert(record_1).unwrap();
        let slot_id_2 = page.insert(record_2).unwrap();

        assert!(page.delete(&slot_id_1));

        assert_eq!(page.get(&slot_id_1), None);
        assert_eq!(page.get(&slot_id_2), Some(&record_2[..]))
    }
    #[test]
    fn deleting_invalid_slot_id_returns_false() {
        let mut page = SlottedPage::new();

        let record = b"hello";

        let slot_id = page.insert(record).unwrap();

        let result = page.delete(&(slot_id + 1));

        assert!(!result)
    }
    #[test]
    fn compact_preserves_existing_records() {
        let mut page = SlottedPage::new();

        let first = page.insert(b"first").unwrap();
        let second = page.insert(b"second").unwrap();
        let third = page.insert(b"third").unwrap();

        assert!(page.delete(&second));

        page.compact().unwrap();

        assert_eq!(page.get(&first), Some(&b"first"[..]));
        assert_eq!(page.get(&second), None);
        assert_eq!(page.get(&third), Some(&b"third"[..]));
    }
    #[test]
    fn compact_reclaims_space_from_deleted_records() {
        let mut page = SlottedPage::new();
        // ヘッダー4バイト、slotが3件の12バイトを足して、全部で4096バイトになる
        let record_1 = vec![1u8; 1000];
        let record_2 = vec![2u8; 1000];
        let record_3 = vec![3u8; 2080];

        let slot_id_1 = page.insert(&record_1).unwrap();
        let slot_id_2 = page.insert(&record_2).unwrap();
        let slot_id_3 = page.insert(&record_3).unwrap();

        assert!(page.delete(&slot_id_2));

        assert!(matches!(page.insert(&[9]), Err(Error::PageFull)));

        page.compact().unwrap();

        let new_record = vec![4u8; 900];
        let new_slot = page.insert(&new_record).unwrap();

        assert_eq!(page.get(&slot_id_1), Some(&record_1[..]));
        assert_eq!(page.get(&slot_id_2), None);
        assert_eq!(page.get(&slot_id_3), Some(&record_3[..]));
        assert_eq!(page.get(&new_slot), Some(&new_record[..]));
    }
    #[test]
    fn from_bytes_rejects_free_end_past_page_boundary() {
        let page = SlottedPage::new();
        let mut bytes = page.bytes;
        bytes[2..4].copy_from_slice(&4097_u16.to_le_bytes());
        assert!(matches!(
            SlottedPage::from_bytes(bytes),
            Err(Error::CorruptedPage(..))
        ));
    }
    #[test]
    fn from_bytes_rejects_slot_directory_past_page_boundary() {
        let page = SlottedPage::new();
        let mut bytes = page.bytes;
        // slot directoryがページ境界値を超える件数
        let invalid_slot_count = 2000;
        bytes[0..2].copy_from_slice(&(invalid_slot_count as u16).to_le_bytes());
        assert!(matches!(
            SlottedPage::from_bytes(bytes),
            Err(Error::CorruptedPage(..))
        ));
    }
    #[test]
    fn from_bytes_rejects_slot_directory_overlapping_record_area() {
        let page = SlottedPage::new();
        let mut bytes = page.into_bytes();

        bytes[0..2].copy_from_slice(&1_u16.to_le_bytes());
        bytes[2..4].copy_from_slice(&4_u16.to_le_bytes());

        assert!(matches!(
            SlottedPage::from_bytes(bytes),
            Err(Error::CorruptedPage(..))
        ));
    }
    #[test]
    fn from_bytes_rejects_record_past_page_boundary() {
        let page = SlottedPage::new();
        let mut bytes = page.into_bytes();

        bytes[0..2].copy_from_slice(&1_u16.to_le_bytes());
        bytes[2..4].copy_from_slice(&4090_u16.to_le_bytes());
        bytes[4..6].copy_from_slice(&4090_u16.to_le_bytes());
        bytes[6..8].copy_from_slice(&10_u16.to_le_bytes());

        assert!(matches!(
            SlottedPage::from_bytes(bytes),
            Err(Error::CorruptedPage(..))
        ));
    }
    #[test]
    fn from_bytes_rejects_record_before_free_end() {
        let page = SlottedPage::new();
        let mut bytes = page.into_bytes();

        bytes[0..2].copy_from_slice(&1_u16.to_le_bytes());
        bytes[2..4].copy_from_slice(&100_u16.to_le_bytes());
        bytes[4..6].copy_from_slice(&50_u16.to_le_bytes());
        bytes[6..8].copy_from_slice(&10_u16.to_le_bytes());

        assert!(matches!(
            SlottedPage::from_bytes(bytes),
            Err(Error::CorruptedPage(..))
        ));
    }
}
