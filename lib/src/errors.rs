use thiserror::Error;
use std::io;

#[derive(Error, Debug)]
pub enum PortalError {
    #[error("Bad registration")]
    BadRegistration,
    #[error("Interrupted")]
    Interrupted,
    #[error("WouldBlock")]
    WouldBlock,
    #[error("data store disconnected")]
    Disconnect(#[from] io::Error),
    #[error("the data for key `{0}` is not available")]
    Redaction(String),
    #[error("invalid header (expected {expected:?}, found {found:?})")]
    InvalidHeader {
        expected: String,
        found: String,
    },
    #[error("unknown data store error")]
    Unknown,
}