use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

use crate::{Error, Result};

pub const PAGE_SIZE: usize = 4096;
pub type PageId = u64;

pub struct Pager {
    file: File,
}

impl Pager {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;
        Ok(Self { file })
    }

    pub fn write_page(&mut self, page_id: PageId, page: &[u8; PAGE_SIZE]) -> Result<()> {
        let offset = page_id * PAGE_SIZE as u64;
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.write_all(page)?;
        self.file.sync_data()?;
        Ok(())
    }

    pub fn read_page(&mut self, page_id: PageId) -> Result<[u8; PAGE_SIZE]> {
        let file_size = self.file.metadata()?.len();
        let page_count = file_size / PAGE_SIZE as u64;
        if page_id >= page_count {
            return Err(Error::PageOutOfBounds {
                page_id,
                page_count,
            });
        }

        let offset = page_id * PAGE_SIZE as u64;
        self.file.seek(SeekFrom::Start(offset))?;

        let mut page = [0u8; PAGE_SIZE];
        self.file.read_exact(&mut page)?;

        Ok(page)
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
}
