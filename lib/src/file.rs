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
    file: RefCell<std::fs::File>,
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

    pub fn process_next_chunk<R>(&self,reader: R) -> Result<usize> 
    where
        R: std::io::Read {
        match self {
            PortalFile::Immutable(_inner) => Err(PortalError::Mutability.into()),
            PortalFile::Mutable(inner) => {return inner.process_next_chunk(reader);},
        }
    }

    pub fn process_given_chunk(&self,data: &[u8]) -> Result<usize> {
        match self {
            PortalFile::Immutable(_inner) => Err(PortalError::Mutability.into()),
            PortalFile::Mutable(inner) => {return inner.process_given_chunk(data);},
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

    pub fn init(file: std::fs::File, state: PortalEncryptState) -> PortalFileMutable {
        PortalFileMutable{
            file: RefCell::new(file),
            state: state,
        }
    }


    fn process_next_chunk<R>(&self,reader: R) -> Result<usize>
    where 
        R: std::io::Read {
        let chunk: PortalChunk = bincode::deserialize_from::<R,PortalChunk>(reader)?;
        Ok(self.write(chunk)?)
    }

    fn process_given_chunk(&self,data: &[u8]) -> Result<usize> {
        let chunk: PortalChunk = bincode::deserialize(data)?;
        Ok(self.write(chunk)?)
    }

    fn write(&self, chunk: PortalChunk) -> Result<usize> {
        use std::io::Write;
        let nonce = Nonce::from_slice(&chunk.nonce[..]);
        let dec_data = self.state.cipher.decrypt(nonce,&chunk.data[..]).expect("decryption failure!");
        self.file.borrow_mut().write_all(&dec_data)?;
        Ok(chunk.data.len())
    } 

}


impl Drop for PortalFileMutable {

    // attempt to flush to disk
    fn drop(&mut self) {
        let _ = self.file.get_mut().sync_all();
    }

} 