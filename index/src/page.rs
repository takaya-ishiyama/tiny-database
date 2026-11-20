use storage::pager::{PAGE_SIZE, PageId};

const INDEX_MAGIC: &[u8; 8] = b"BPTREE01";
const INDEX_FORMAT_VERSION: u32 = 1;

const MAGIC_OFFSET: usize = 0;
const VERSION_OFFSET: usize = MAGIC_OFFSET + INDEX_MAGIC.len();
const ROOT_PAGE_ID_OFFSET: usize = VERSION_OFFSET + std::mem::size_of::<u32>();

#[derive(Debug, PartialEq, Eq)]
struct IndexHeaderPage {
    root_page_id: PageId,
}

impl IndexHeaderPage {
    fn new(root_page_id: PageId) -> Self {
        Self { root_page_id }
    }
    fn to_bytes(&self) -> [u8; PAGE_SIZE] {
        let mut bytes = [0u8; PAGE_SIZE];
        bytes[MAGIC_OFFSET..MAGIC_OFFSET + 8].copy_from_slice(INDEX_MAGIC);
        bytes[VERSION_OFFSET..VERSION_OFFSET + 4]
            .copy_from_slice(&INDEX_FORMAT_VERSION.to_le_bytes());
        bytes[ROOT_PAGE_ID_OFFSET..ROOT_PAGE_ID_OFFSET + 8]
            .copy_from_slice(&self.root_page_id.to_le_bytes());
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_header_encodes_root_page_id() {
        let header = IndexHeaderPage::new(42);

        let bytes = header.to_bytes();

        assert_eq!(&bytes[0..8], b"BPTREE01");
        assert_eq!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()), 1);
        assert_eq!(u64::from_le_bytes(bytes[12..20].try_into().unwrap()), 42);
    }
}
