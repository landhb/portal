use crate::errors::PortalError::*;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::{Path, PathBuf};

/// Metadata about the transfer to be exchanged
/// between peers after key derivation (encrypted)
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
pub struct Metadata {
    //pub id: u32,
    pub filesize: u64,
    pub filename: String,
}

/// Contains the metadata for all files that will be sent
/// during a particular transfer
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
pub struct TransferInfo {
    /// The metadata to send to the peer. These
    /// filenames are striped of their path information
    pub all: Vec<Metadata>,

    /// Internal state for a sender to locate files
    #[serde(skip)]
    pub localpaths: Vec<PathBuf>,
}

impl TransferInfo {
    /// Create a TransferInfo object
    pub fn empty() -> TransferInfo {
        TransferInfo {
            all: Vec::new(),
            localpaths: Vec::new(),
        }
    }

    /// Add a file to this transfer
    pub fn add_file<'a>(&'a mut self, path: &Path) -> Result<&'a mut TransferInfo, Box<dyn Error>> {
        self.localpaths.push(path.to_path_buf());
        self.all.push(Metadata {
            filesize: path.metadata()?.len(),
            filename: path
                .file_name()
                .ok_or(BadFileName)?
                .to_str()
                .ok_or(BadFileName)?
                .to_string(),
        });
        Ok(self)
    }

    /// Finalize this TransferInfo, consuming the mutable builder.
    pub fn finalize(self) -> TransferInfo {
        self
    }
}
