//! portal-lib
//!
//! A small Protocol Library for [Portal](https://github.com/landhb/portal) - An encrypted file transfer utility 
//!
//! This crate enables a consumer to: 
//!
//! - Create/serialize/deserialize Portal request/response messages.
//! - Negoticate a symmetric key with a peer using [SPAKE2](https://docs.rs/spake2/0.2.0/spake2) 
//! - Encrypt files with [Chacha20poly1305](https://blog.cloudflare.com/it-takes-two-to-chacha-poly/) using the [RustCrypto implementation](https://github.com/rusticata/tls-parser)
//! - Send/receive files through a Portal relay
//!
//!
//! Example of SPAKE2 key negotiation:
//!
//! ```no_run
//! use portal_lib::{Portal,Direction};
//!
//! // receiver
//! let dir = Direction::Receiver;
//! let pass ="test".to_string();
//! let (mut receiver,receiver_msg) = Portal::init(dir,"id".to_string(),pass,None);
//!
//! // sender
//! let dir = Direction::Sender;
//! let pass ="test".to_string();
//! let (mut sender,sender_msg) = Portal::init(dir,"id".to_string(),pass,None);
//!
//! receiver.derive_key(&sender_msg).unwrap();
//! sender.derive_key(&receiver_msg).unwrap();
//!
//! // assert_eq!(receiver.key,sender.key);
//! ```
//!
//! Example of Sending a file:
//!
//! ```ignore
//! use portal_lib::{Portal,Direction};
//!
//! // sender
//! let dir = Direction::Sender;
//! let pass ="test".to_string();
//! let (mut portal,msg) = Portal::init(dir,"id".to_string(),pass,None);
//!
//! // complete key derivation + peer verification
//!
//! // open file read-only for sending
//! let mut file = portal.load_file(fpath)?;
//!
//! // Encrypt the file and share state 
//! file.encrypt()?;
//! file.sync_file_state(&mut client)?;
//!
//! // This will be empty for files created with create_file()
//! let chunks = portal.get_chunks(&file,portal::CHUNK_SIZE);
//!
//! for data in chunks.into_iter() {
//!     client.write_all(&data)?;
//!     total += data.len(); 
//! }
//! ```
//!
//! Example of Receiving a file:
//!
//! ```ignore
//! use portal_lib::{Portal,Direction};
//!
//! // receiver
//! let dir = Direction::Receiver;
//! let pass ="test".to_string();
//! let (mut portal,msg) = Portal::init(dir,"id".to_string(),pass,None);
//!
//! // complete key derivation + peer verification
//!
//! // create outfile
//! let mut file = portal.create_file(&fname, fsize)?;
//!
//! // Receive until connection is done
//! let len = file.download_file(&client,|x| {pb.set_position(x)})?;
//!
//! assert_eq!(len as u64, fsize);
//!
//! // Decrypt the file
//! file.decrypt()?;
//! ```


use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::fs::File;
use memmap::MmapOptions;
use std::fs::OpenOptions;
use hkdf::Hkdf;

// Key Exchange
use spake2::{Ed25519Group, Identity, Password, SPAKE2,Group};
use sha2::{Sha256, Digest};

// File encryption
use chacha20poly1305::{ChaCha20Poly1305, Key}; 
use chacha20poly1305::aead::{NewAead};

pub mod errors;
mod file;
mod chunks;


use errors::PortalError;
use file::PortalFile;
use chunks::PortalChunks;

pub const DEFAULT_PORT: u16 = 13265;
pub const CHUNK_SIZE: usize = 65535;


/**
 * The primary interface into the library
 */
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Portal{

    // Information to correlate
    // connections on the relay
    id: String,
    direction: Direction,

    // Metadata to be exchanged
    // between peers
    filename: Option<String>,
    filesize: u64,

    // Never serialized or sent to the relay
    #[serde(skip)]
    state: Option<SPAKE2<Ed25519Group>>,

    // Never serialized or sent to the relay
    #[serde(skip)]
    key: Option<Vec<u8>>,
}



#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum Direction {
    Sender,
    Receiver,
}

/**
 * Method to compair arbitrary &[u8] slices, used internally
 * to compare key exchange and derivation data
 */
fn compare_key_derivations(a: &[u8], b: &[u8]) -> std::cmp::Ordering {
    for (ai, bi) in a.iter().zip(b.iter()) {
        match ai.cmp(&bi) {
            std::cmp::Ordering::Equal => continue,
            ord => return ord
        }
    }

    /* if every single element was equal, compare length */
    a.len().cmp(&b.len())
}

impl Portal {
    
    /**
     * Initialize a new portal request
     */
    pub fn init(direction: Direction, 
                id: String,
                password: String,
                mut filename: Option<String>) -> (Portal,Vec<u8>) {

        
        // hash the ID string
        let mut hasher = Sha256::new();
        hasher.update(&id);
        let id_bytes = hasher.finalize();
        let id_hash = hex::encode(&id_bytes);

        let (s1, outbound_msg) = SPAKE2::<Ed25519Group>::start_symmetric(
           &Password::new(&password.as_bytes()),
           &Identity::new(&id_bytes));
       
        // if a file was provided, trim it to just the file name
        if let Some(file) = filename {
            let f = std::path::Path::new(&file);
            let f = f.file_name().unwrap().to_str().unwrap();
            filename = Some(f.to_string());
        }

        (Portal {
            direction,
            id: id_hash,
            filename,
            filesize: 0,
            state: Some(s1),
            key: None,
        }, outbound_msg)
    }

    /**
     * Construct from a stream reader, consuming the bytes
     */
    pub fn read_response_from<R>(reader: R) -> Result<Portal> 
    where
        R: std::io::Read {
        Ok(bincode::deserialize_from::<R,Portal>(reader)?)
    }

    /**
     * Receive the bytes necessary for a confirmation message
     * from a stream reader, consuming the bytes
     */
    pub fn read_confirmation_from<R>(mut reader: R) -> Result<[u8;33]> 
    where
       R: std::io::Read {
        assert_eq!(33,Portal::get_peer_msg_size());
        let mut res = [0u8;33];
        reader.read_exact(&mut res)?;
        Ok(res)
    }

    /**
     * Attempt to deserialize from a vector
     */
    pub fn parse(data: &[u8]) -> Result<Portal> {
        Ok(bincode::deserialize(&data)?)
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        Ok(bincode::serialize(&self)?)
    }

    pub fn get_file_size(&self) -> u64 {
        self.filesize
    }

    pub fn set_file_size(&mut self, size: u64) {
        self.filesize = size;
    } 

    pub fn get_file_name<'a>(&'a self) -> Result<&'a str> {
        match &self.filename {
            Some(f) => Ok(f.as_str()),
            None => Err(PortalError::NoneError.into()),
        }
    }

    pub fn get_id(&self) -> &String {
        &self.id
    }

    pub fn get_direction(&self) -> Direction {
        self.direction.clone()
    }

    pub fn set_id(&mut self, id: String) {
        self.id = id;
    }

    pub fn set_direction(&mut self, direction: Direction) {
        self.direction = direction;
    }

    /*
     * mmap's a file into memory for reading
     */
    pub fn load_file<'a>(&'a self, f: &str) -> Result<PortalFile>  {
        let file = File::open(f)?;
        let mmap = unsafe { MmapOptions::new().map_copy(&file)? };

        let key = self.key.as_ref().ok_or_else(|| PortalError::NoPeer)?;
        let cha_key = Key::from_slice(&key[..]);

        let cipher = ChaCha20Poly1305::new(cha_key);

        Ok(PortalFile::init(mmap,cipher))
    }


    /*
     * mmap's a file into memory for writing
     */
    pub fn create_file<'a>(&'a self, f: &str, size: u64) -> Result<PortalFile>  {

        let file = OpenOptions::new()
                       .read(true)
                       .write(true)
                       .create(true)
                       .open(&f)?;

        file.set_len(size)?;

        let key = self.key.as_ref().ok_or_else(|| PortalError::NoPeer)?;


        let mmap = unsafe {
            MmapOptions::new().map_mut(&file)?
        };

        let cha_key = Key::from_slice(&key[..]);

        let cipher = ChaCha20Poly1305::new(cha_key);

        Ok(PortalFile::init(mmap,cipher))
    }

    /**
     * Returns an iterator over the chunks to send it over the
     * network
     */
    pub fn get_chunks<'a>(&self, data: &'a PortalFile, chunk_size: usize) -> PortalChunks<'a,u8> {
        PortalChunks::init(
            &data.mmap[..], // TODO: verify that this is zero-copy
            chunk_size,
        )
    }

    /**
     * Derive a shared key with the exchanged data
     * at this point in the exchange we have not verified that our peer
     * has derived the same key as us
     */
    pub fn derive_key(&mut self, msg_data: &[u8]) -> Result<()> {

        // after calling finish() the SPAKE2 struct will be consumed
        // so we must replace the value stored in self.state
        let state = std::mem::replace(&mut self.state, None);

        let state = state.ok_or_else(|| PortalError::BadState)?;

        self.key = match state.finish(msg_data) {
            Ok(res) => Some(res),
            Err(_) => {return Err(PortalError::BadMsg.into());}
        };

        Ok(())
    }

    

    /**
     * Confirm that the peer has derived the same key
     */
    pub fn confirm_peer<R>(&mut self, mut client: R) -> Result<()>
    where
       R: std::io::Read + std::io::Write {

        let key = self.key.as_ref().ok_or_else(|| PortalError::NoPeer)?;

        let sender_info = format!("{}-{}",self.id,"senderinfo");
        let receiver_info = format!("{}-{}",self.id,"receiverinfo");

        // Perform key confirmation step
        let h = Hkdf::<Sha256>::new(None,&key);
        let mut peer_msg = [0u8;42];
        let mut sender_confirm = [0u8; 42];
        let mut receiver_confirm = [0u8; 42];
        h.expand(&sender_info.as_bytes(), &mut sender_confirm).unwrap();
        h.expand(&receiver_info.as_bytes(), &mut receiver_confirm).unwrap();


        match self.direction {
            Direction::Sender => {
                client.write_all(&sender_confirm)?;
                client.read_exact(&mut peer_msg)?;

                if compare_key_derivations(&peer_msg,&receiver_confirm) == std::cmp::Ordering::Equal {
                    return Ok(());
                }

                Err(PortalError::BadMsg.into())
            }
            Direction::Receiver => {
                client.write_all(&receiver_confirm)?;
                client.read_exact(&mut peer_msg)?;

                if compare_key_derivations(&peer_msg,&sender_confirm) == std::cmp::Ordering::Equal {
                    return Ok(());
                }

                Err(PortalError::BadMsg.into())
            }
        }

    }

    fn get_peer_msg_size() -> usize {
        // The exchanged message is the CompressedEdwardsY + 1 byte for the SPAKE direction
        let edwards_point = <spake2::Ed25519Group as Group>::Element::default();
        let compressed = edwards_point.compress();
        std::mem::size_of_val(&compressed)+1
    }

}

#[cfg(test)]
mod tests {
    use super::{Portal,Direction};

    #[test]
    fn key_derivation() {

        // receiver
        let dir = Direction::Receiver;
        let pass ="test".to_string();
        let (mut receiver,receiver_msg) = Portal::init(dir,"id".to_string(),pass,None);

        // sender
        let dir = Direction::Sender;
        let pass ="test".to_string();
        let (mut sender,sender_msg) = Portal::init(dir,"id".to_string(),pass,None);

        receiver.derive_key(&sender_msg).unwrap();
        sender.derive_key(&receiver_msg).unwrap();

        assert_eq!(receiver.key,sender.key);
    }

    #[test]
    fn portal_load_file() {
        let dir = Direction::Receiver;
        let pass ="test".to_string();
        let (_receiver,receiver_msg) = Portal::init(dir,"id".to_string(),pass,None);

        // sender
        let dir = Direction::Sender;
        let pass ="test".to_string();
        let (mut sender,_sender_msg) = Portal::init(dir,"id".to_string(),pass,None);

        // Confirm
        sender.derive_key(&receiver_msg).unwrap();

        // TODO change test file
        let _file = sender.load_file("/etc/passwd").unwrap();
    }

    #[test]
    fn portalfile_chunks_iterator() {
        
        // receiver
        let dir = Direction::Receiver;
        let pass ="test".to_string();
        let (_receiver,receiver_msg) = Portal::init(dir,"id".to_string(),pass,None);

        // sender
        let dir = Direction::Sender;
        let pass ="test".to_string();
        let (mut sender,_sender_msg) = Portal::init(dir,"id".to_string(),pass,None);

        // Confirm
        sender.derive_key(&receiver_msg).unwrap();

        // TODO change test file
        let file = sender.load_file("/etc/passwd").unwrap();

        let chunk_size = 10;
        let chunks = sender.get_chunks(&file,chunk_size);
        for v in chunks.into_iter() {
            assert!(v.len() <= chunk_size);
        }


        let chunk_size = 1024;
        let chunks = sender.get_chunks(&file,chunk_size);
        for v in chunks.into_iter() {
            assert!(v.len() <= chunk_size);
        }

    }

    #[test]
    fn portal_createfile() {
        // receiver
        let dir = Direction::Receiver;
        let pass ="test".to_string();
        let (mut receiver,receiver_msg) = Portal::init(dir,"id".to_string(),pass,None);

        // sender
        let dir = Direction::Sender;
        let pass ="test".to_string();
        let (mut sender,sender_msg) = Portal::init(dir,"id".to_string(),pass,None);

        // Confirm
        sender.derive_key(&receiver_msg).unwrap();
        receiver.derive_key(&sender_msg).unwrap();

        // TODO change test file
        let _file_dst = receiver.create_file("/tmp/passwd",4096).unwrap();
    }

    #[test]
    fn portal_write_chunk() {
        // receiver
        let dir = Direction::Receiver;
        let pass ="test".to_string();
        let (mut receiver,receiver_msg) = Portal::init(dir,"id".to_string(),pass,None);

        // sender
        let dir = Direction::Sender;
        let pass ="test".to_string();
        let (mut sender,sender_msg) = Portal::init(dir,"id".to_string(),pass,None);

        // Confirm
        sender.derive_key(&receiver_msg).unwrap();
        receiver.derive_key(&sender_msg).unwrap();

        // TODO change test file
        let file_src = sender.load_file("/etc/passwd").unwrap();
        let mut file_dst = receiver.create_file("/tmp/passwd",4096).unwrap();

        let chunk_size = 4096;
        let chunks = sender.get_chunks(&file_src,chunk_size);

        for v in chunks.into_iter() {

            assert!(v.len() <= chunk_size);

            // test writing chunk
            file_dst.write_given_chunk(&v).unwrap();
        } 
    }

    #[test]
    #[should_panic]
    fn portal_createfile_no_peer() {
        let dir = Direction::Sender;
        let pass = "test".to_string();
        let (portal,_msg) = Portal::init(dir,"id".to_string(),pass, None);

        // will panic due to lack of peer
        let _file_dst = portal.create_file("/tmp/passwd",4096).unwrap();
    }

    #[test]
    #[should_panic]
    fn portal_loadfile_no_peer() {
        let dir = Direction::Sender;
        let pass = "test".to_string();
        let (portal,_msg) = Portal::init(dir,"id".to_string(),pass, None);

        // will panic due to lack of peer
        let _file_src = portal.load_file("/etc/passwd").unwrap();
    }

}
