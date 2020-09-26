use thiserror::Error;
use std::io;

#[derive(Error, Debug)]
pub enum PortalError {
    #[error("Must be immutable")]
    Mutablility,
    #[error("Bad registration")]
    BadRegistration,
    #[error("No state initialized")]
    BadState,
    #[error("KeyDerivationFailed")]
    BadMsg,
    #[error("Interrupted")]
    Interrupted,
    #[error("WouldBlock")]
    WouldBlock,
    #[error("Disconnected")]
    Disconnect(#[from] io::Error),
}
