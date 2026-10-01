use std::fs::OpenOptions;
use storage::Database;

#[test]
fn ignores_truncated_final_record() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("database.log");

    {
        let mut database = Database::open(&path).unwrap();
        database.put(b"stable".to_vec(), b"saved".to_vec()).unwrap();
        database
            .put(b"incomplete".to_vec(), b"lost".to_vec())
            .unwrap();
    }

    let file = OpenOptions::new().write(true).open(&path).unwrap();
    let original_len = file.metadata().unwrap().len();
    file.set_len(original_len - 1).unwrap();
    drop(file);

    let recovered = Database::open(&path).unwrap();

    assert_eq!(recovered.get(b"stable"), Some(b"saved".as_slice()));
    assert_eq!(recovered.get(b"incomplete"), None);
}
