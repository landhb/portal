use chacha20poly1305::ChaCha20Poly1305;
use anyhow::Result;
use memmap::MmapMut;
use rand::Rng;


use serde::{Serialize, Deserialize};
use chacha20poly1305::{Nonce,Tag}; 


use crate::errors::PortalError;

use chacha20poly1305::aead::AeadInPlace;


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
}

#[derive(Serialize, Deserialize, PartialEq)]
struct StateMetadata {
    nonce: Vec<u8>,
    tag: Vec<u8>,
}



impl PortalFile {


    pub fn init(mmap: MmapMut, cipher: ChaCha20Poly1305) -> PortalFile {
        PortalFile{
            mmap: mmap,
            cipher: cipher,
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


    /*pub fn send_file<W,F>(&mut self, mut writer: W,callback: F) -> Result<usize> 
    where 
        W: std::io::Write, 
        F: Fn(u64) {

        Ok(0)
    } */

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
        use std::io::Write;
        let mut buf = vec![0u8;crate::CHUNK_SIZE];
        let mut written = 0;

        // First deserialize the Nonce + Tag
        let remote_state: StateMetadata = bincode::deserialize_from(&mut reader)?;
        self.state.nonce.extend(&remote_state.nonce);
        self.state.tag.extend(&remote_state.tag);

        // Anything else is file data
        loop {

            let len = match reader.read(&mut buf) {
                Ok(0) => {return Ok(written as u64);},
                Ok(len) => len,
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e.into()),
            };

            match (&mut self.mmap[written..]).write_all(&buf[..len]) {
                Ok(_) => {},
                Err(_) => {
                    println!("written: {:?} len: {:?}", written,len);
                }
            }
            written += len;
            callback(written as u64);
        }
    }
}

