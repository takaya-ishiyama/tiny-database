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
