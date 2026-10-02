use std::{
    fs::{File, OpenOptions},
    path::Path,
};

use crate::Result;

pub(crate) const PAGE_SIZE: usize = 4096;
pub(crate) type PageId = u64;

pub(crate) struct Pager {
    file: File,
}

impl Pager {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;
        Ok(Self { file })
    }

    pub fn wrige_page(&mut self, page_id: PageId, page: &[u8; PAGE_SIZE]) -> Result<()> {
        todo!()
    }

    pub fn read_page(&mut self, pae_id: PageId) -> Result<[u8; PAGE_SIZE]> {
        todo!()
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
