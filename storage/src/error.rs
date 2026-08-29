use std::{error, fmt, io};

/// An error produced while validating, encoding, decoding, or persisting data.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// An operation on the underlying storage failed.
    Io(io::Error),
    /// A key exceeded the configured on-disk limit.
    KeyTooLarge { actual: usize, max: usize },
    /// A value exceeded the configured on-disk limit.
    ValueTooLarge { actual: usize, max: usize },
    /// The input ended before a complete record could be read.
    TruncatedRecord { expected: usize, actual: usize },
    /// A record contains a tombstone byte other than 0 or 1.
    InvalidTombstone(u8),
    /// The stored checksum does not match the record contents.
    ChecksumMismatch { expected: u32, actual: u32 },
    /// Record metadata is invalid even though all required bytes are present.
    CorruptedRecord(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "storage I/O error: {error}"),
            Self::KeyTooLarge { actual, max } => {
                write!(f, "key is too large: {actual} bytes (maximum: {max} bytes)")
            }
            Self::ValueTooLarge { actual, max } => {
                write!(
                    f,
                    "value is too large: {actual} bytes (maximum: {max} bytes)"
                )
            }
            Self::TruncatedRecord { expected, actual } => write!(
                f,
                "record is truncated: expected {expected} bytes, found {actual} bytes"
            ),
            Self::InvalidTombstone(value) => {
                write!(f, "invalid tombstone value: {value} (expected 0 or 1)")
            }
            Self::ChecksumMismatch { expected, actual } => write!(
                f,
                "record checksum mismatch: expected {expected:#010x}, calculated {actual:#010x}"
            ),
            Self::CorruptedRecord(reason) => write!(f, "corrupted record: {reason}"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// The result type returned by storage operations.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as _;

    #[test]
    fn io_error_preserves_its_source() {
        let error = Error::from(io::Error::new(io::ErrorKind::NotFound, "database file"));

        assert_eq!(error.source().unwrap().to_string(), "database file");
        assert_eq!(error.to_string(), "storage I/O error: database file");
    }

    #[test]
    fn validation_error_has_no_source() {
        let error = Error::KeyTooLarge {
            actual: 1025,
            max: 1024,
        };

        assert!(error.source().is_none());
        assert_eq!(
            error.to_string(),
            "key is too large: 1025 bytes (maximum: 1024 bytes)"
        );
    }
}
