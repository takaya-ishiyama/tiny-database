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

            // Read and validate the file header
            let mut magic = [0u8; 8];
            file.read_exact(&mut magic)?;
            if magic != *MAGIC {
                return Err(Error::InvalidMagic {
                    expected: *MAGIC,
                    actual: magic,
                });
            }

            // magicを読み込んだ後なので、ファイルのカーソル位置はすでに8バイト進んでいる。次に4バイトを読み込むことで、フォーマットバージョンを取得することができる。
            let mut version_bytes = [0u8; 4];
            file.read_exact(&mut version_bytes)?;
            let version = u32::from_le_bytes(version_bytes);
            if version != FORMAT_VERSION {
                return Err(Error::UnsupportedFormatVersion {
                    expected: FORMAT_VERSION,
                    actual: version,
                });
            }
        }

        Ok(Self { file })
    }

    pub fn write_page(&mut self, page_id: PageId, page: &[u8; PAGE_SIZE]) -> Result<()> {
        let page_count = self.page_count()?;
        if page_id >= page_count {
            return Err(Error::PageOutOfBounds {
                page_id,
                page_count,
            });
        }

        let offset = Self::page_offset(page_id);
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.write_all(page)?;
        self.file.sync_data()?;
        Ok(())
    }

    pub fn read_page(&mut self, page_id: PageId) -> Result<[u8; PAGE_SIZE]> {
        let page_count = self.page_count()?;
        if page_id >= page_count {
            return Err(Error::PageOutOfBounds {
                page_id,
                page_count,
            });
        }

        let offset = Self::page_offset(page_id);
        self.file.seek(SeekFrom::Start(offset))?;

        let mut page = [0u8; PAGE_SIZE];
        self.file.read_exact(&mut page)?;

        Ok(page)
    }

    pub fn allocate_page(&mut self) -> Result<PageId> {
        let next_page_id = self.page_count()?;
        let empty_page = [0u8; PAGE_SIZE];
        let offset = Self::page_offset(next_page_id);
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.write_all(&empty_page)?;
        self.file.sync_data()?;
        Ok(next_page_id)
    }
    fn page_count(&self) -> Result<u64> {
        let file_size = self.file.metadata()?.len();
        Ok((file_size - FILE_HEADER_SIZE) / PAGE_SIZE as u64)
    }
    fn page_offset(page_id: PageId) -> u64 {
        FILE_HEADER_SIZE + page_id * PAGE_SIZE as u64
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

        let page_id = pager.allocate_page().unwrap();
        pager.write_page(page_id, &page).unwrap();
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

        let first_page_id = pager.allocate_page().unwrap();
        let second_page_id = pager.allocate_page().unwrap();
        pager.write_page(first_page_id, &first_page).unwrap();
        pager.write_page(second_page_id, &second_page).unwrap();
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
    #[test]
    fn rejects_file_with_invalid_magic() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("database.log");
        let invalid_magic: &[u8; 8] = b"INVALID!";
        let mut file = File::create(&path).unwrap();
        file.write_all(invalid_magic).unwrap();
        file.write_all(&FORMAT_VERSION.to_le_bytes()).unwrap();
        drop(file);

        let result = Pager::open(&path);

        assert!(matches!(
            result,
            Err(Error::InvalidMagic {
                expected: [b'T', b'I', b'N', b'Y', b'D', b'B', b'0', b'1'],
                actual: [b'I', b'N', b'V', b'A', b'L', b'I', b'D', b'!']
            })
        ));
    }
    #[test]
    fn rejects_file_with_unsupported_version() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("database.log");
        let invalid_version: u32 = 999;
        let mut file = File::create(&path).unwrap();
        file.write_all(MAGIC).unwrap();
        file.write_all(&invalid_version.to_le_bytes()).unwrap();
        drop(file);

        let result = Pager::open(&path);

        assert!(matches!(
            result,
            Err(Error::UnsupportedFormatVersion {
                expected: 1,
                actual: 999
            })
        ));
    }
    #[test]
    fn writing_unallocated_page_fails() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("database.pages");

        let mut pager = Pager::open(&path).unwrap();
        let page = [1u8; PAGE_SIZE];

        let result = pager.write_page(0, &page);

        assert!(matches!(
            result,
            Err(Error::PageOutOfBounds {
                page_id: 0,
                page_count: 0,
            })
        ));
    }
}
