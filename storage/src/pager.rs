use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

use crate::{Error, Result};

pub const PAGE_SIZE: usize = 4096;
pub type PageId = u64;

pub const MAGIC: &[u8; 8] = b"TINYDB01";
pub const FORMAT_VERSION: u32 = 1;
pub const FILE_HEADER_SIZE: u64 = 12;

pub struct Pager {
    file: File,
}

impl Pager {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;

        let file_size = file.metadata()?.len();

        if file_size == 0 {
            // Write the file header
            file.write_all(MAGIC)?;
            file.write_all(&FORMAT_VERSION.to_le_bytes())?;
            file.sync_data()?;
        } else if file_size < FILE_HEADER_SIZE {
            return Err(Error::InvalidPageFileSize {
                actual: file_size,
                page_size: FILE_HEADER_SIZE,
            });
        } else {
            // ページ領域のサイズがPAGE_SIZEの倍数であることを確認する
            if !(file_size - FILE_HEADER_SIZE).is_multiple_of(PAGE_SIZE as u64) {
                return Err(Error::InvalidPageFileSize {
                    actual: file_size,
                    page_size: PAGE_SIZE as u64,
                });
            }
        }

        Ok(Self { file })
    }

    pub fn write_page(&mut self, page_id: PageId, page: &[u8; PAGE_SIZE]) -> Result<()> {
        let offset = FILE_HEADER_SIZE + page_id * PAGE_SIZE as u64;
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.write_all(page)?;
        self.file.sync_data()?;
        Ok(())
    }

    pub fn read_page(&mut self, page_id: PageId) -> Result<[u8; PAGE_SIZE]> {
        let file_size = self.file.metadata()?.len();
        let page_count = (file_size - FILE_HEADER_SIZE) / PAGE_SIZE as u64;
        if page_id >= page_count {
            return Err(Error::PageOutOfBounds {
                page_id,
                page_count,
            });
        }

        let offset = FILE_HEADER_SIZE + page_id * PAGE_SIZE as u64;
        self.file.seek(SeekFrom::Start(offset))?;

        let mut page = [0u8; PAGE_SIZE];
        self.file.read_exact(&mut page)?;

        Ok(page)
    }

    pub fn allocate_page(&mut self) -> Result<PageId> {
        let file_size = self.file.metadata()?.len();
        let next_page_id = (file_size - FILE_HEADER_SIZE) / PAGE_SIZE as u64;
        let empty_page = [0u8; PAGE_SIZE];
        self.write_page(next_page_id, &empty_page)?;
        Ok(next_page_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;

    #[test]
    fn page_round_trips() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("database.log");

        let mut pager = Pager::open(&path).unwrap();

        let page = [42u8; PAGE_SIZE];

        pager.write_page(0, &page).unwrap();
        let loaded = pager.read_page(0).unwrap();

        assert_eq!(loaded, page);
    }

    #[test]
    fn different_pages_do_not_overwrite_each_other() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("database.log");

        let mut pager = Pager::open(&path).unwrap();
        let first_page = [1u8; PAGE_SIZE];
        let second_page = [2u8; PAGE_SIZE];

        pager.write_page(0, &first_page).unwrap();
        pager.write_page(1, &second_page).unwrap();
        let loaded_first = pager.read_page(0).unwrap();
        let loaded_second = pager.read_page(1).unwrap();

        assert_eq!(loaded_first, first_page);
        assert_eq!(loaded_second, second_page);
    }

    #[test]
    fn reading_nonexistent_page_fails() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("database.log");

        let mut pager = Pager::open(&path).unwrap();

        let not_exists_page = pager.read_page(0);

        assert!(matches!(
            not_exists_page,
            Err(Error::PageOutOfBounds {
                page_id: 0,
                page_count: 0
            })
        ));
    }

    #[test]
    fn allocates_pages_sequentially() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("database.log");

        let mut pager = Pager::open(&path).unwrap();

        let first_page_id = pager.allocate_page().unwrap();
        let second_page_id = pager.allocate_page().unwrap();

        assert_eq!(first_page_id, 0);
        assert_eq!(second_page_id, 1);

        // 割り当てたページが実際に存在し、空ページとして読み込めることを確認する
        let empty_page = [0u8; PAGE_SIZE];
        let loaded_first = pager.read_page(first_page_id).unwrap();
        let loaded_second = pager.read_page(second_page_id).unwrap();

        assert_eq!(loaded_first, empty_page);
        assert_eq!(loaded_second, empty_page);
    }
    #[test]
    fn rejects_file_with_partial_page() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("database.log");

        let partial_page = vec![0u8; PAGE_SIZE - 1];
        std::fs::write(&path, partial_page).unwrap();

        let result = Pager::open(&path);
        assert!(matches!(
            result,
            Err(Error::InvalidPageFileSize {
                actual: 4095,
                page_size: 4096
            })
        ))
    }
    #[test]
    fn page_survives_reopening_pager() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("database.log");

        let page = [1u8; PAGE_SIZE];
        {
            let mut pager = Pager::open(&path).unwrap();
            let page_id = pager.allocate_page().unwrap();
            assert_eq!(page_id, 0);

            pager.write_page(page_id, &page).unwrap();
        }
        let mut reopened = Pager::open(&path).unwrap();
        let loaded = reopened.read_page(0).unwrap();
        assert_eq!(loaded, page);
    }
    #[test]
    fn new_file_contains_header() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("database.log");
        {
            let _pager = Pager::open(&path).unwrap();
        }
        let bytes = std::fs::read(&path).unwrap();

        let mut expected = Vec::new();
        expected.extend_from_slice(MAGIC);
        expected.extend_from_slice(&FORMAT_VERSION.to_le_bytes());

        assert_eq!(bytes, expected);
    }
}
