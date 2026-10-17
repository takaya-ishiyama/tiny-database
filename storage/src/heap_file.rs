use std::path::Path;

use crate::Result;
use crate::pager::Pager;
use crate::slotted_page::SlottedPage;
use crate::{pager::PageId, slotted_page::SlotId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordId {
    pub page_id: PageId,
    pub slot_id: SlotId,
}

pub struct HeapFile {
    pub pager: Pager,
}

impl HeapFile {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let pager = Pager::open(&path)?;
        Ok(Self { pager })
    }
    pub fn insert(&mut self, record: &[u8]) -> Result<RecordId> {
        let page_id = self.pager.allocate_page()?;
        let mut page = SlottedPage::new();
        let slot_id = page.insert(record)?;

        self.pager.write_page(page_id, &page.into_bytes())?;

        Ok(RecordId { page_id, slot_id })
    }
    pub fn get(&mut self, record_id: RecordId) -> Result<Option<Vec<u8>>> {
        let loaded_page_with_byte = self.pager.read_page(record_id.page_id)?;

        let page = SlottedPage::from_bytes(loaded_page_with_byte)?;

        let record = page.get(&record_id.slot_id).map(|bytes| bytes.to_vec());

        Ok(record)
    }
}

#[cfg(test)]
mod tests {
    use tempfile;

    use super::*;

    #[test]
    fn inserted_record_can_be_read_by_record_id() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("database.pages");

        let mut heap_file = HeapFile::open(&path).unwrap();

        let record_id = heap_file.insert(b"hello").unwrap();
        let loaded = heap_file.get(record_id).unwrap();

        assert_eq!(loaded, Some(b"hello".to_vec()));
    }

    #[test]
    fn record_can_be_read_after_reopening_heap_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("database.pages");

        let mut heap_file = HeapFile::open(&path).unwrap();

        let record_id = heap_file.insert(b"hello").unwrap();
        drop(heap_file);

        let mut reopen_heap_file = HeapFile::open(&path).unwrap();
        let loaded = reopen_heap_file.get(record_id).unwrap();

        assert_eq!(loaded, Some(b"hello".to_vec()));
    }
    #[test]
    fn multiple_records_share_page_when_they_fit() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("database.pages");

        let mut heap_file = HeapFile::open(&path).unwrap();

        let record_id_1 = heap_file.insert(b"first").unwrap();
        let record_id_2 = heap_file.insert(b"second").unwrap();

        assert_eq!(
            record_id_1,
            RecordId {
                page_id: 0,
                slot_id: 0
            }
        );
        assert_eq!(
            record_id_2,
            RecordId {
                page_id: 0,
                slot_id: 1
            }
        );

        let loaded_1 = heap_file.get(record_id_1).unwrap();
        let loaded_2 = heap_file.get(record_id_2).unwrap();
        assert_eq!(loaded_1, Some(b"first".to_vec()));
        assert_eq!(loaded_2, Some(b"second".to_vec()));
    }
}
