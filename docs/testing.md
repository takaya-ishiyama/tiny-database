## テストの書き方
- 単体テストは対象module内の `#[cfg(test)]` に置く
- 公開APIを使うプロパティテストは各crateの `tests/property.rs` に置く
- 再起動、ファイル切断、I/O障害を扱うテストは`storage/tests/crash_recovery.rs` に置く
