use storage::Database;

#[test]
#[ignore = "1万回sync_dataするため、通常テストからは除外"]
fn persists_ten_thousand_records() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("database.log");

    {
        let mut database = Database::open(&path).unwrap();

        for number in 0..10_000 {
            let key = format!("key-{number}").into_bytes();
            let value = format!("value-{number}").into_bytes();
            database.put(key, value).unwrap();
        }
    }
    let reopened = Database::open(&path).unwrap();

    for number in 0..10_000 {
        let key = format!("key-{number}");
        let expected = format!("value-{number}");

        assert_eq!(
            reopened.get(key.as_bytes()),
            Some(expected.as_bytes()),
            "取得に失敗したkey: {key}"
        );
    }
}
