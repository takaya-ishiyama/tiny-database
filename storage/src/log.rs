use std::io::{Read, Seek, SeekFrom, Write};
use std::{
    fs::{File, OpenOptions},
    path::Path,
};

use crate::{Result, record::Record};

pub(crate) struct Log {
    file: File,
}

impl Log {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(path)?;
        Ok(Self { file })
    }
    pub fn append(&mut self, record: &Record) -> Result<()> {
        let bytes = record.encode()?;

        self.file.write_all(&bytes)?;
        self.file.sync_data()?;

        Ok(())
    }

    pub fn scan(&mut self) -> Result<Vec<Record>> {
        self.file.seek(SeekFrom::Start(0))?;

        let mut bytes = Vec::new();
        self.file.read_to_end(&mut bytes)?;

        let mut records = Vec::new();
        let mut offset = 0;

        while offset < bytes.len() {
            let record_len = Record::encoded_len(&bytes[offset..])?;

            let record_end = offset + record_len;

            let record = Record::decode(&bytes[offset..record_end])?;
            records.push(record);

            offset = record_end;
        }

        Ok(records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn appends_encoded_record_to_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("database.log");

        let record = Record::new(b"name".to_vec(), b"Taro".to_vec(), false);

        let expected = record.encode().unwrap();

        let mut log = Log::open(&path).unwrap();
        log.append(&record).unwrap();
        drop(log);

        let actual = fs::read(&path).unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn appends_multiple_records_in_order() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("database.log");

        let first = Record::new(b"name".to_vec(), b"Taro".to_vec(), false);

        let second = Record::new(b"city".to_vec(), b"Tokyo".to_vec(), false);

        let mut expected = first.encode().unwrap();
        expected.extend_from_slice(&second.encode().unwrap());

        let mut log = Log::open(&path).unwrap();
        log.append(&first).unwrap();
        log.append(&second).unwrap();
        drop(log);

        let actual = fs::read(&path).unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn scans_multiple_records_in_order() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("database.log");

        let first = Record::new(b"name".to_vec(), b"Taro".to_vec(), false);
        let second = Record::new(b"city".to_vec(), b"Tokyo".to_vec(), false);

        let mut log = Log::open(&path).unwrap();
        log.append(&first).unwrap();
        log.append(&second).unwrap();

        let records = log.scan().unwrap();

        assert_eq!(records, vec![first, second]);
    }

    #[test]
    fn scans_records_after_reopening_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("database.log");

        let first = Record::new(b"name".to_vec(), b"Taro".to_vec(), false);

        let second = Record::new(b"city".to_vec(), b"Tokyo".to_vec(), false);

        {
            let mut log = Log::open(&path).unwrap();
            log.append(&first).unwrap();
            log.append(&second).unwrap();
        } // ここでlogがdropされ、ファイルが閉じられる

        let mut reopened_log = Log::open(&path).unwrap();
        let records = reopened_log.scan().unwrap();

        assert_eq!(records, vec![first, second]);
    }
}
