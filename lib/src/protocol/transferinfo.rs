use crate::errors::PortalError::*;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::{Path, PathBuf};

/// Metadata about the transfer to be exchanged
/// between peers after key derivation (encrypted)
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Default)]
pub struct Metadata {
    //pub id: u32,
    pub filesize: u64,
    pub filename: String,
}

/// Contains the metadata for all files that will be sent
/// during a particular transfer
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Default)]
pub struct TransferInfo {
    /// The metadata to send to the peer. These
    /// filenames are striped of their path information
    pub all: Vec<Metadata>,

    /// Internal state for a sender to locate files
    #[serde(skip)]
    pub localpaths: Vec<PathBuf>,
}

/// Builder for TransferInfo
pub struct TransferInfoBuilder(TransferInfo);

impl TransferInfo {
    /// Owned TransferInfo
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use std::error::Error;
    /// use portal_lib::TransferInfo;
    ///
    /// fn create_info(files: Vec<PathBuf>) -> Result<TransferInfo, Box<dyn Error>> {
    ///     let mut info = TransferInfo::empty();
    ///
    ///     for file in files {
    ///         info.add_file(file.as_path())?;
    ///     }
    ///
    ///     Ok(info)
    /// }
    /// ```
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
}

impl Default for TransferInfoBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TransferInfoBuilder {
    /// Builder pattern for TransferInfo
    ///
    /// ```
    /// use std::path::Path;
    /// use std::error::Error;
    /// use portal_lib::TransferInfoBuilder;
    ///
    /// fn some_method() -> Result<(), Box<dyn Error>> {
    ///     // Use the builder to create a TransferInfo object
    ///     let mut info = TransferInfoBuilder::new()
    ///         .add_file(Path::new("/etc/passwd"))?
    ///         .finalize();
    ///
    ///     // ... Pass it to methods that require it ...
    ///     Ok(())
    /// }
    /// ```
    pub fn new() -> TransferInfoBuilder {
        Self(TransferInfo::empty())
    }

    pub fn add_file(mut self, path: &Path) -> Result<TransferInfoBuilder, Box<dyn Error>> {
        let _ = self.0.add_file(path)?;
        Ok(self)
    }

    /// Finalize the builder into a TransferInfo object
    pub fn finalize(self) -> TransferInfo {
        self.0
    }
}
