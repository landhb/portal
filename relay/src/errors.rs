use core::fmt::Debug;
use core::fmt::Display;
use core::fmt::Formatter;
use core::fmt::Result;
use std::error::Error;

/// Error type for this library, optionally implements `std::error::Error`.
#[allow(dead_code)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RelayError {
    /// Operation Would Block
    WouldBlock,

    /// No file descriptor
    MissingFileDescriptor,

    /// FFI Type Error
    TypeError,

    /// Unknown - catch all, return this instead of panicing
    Unknown(i32),
}

impl Display for RelayError {
    #[inline(always)]
    fn fmt(&self, f: &mut Formatter) -> Result {
        <RelayError as Debug>::fmt(self, f)
    }
}

impl Error for RelayError {}

impl RelayError {
    /// Obtain the KeyError derived from checking errno
    pub fn from_errno() -> RelayError {
        match unsafe { *libc::__errno_location() } {
            // Known error conversion
            libc::EWOULDBLOCK => RelayError::WouldBlock,

            // Unknown, provide error code for debugging
            x => RelayError::Unknown(x),
        }
    }
}
