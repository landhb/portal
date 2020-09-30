use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::fs::File;
use memmap::Mmap;
use std::fs::OpenOptions;

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
use file::{PortalFile,PortalFileImmutable,PortalFileMutable};
use chunks::PortalChunks;

pub const DEFAULT_PORT: u16 = 13265;

/**
 * The primary interface into the library
 */
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Portal{

    // Information to correlate
    // connections on the relay
    id: String,
    direction: Option<Direction>,

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

pub struct PortalEncryptState {
    cipher: ChaCha20Poly1305,
}


#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum Direction {
    Sender,
    Receiver,
}


impl Portal {
    
    /**
     * Initialize 
     */
    pub fn init(direction: Option<Direction>, 
                id: String,
                password: String,
                mut filename: Option<String>) -> (Portal,Vec<u8>) {

        
        // use password to compute ID string
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

        return (Portal {
            direction: direction,
            id: id_hash,
            filename: filename,
            filesize: 0,
            state: Some(s1),
            key: None,
        }, outbound_msg);
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
        reader.read(&mut res)?;
        Ok(res)
    }

    /**
     * Attempt to deserialize from a vector
     */
    pub fn parse(data: &Vec<u8>) -> Result<Portal> {
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

    pub fn get_direction(&self) -> Option<Direction> {
        self.direction.clone()
    }

    pub fn set_id(&mut self, id: String) {
        self.id = id;
    }

    pub fn set_direction(&mut self, direction: Option<Direction>) {
        self.direction = direction;
    }

    /*
     * mmap's a file into memory for reading
     */
    pub fn load_file<'a>(&'a self, f: &str) -> Result<PortalFile>  {
        let file = File::open(f)?;
        let mmap = unsafe { Mmap::map(&file)?  };

        let key = self.key.as_ref().ok_or_else(|| PortalError::NoPeer)?;
        let cha_key = Key::from_slice(&key[..]);

        let state = PortalEncryptState {
            cipher: ChaCha20Poly1305::new(cha_key),
        };

        Ok(PortalFile::Immutable(PortalFileImmutable::init(mmap,state)))
    }


    /*
     * mmap's a file into memory for writing
     */
    pub fn create_file<'a>(&'a self, f: &str) -> Result<PortalFile>  {

        let file = OpenOptions::new()
                       .read(true)
                       .write(true)
                       .create(true)
                       .open(&f)?;

        let key = self.key.as_ref().ok_or_else(|| PortalError::NoPeer)?;

        let cha_key = Key::from_slice(&key[..]);

        let state = PortalEncryptState {
            cipher: ChaCha20Poly1305::new(cha_key),
        };

        Ok(PortalFile::Mutable(PortalFileMutable::init(file,state)))
    }

    /**
     * Returns an iterator over the chunks to send it over the
     * network
     */
    pub fn get_chunks<'a>(&self, data: &'a PortalFile, chunk_size: usize) -> PortalChunks<'a,u8> {
        
        let bytes = match data.get_bytes() {
            Ok(data) => data,
            Err(_) => &[], // iterator will be empty for writer files
        };

        PortalChunks::init(
            &bytes, // TODO: verify that this is zero-copy/move
            chunk_size,
            data,
        )
    }


    pub fn confirm_peer(&mut self, msg_data: &[u8]) -> Result<()> {

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
        let dir = Some(Direction::Receiver);
        let pass ="test".to_string();
        let (mut receiver,receiver_msg) = Portal::init(dir,pass,None);

        // sender
        let dir = Some(Direction::Sender);
        let pass ="test".to_string();
        let (mut sender,sender_msg) = Portal::init(dir,pass,None);

        receiver.confirm_peer(&sender_msg).unwrap();
        sender.confirm_peer(&receiver_msg).unwrap();

        assert_eq!(receiver.key,sender.key);
    }


    #[test]
    fn portalfile_iterator() {
        
        // receiver
        let dir = Some(Direction::Receiver);
        let pass ="test".to_string();
        let (_receiver,receiver_msg) = Portal::init(dir,pass,None);

        // sender
        let dir = Some(Direction::Sender);
        let pass ="test".to_string();
        let (mut sender,_sender_msg) = Portal::init(dir,pass,None);

        // Confirm
        sender.confirm_peer(&receiver_msg).unwrap();

        // TODO change test file
        let file = sender.load_file("/etc/passwd").unwrap();

        let chunk_size = 10;
        let chunks = sender.get_chunks(&file,chunk_size);
        for v in chunks.into_iter() {
            // Encrypted chunk size will always be
            // chunk_size + 32 + 12, because: 
            //
            // - ChaCha20 is a 256bit cipher = 32 bytes
            // - The attached nonce is 12 bytes
            assert!(v.len() <= chunk_size+32+12);
        }


        let chunk_size = 1024;
        let chunks = sender.get_chunks(&file,chunk_size);
        for v in chunks.into_iter() {
            assert!(v.len() <= chunk_size+32+12);
        }

    }

    #[test]
    fn portal_createfile() {
        // receiver
        let dir = Some(Direction::Receiver);
        let pass ="test".to_string();
        let (mut receiver,receiver_msg) = Portal::init(dir,pass,None);

        // sender
        let dir = Some(Direction::Sender);
        let pass ="test".to_string();
        let (mut sender,sender_msg) = Portal::init(dir,pass,None);

        // Confirm
        sender.confirm_peer(&receiver_msg).unwrap();
        receiver.confirm_peer(&sender_msg).unwrap();

        // TODO change test file
        let file_src = sender.load_file("/etc/passwd").unwrap();
        let file_dst = receiver.create_file("/tmp/passwd").unwrap();

        let chunk_size = 4096;
        let chunks = sender.get_chunks(&file_src,chunk_size);

        for v in chunks.into_iter() {

             assert!(v.len() <= chunk_size+32+12);

            // test writing chunk
            file_dst.process_given_chunk(&v).unwrap();
        } 
    }


    #[test]
    #[should_panic]
    fn portal_createfile_no_peer() {
        let dir = Some(Direction::Sender);
        let pass = "test".to_string();
        let (portal,_msg) = Portal::init(dir,pass, None);

        // will panic due to lack of peer
        let _file_dst = portal.create_file("/tmp/passwd").unwrap();
    }

    #[test]
    #[should_panic]
    fn portal_loadfile_no_peer() {
        let dir = Some(Direction::Sender);
        let pass = "test".to_string();
        let (portal,_msg) = Portal::init(dir,pass, None);

        // will panic due to lack of peer
        let _file_src = portal.load_file("/etc/passwd").unwrap();
    }

}
