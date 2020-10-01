use anyhow::Result;
use memmap::Mmap;
use std::cell::RefCell;
use serde::{Serialize, Deserialize};
use chacha20poly1305::Nonce; 
use chacha20poly1305::aead::Aead;

use crate::PortalEncryptState;
use crate::errors::PortalError;


/**
 * A file mapping, either an immutable mmap 
 * or a mutable std::fs::File wrapped in a RefCell
 */
pub enum PortalFile {
    Immutable(PortalFileImmutable),
    Mutable(PortalFileMutable),
}

pub struct PortalFileImmutable {
    mmap: Mmap,
    pub state: PortalEncryptState,
}

pub struct PortalFileMutable {
    file: RefCell<async_std::fs::File>, //RefCell<std::fs::File>,
    pub state: PortalEncryptState,
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct PortalChunk {
    pub nonce: Vec<u8>,
    pub data: Vec<u8>,
}


impl PortalFile {

    pub fn get_bytes(&self) -> Result<&[u8]> {
        match self {
            PortalFile::Immutable(inner) => Ok(&inner.mmap[..]),
            PortalFile::Mutable(_inner) => Err(PortalError::Mutability.into()),
        }
    }

    pub async fn process_next_chunk<R>(&self,reader: R) -> Result<PortalChunk> 
    where
        R: async_std::io::ReadExt + std::marker::Unpin {
        match self {
            PortalFile::Immutable(_inner) => Err(PortalError::Mutability.into()),
            PortalFile::Mutable(inner) => {return inner.process_next_chunk(reader).await;},
        }
    }

    pub fn process_given_chunk(&self,data: &[u8]) -> Result<PortalChunk> {
        match self {
            PortalFile::Immutable(_inner) => Err(PortalError::Mutability.into()),
            PortalFile::Mutable(inner) => {return inner.process_given_chunk(data);},
        }
    } 

    pub async fn write(&self, chunk: PortalChunk) -> Result<usize> {
        match self {
            PortalFile::Immutable(_inner) => Err(PortalError::Mutability.into()),
            PortalFile::Mutable(inner) => {return inner.write(chunk).await;},
        }
    }
}

impl PortalFileImmutable {
    pub fn init(mmap: Mmap, state: PortalEncryptState) -> PortalFileImmutable {
        PortalFileImmutable{
            mmap: mmap,
            state: state,
        }
    }
}


impl PortalFileMutable {

    pub fn init(file: async_std::fs::File, state: PortalEncryptState) -> PortalFileMutable {
        PortalFileMutable{
            file: RefCell::new(file),
            state: state,
        }
    }


    async fn process_next_chunk<R>(&self,mut reader: R) -> Result<PortalChunk>
    where 
        R: async_std::io::ReadExt + std::marker::Unpin{

        // Parse the size from the stream
        let mut size_data = vec![0u8; 8];
        reader.read_exact(&mut size_data).await?;
        let size: usize = bincode::deserialize(&size_data)?;

        // Attempt to read chunk from socket
        let mut buf = vec![0;size];
        reader.read_exact(&mut buf).await?;

        // Attempt to deserialize
        let chunk: PortalChunk = bincode::deserialize(&buf)?;
        Ok(chunk)
    }

    fn process_given_chunk(&self,data: &[u8]) -> Result<PortalChunk> {
        let chunk: PortalChunk = bincode::deserialize(data)?;
        Ok(chunk)
    } 

    async fn write(&self, chunk: PortalChunk) -> Result<usize> {
        //use std::io::Write;
        use async_std::prelude::*;
        let nonce = Nonce::from_slice(&chunk.nonce[..]);
        let dec_data = self.state.cipher.decrypt(nonce,&chunk.data[..]).expect("decryption failure!");
        self.file.borrow_mut().write_all(&dec_data).await?;
        Ok(chunk.data.len())
    } 

}


impl Drop for PortalFileMutable {

    // attempt to flush to disk
    fn drop(&mut self) {
        let _ = self.file.get_mut().sync_all();
    }

} 