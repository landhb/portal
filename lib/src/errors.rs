use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PortalError {
    #[error("Value doesn't exist")]
    NoneError,
    #[error("Provided filename doesn't return a base filename")]
    BadFileName,
    #[error("Cancelled")]
    Cancelled,
    #[error("Incomplete")]
    Incomplete,
    #[error("Underlying crypto error")]
    CryptoError,
    #[error("Incorrect Mutability")]
    Mutability,
    #[error("Provided storage is too small")]
    BufferTooSmall,
    #[error("Bad registration")]
    BadRegistration,
    #[error("No state initialized")]
    BadState,
    #[error("No peer confirmed")]
    NoPeer,
    #[error("KeyDerivationFailed")]
    BadMsg,
    #[error("EncryptError")]
    EncryptError,
    #[error("DecryptError")]
    DecryptError,
    #[error("IOError")]
    IOError,
    #[error("Interrupted")]
    Interrupted,
    #[error("WouldBlock")]
    WouldBlock,
    #[error("Object could not be serialized")]
    SerializeError,
    #[error("Disconnected")]
    Disconnect(#[from] io::Error),
}
