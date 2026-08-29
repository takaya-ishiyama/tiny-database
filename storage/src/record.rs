use crate::{Error, Result};

pub(crate) struct Record {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    pub tombstone: bool,
}

impl Record {
    pub fn new(key: Vec<u8>, value: Vec<u8>, tombstone: bool) -> Self {
        Self {
            key,
            value,
            tombstone,
        }
    }

    // pub fn encord(&self) -> Result<Vec<u8>, Error>{
    //
    // };
    // pub fn decode(bytes: &[u8]) -> Result<Self, Error>{
    //
    // };
}
