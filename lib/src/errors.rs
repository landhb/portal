use thiserror::Error;
use std::io;

#[derive(Error, Debug)]
pub enum PortalError {
    #[error("Value doesn't exist")]
    NoneError,
    #[error("Incorrect Mutability")]
    Mutability,
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
    #[error("Interrupted")]
    Interrupted,
    #[error("WouldBlock")]
    WouldBlock,
    #[error("Disconnected")]
    Disconnect(#[from] io::Error),
}


