use std::mem::size_of;
use storage::{
    Error, Result,
    pager::{PAGE_SIZE, PageId},
};

const INDEX_MAGIC: &[u8; 8] = b"BPTREE01";
const INDEX_FORMAT_VERSION: u32 = 1;

const MAGIC_OFFSET: usize = 0;
const VERSION_OFFSET: usize = MAGIC_OFFSET + INDEX_MAGIC.len();
const ROOT_PAGE_ID_OFFSET: usize = VERSION_OFFSET + std::mem::size_of::<u32>();

const NODE_TYPE_OFFSET: usize = 0;
const ENTRY_COUNT_OFFSET: usize = 1;
const NEXT_LEAF_PAGE_ID_OFFSET: usize = ENTRY_COUNT_OFFSET + size_of::<u16>();

const LEAF_NODE_TYPE: u8 = 1;
const NO_NEXT_LEAF_PAGE: PageId = PageId::MAX;

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
    fn from_bytes(bytes: &[u8; PAGE_SIZE]) -> Result<Self> {
        let actual_magic: [u8; 8] = bytes[MAGIC_OFFSET..MAGIC_OFFSET + INDEX_MAGIC.len()]
            .try_into()
            .unwrap();
        if actual_magic != *INDEX_MAGIC {
            return Err(Error::InvalidMagic {
                expected: *INDEX_MAGIC,
                actual: actual_magic,
            });
        }
        let version = u32::from_le_bytes(
            bytes[VERSION_OFFSET..VERSION_OFFSET + size_of::<u32>()]
                .try_into()
                .unwrap(),
        );
        if version != INDEX_FORMAT_VERSION {
            return Err(Error::UnsupportedFormatVersion {
                expected: INDEX_FORMAT_VERSION,
                actual: version,
            });
        }
        let root_page_id = PageId::from_le_bytes(
            bytes[ROOT_PAGE_ID_OFFSET..ROOT_PAGE_ID_OFFSET + size_of::<PageId>()]
                .try_into()
                .unwrap(),
        );
        Ok(Self { root_page_id })
    }
}

#[derive(Debug, PartialEq, Eq)]
struct LeafNodePage {
    next_leaf_page_id: Option<PageId>,
}

impl LeafNodePage {
    fn empty() -> Self {
        Self {
            next_leaf_page_id: None,
        }
    }

    fn to_bytes(&self) -> [u8; PAGE_SIZE] {
        let mut bytes = [0u8; PAGE_SIZE];

        bytes[NODE_TYPE_OFFSET] = LEAF_NODE_TYPE;

        let entry_count = 0u16;
        bytes[ENTRY_COUNT_OFFSET..ENTRY_COUNT_OFFSET + size_of::<u16>()]
            .copy_from_slice(&entry_count.to_le_bytes());

        let next_leaf_page_id = self.next_leaf_page_id.unwrap_or(NO_NEXT_LEAF_PAGE);

        bytes[NEXT_LEAF_PAGE_ID_OFFSET..NEXT_LEAF_PAGE_ID_OFFSET + size_of::<PageId>()]
            .copy_from_slice(&next_leaf_page_id.to_le_bytes());

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

    #[test]
    fn index_header_survives_page_round_trip() {
        let original = IndexHeaderPage::new(42);

        let bytes = original.to_bytes();
        let decoded = IndexHeaderPage::from_bytes(&bytes).unwrap();

        assert_eq!(decoded, original);
    }
    #[test]
    fn index_header_rejects_invalid_magic() {
        let header = IndexHeaderPage::new(42);
        let mut bytes = header.to_bytes();

        bytes[MAGIC_OFFSET] = b'X';

        let result = IndexHeaderPage::from_bytes(&bytes);

        assert!(matches!(
            result,
            Err(Error::InvalidMagic { expected, actual })
                if expected == *INDEX_MAGIC
                    && actual == *b"XPTREE01"
        ));
    }
    #[test]
    fn index_header_rejects_unsupported_format_version() {
        let header = IndexHeaderPage::new(42);
        let mut bytes = header.to_bytes();

        let unsupported_version = 2u32;

        bytes[VERSION_OFFSET..VERSION_OFFSET + size_of::<u32>()]
            .copy_from_slice(&unsupported_version.to_le_bytes());

        let result = IndexHeaderPage::from_bytes(&bytes);

        assert!(matches!(
            result,
            Err(Error::UnsupportedFormatVersion { expected, actual })
                if expected == INDEX_FORMAT_VERSION
                    && actual == unsupported_version
        ));
    }
    #[test]
    fn index_header_survives_pager_round_trip() {
        use storage::pager::Pager;

        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("index.db");

        let mut pager = Pager::open(&path).unwrap();

        let header_page_id = pager.allocate_page().unwrap();
        assert_eq!(header_page_id, 0);

        let root_page_id = pager.allocate_page().unwrap();
        assert_eq!(root_page_id, 1);

        let header = IndexHeaderPage::new(root_page_id);
        pager
            .write_page(header_page_id, &header.to_bytes())
            .unwrap();

        drop(pager);

        let mut reopened_pager = Pager::open(&path).unwrap();
        let bytes = reopened_pager.read_page(header_page_id).unwrap();
        let decoded = IndexHeaderPage::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.root_page_id, root_page_id);
    }

    #[test]
    fn empty_leaf_page_encodes_header() {
        let leaf = LeafNodePage::empty();

        let bytes = leaf.to_bytes();

        assert_eq!(bytes[NODE_TYPE_OFFSET], LEAF_NODE_TYPE);

        let entry_count = u16::from_le_bytes(
            bytes[ENTRY_COUNT_OFFSET..ENTRY_COUNT_OFFSET + size_of::<u16>()]
                .try_into()
                .unwrap(),
        );
        assert_eq!(entry_count, 0);

        let next_leaf_page_id = PageId::from_le_bytes(
            bytes[NEXT_LEAF_PAGE_ID_OFFSET..NEXT_LEAF_PAGE_ID_OFFSET + size_of::<PageId>()]
                .try_into()
                .unwrap(),
        );
        assert_eq!(next_leaf_page_id, NO_NEXT_LEAF_PAGE);
    }
}
