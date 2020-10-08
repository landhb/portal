use chacha20poly1305::ChaCha20Poly1305;
use anyhow::Result;
use memmap::MmapMut;
use rand::Rng;
use std::io::Write;
use serde::{Serialize, Deserialize};
use chacha20poly1305::{Nonce,Tag,aead::AeadInPlace}; 


use crate::errors::PortalError;



/**
 * A file mapping, either an immutable mmap 
 * or a mutable std::fs::File wrapped in a RefCell
 */
pub struct PortalFile {
    // Memory mapped file
    pub mmap: MmapMut,
    
    // Encryption State
    pub cipher: ChaCha20Poly1305,
    state: StateMetadata,

    // Position
    pos: usize,
}

/**
 * PortalFile metadata containing encryption state 
 * data that must be transferred to the peer for 
 * decryption
 */
#[derive(Serialize, Deserialize, PartialEq)]
struct StateMetadata {
    nonce: Vec<u8>,
    tag: Vec<u8>,
}



impl PortalFile {


    pub fn init(mmap: MmapMut, cipher: ChaCha20Poly1305) -> PortalFile {
        PortalFile{
            mmap,
            cipher,
            pos: 0,
            state: StateMetadata {
                nonce: Vec::new(),
                tag: Vec::new(),
            }
        }
    }

    pub fn encrypt(&mut self) -> Result<()> {

        // Generate random nonce
        let mut rng = rand::thread_rng();
        let rbytes  = rng.gen::<[u8;12]>();
        let nonce = Nonce::from_slice(&rbytes); // 128-bits; unique per chunk
        self.state.nonce.extend(nonce);

        let tag = match self.cipher.encrypt_in_place_detached(nonce, b"", &mut self.mmap[..]) {
            Ok(tag) => tag,
            Err(_e) => {return Err(PortalError::EncryptError.into())},
        };
        self.state.tag.extend(tag);
        Ok(())

    }

    pub fn decrypt(&mut self) -> Result<()> {
        let nonce = Nonce::from_slice(&self.state.nonce);
        let tag = Tag::from_slice(&self.state.tag);
        match self.cipher.decrypt_in_place_detached(nonce, b"", &mut self.mmap[..], &tag) {
            Ok(_) => {Ok(())},
            Err(_e) => {Err(PortalError::EncryptError.into())},
        }
    }


    pub fn sync_file_state<W>(&mut self, mut writer: W) -> Result<usize> 
    where 
        W: std::io::Write {
        let data: Vec<u8> = bincode::serialize(&self.state)?;
        writer.write_all(&data)?;
        Ok(0)
    }

    pub fn download_file<R,F>(&mut self,mut reader: R, callback: F) -> Result<u64>
    where 
        R: std::io::Read, 
        F: Fn(u64) {
        
        let mut buf = vec![0u8;crate::CHUNK_SIZE];

        // First deserialize the Nonce + Tag
        let remote_state: StateMetadata = bincode::deserialize_from(&mut reader)?;
        self.state.nonce.extend(&remote_state.nonce);
        self.state.tag.extend(&remote_state.tag);

        // Anything else is file data
        loop {

            let len = match reader.read(&mut buf) {
                Ok(0) => {return Ok(self.pos as u64);},
                Ok(len) => len,
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e.into()),
            };

            (&mut self.mmap[self.pos..]).write_all(&buf[..len])?;
            self.pos += len;
            callback(self.pos as u64);
        }
    }

    pub fn write_given_chunk(&mut self,data: &[u8]) -> Result<u64> {
        (&mut self.mmap[self.pos..]).write_all(&data)?;
        self.pos += data.len();
        Ok(data.len() as u64)
    }
}

