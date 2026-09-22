use std::io::Write;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempfile;

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
}
