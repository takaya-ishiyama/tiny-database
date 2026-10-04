use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

use crate::Result;

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
}
