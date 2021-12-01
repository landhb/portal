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
//! let id = "id".to_string();
//! let pass ="test".to_string();
//! let (mut receiver,receiver_msg) = Portal::init(Direction::Receiver,id,pass,None);
//!
//! // sender
//! let id = "id".to_string();
//! let pass ="test".to_string();
//! let (mut sender,sender_msg) = Portal::init(Direction::Sender,id,pass,None);
//!
//! // Both clients should derive the same key
//! receiver.derive_key(&sender_msg).unwrap();
//! sender.derive_key(&receiver_msg).unwrap();
//!
//! ```
//! You can use the `Portal::confirm_peer()` method to verify that a remote peer has derived the same key
//! as you, as long as the communication stream implements the std::io::Read and std::io::Write traits.
//!
//! Example of Sending a file:
//!
//! ```no_run
//! use portal_lib::{Portal,Direction};
//! use std::net::TcpStream;
//! use std::io::Write;
//!
//! let mut client = TcpStream::connect("127.0.0.1:34254").unwrap();
//!
//! // Create portal request as the Sender
//! let id = "id".to_string();
//! let pass ="test".to_string();
//! let (mut portal,msg) = Portal::init(Direction::Sender,id,pass,None);
//!
//! // complete key derivation + peer verification
//!
//! let mut file = portal.load_file("/tmp/test").unwrap();
//!
//! // Encrypt the file and share state
//! file.encrypt().unwrap();
//! file.sync_file_state(&mut client).unwrap();
//!
//! for data in file.get_chunks(portal_lib::CHUNK_SIZE) {
//!     client.write_all(&data).unwrap();
//! }
//! ```
//!
//! Example of Receiving a file:
//!
//! ```no_run
//! use portal_lib::{Portal,Direction};
//! use std::net::TcpStream;
//! use std::io::Write;
//!
//! let mut client = TcpStream::connect("127.0.0.1:34254").unwrap();
//!
//! // receiver
//! let dir = Direction::Receiver;
//! let pass ="test".to_string();
//! let (mut portal,msg) = Portal::init(dir,"id".to_string(),pass,None);
//!
//! // serialize & send request
//! let request = portal.serialize().unwrap();
//! client.write_all(&request).unwrap();
//!
//! // get response
//! let response = Portal::read_response_from(&client).unwrap();
//!
//! // complete key derivation + peer verification
//!
//! // create outfile
//! let fsize = response.get_file_size();
//! let mut file = portal.create_file("/tmp/test", fsize).unwrap();
//!
//! let callback = |x| { println!("Received {} bytes",x); };
//!
//! // Receive until connection is done
//! let len = file.download_file(&client,callback).unwrap();
//!
//! assert_eq!(len as u64, fsize);
//!
//! // Decrypt the file
//! file.decrypt().unwrap();
//! ```

use anyhow::Result;
use memmap::MmapOptions;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::fs::OpenOptions;

// Key Exchange
use hkdf::Hkdf;
use sha2::{Digest, Sha256};
use spake2::{Ed25519Group, Identity, Password, SPAKE2};

// File encryption
use chacha20poly1305::aead::NewAead;
use chacha20poly1305::{ChaCha20Poly1305, Key};

mod chunks;
pub mod errors;
pub mod file;

use errors::PortalError;
use file::PortalFile;

/**
 * Arbitrary port for the Portal protocol
 */
pub const DEFAULT_PORT: u16 = 13265;

/**
 * Default chunk size
 */
pub const CHUNK_SIZE: usize = 65535;

/**
 * A data format exchanged by each peer to derive
 * the shared session key
 */
pub type PortalConfirmation = [u8; 33];

/**
 * The primary interface into the library. The Portal struct
 * contains data associated with either a new request or a response
 * from a peer.
 */
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Portal {
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

/**
 * An enum to describe the direction of each file transfer
 * participant (i.e Sender/Receiver)
 */
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
            ord => return ord,
        }
    }

    /* if every single element was equal, compare length */
    a.len().cmp(&b.len())
}

impl Portal {
    /**
     * Initialize a new portal request
     *
     * # Example
     *
     * ```
     * use portal_lib::{Portal,Direction};
     *
     * // the shared password should be secret and hard to guess/crack
     * // see the portal-client as an example of potential usage
     * let id = "my client ID".to_string();
     * let password = "testpasswd".to_string();
     * let portal = Portal::init(Direction::Sender,id,password,None);
     * ```
     */
    pub fn init(
        direction: Direction,
        id: String,
        password: String,
        mut filename: Option<String>,
    ) -> (Portal, Vec<u8>) {
        // hash the ID string
        let mut hasher = Sha256::new();
        hasher.update(&id);
        let id_bytes = hasher.finalize();
        let id_hash = hex::encode(&id_bytes);

        let (s1, outbound_msg) = SPAKE2::<Ed25519Group>::start_symmetric(
            &Password::new(&password.as_bytes()),
            &Identity::new(&id_bytes),
        );

        // if a file was provided, trim it to just the file name
        if let Some(file) = filename {
            let f = std::path::Path::new(&file);
            let f = f.file_name().unwrap().to_str().unwrap();
            filename = Some(f.to_string());
        }

        (
            Portal {
                direction,
                id: id_hash,
                filename,
                filesize: 0,
                state: Some(s1),
                key: None,
            },
            outbound_msg,
        )
    }

    /**
     * Initialize with existing data, attempting to deserialize bytes
     * into a Portal struct
     *
     * # Example
     *
     * ```
     * use portal_lib::Portal;
     * use std::io::Read;
     * fn example(mut client: std::net::TcpStream) {
     *     let mut buf = [0; 1024];
     *     let len = client.read(&mut buf);
     *     let response = match Portal::parse(&buf) {
     *          Ok(r) => r,
     *          Err(_) => {
     *               println!("Failed to read request/response...");
     *               return;
     *          }
     *     };
     * }
     * ```
     */
    pub fn parse(data: &[u8]) -> Result<Portal> {
        Ok(bincode::deserialize(&data)?)
    }

    /**
     * Initialize by reading from a stream that implements the
     * std::io::Read trait, consuming the bytes
     *
     * # Example
     *
     * ```
     * use portal_lib::Portal;
     * fn example(client: std::net::TcpStream) {
     *     let response = match Portal::read_response_from(&client) {
     *          Ok(r) => r,
     *          Err(_) => {
     *               println!("Failed to read response...");
     *               return;
     *          }
     *     };
     * }
     * ```
     */
    pub fn read_response_from<R>(reader: R) -> Result<Portal>
    where
        R: std::io::Read,
    {
        Ok(bincode::deserialize_from::<R, Portal>(reader)?)
    }

    /**
     * Receive the bytes necessary for a confirmation message
     * from a stream that implements std::io::Read, consuming the bytes
     */
    pub fn read_confirmation_from<R>(mut reader: R) -> Result<PortalConfirmation>
    where
        R: std::io::Read,
    {
        let mut res: PortalConfirmation = [0u8; 33];
        reader.read_exact(&mut res)?;
        Ok(res)
    }

    /**
     * Attempt to serialize a Portal struct into a vector
     */
    pub fn serialize(&self) -> Result<Vec<u8>> {
        Ok(bincode::serialize(&self)?)
    }

    /*
     * mmap's a file into memory for reading
     */
    pub fn load_file<'a>(&'a self, f: &str) -> Result<PortalFile> {
        let file = File::open(f)?;
        let mmap = unsafe { MmapOptions::new().map_copy(&file)? };

        let key = self.key.as_ref().ok_or_else(|| PortalError::NoPeer)?;
        let cha_key = Key::from_slice(&key[..]);

        let cipher = ChaCha20Poly1305::new(cha_key);

        Ok(PortalFile::init(mmap, cipher))
    }

    /*
     * mmap's a file into memory for writing
     */
    pub fn create_file<'a>(&'a self, f: &str, size: u64) -> Result<PortalFile> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&f)?;

        file.set_len(size)?;

        let key = self.key.as_ref().ok_or_else(|| PortalError::NoPeer)?;

        let mmap = unsafe { MmapOptions::new().map_mut(&file)? };

        let cha_key = Key::from_slice(&key[..]);

        let cipher = ChaCha20Poly1305::new(cha_key);

        Ok(PortalFile::init(mmap, cipher))
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
            Err(_) => {
                return Err(PortalError::BadMsg.into());
            }
        };

        Ok(())
    }

    /**
     * Confirm that the peer has derived the same key
     */
    pub fn confirm_peer<R>(&mut self, mut client: R) -> Result<()>
    where
        R: std::io::Read + std::io::Write,
    {
        let key = self.key.as_ref().ok_or_else(|| PortalError::NoPeer)?;

        let sender_info = format!("{}-{}", self.id, "senderinfo");
        let receiver_info = format!("{}-{}", self.id, "receiverinfo");

        // Perform key confirmation step
        let h = Hkdf::<Sha256>::new(None, &key);
        let mut peer_msg = [0u8; 42];
        let mut sender_confirm = [0u8; 42];
        let mut receiver_confirm = [0u8; 42];
        h.expand(&sender_info.as_bytes(), &mut sender_confirm)
            .unwrap();
        h.expand(&receiver_info.as_bytes(), &mut receiver_confirm)
            .unwrap();

        match self.direction {
            Direction::Sender => {
                client.write_all(&sender_confirm)?;
                client.read_exact(&mut peer_msg)?;

                if compare_key_derivations(&peer_msg, &receiver_confirm)
                    == std::cmp::Ordering::Equal
                {
                    return Ok(());
                }

                Err(PortalError::BadMsg.into())
            }
            Direction::Receiver => {
                client.write_all(&receiver_confirm)?;
                client.read_exact(&mut peer_msg)?;

                if compare_key_derivations(&peer_msg, &sender_confirm) == std::cmp::Ordering::Equal
                {
                    return Ok(());
                }

                Err(PortalError::BadMsg.into())
            }
        }
    }

    /**
     * Returns the file size associated with this request
     */
    pub fn get_file_size(&self) -> u64 {
        self.filesize
    }

    /**
     * Sets the file size associated with this request
     */
    pub fn set_file_size(&mut self, size: u64) {
        self.filesize = size;
    }

    /**
     * Returns the file name associated with this request
     * or a PortalError::NoneError if none exists
     */
    pub fn get_file_name<'a>(&'a self) -> Result<&'a str> {
        match &self.filename {
            Some(f) => Ok(f.as_str()),
            None => Err(PortalError::NoneError.into()),
        }
    }

    /**
     * Returns a copy of the Portal::Direction associated with
     * this Portal request
     */
    pub fn get_direction(&self) -> Direction {
        self.direction.clone()
    }

    /**
     * Sets the Portal::Direction associated with this Poral request
     */
    pub fn set_direction(&mut self, direction: Direction) {
        self.direction = direction;
    }

    /**
     * Returns a reference to the ID associated with this
     * Portal request
     */
    pub fn get_id(&self) -> &String {
        &self.id
    }

    /**
     * Sets the ID associated with this Poral request
     */
    pub fn set_id(&mut self, id: String) {
        self.id = id;
    }
}

#[cfg(test)]
mod tests {
    use super::{Direction, Portal};
    use crate::file::tests::MockTcpStream;
    use hkdf::Hkdf;
    use sha2::Sha256;
    use std::io::Write;

    #[test]
    fn key_derivation() {
        // receiver
        let dir = Direction::Receiver;
        let pass = "test".to_string();
        let (mut receiver, receiver_msg) = Portal::init(dir, "id".to_string(), pass, None);

        // sender
        let dir = Direction::Sender;
        let pass = "test".to_string();
        let (mut sender, sender_msg) = Portal::init(dir, "id".to_string(), pass, None);

        receiver.derive_key(sender_msg.as_slice()).unwrap();
        sender.derive_key(receiver_msg.as_slice()).unwrap();

        assert_eq!(receiver.key, sender.key);
    }

    #[test]
    fn key_confirmation() {
        let mut receiver_side = MockTcpStream {
            data: Vec::with_capacity(crate::CHUNK_SIZE),
        };

        let mut sender_side = MockTcpStream {
            data: Vec::with_capacity(crate::CHUNK_SIZE),
        };

        // receiver
        let dir = Direction::Receiver;
        let pass = "test".to_string();
        let (mut receiver, receiver_msg) = Portal::init(dir, "id".to_string(), pass, None);

        // sender
        let dir = Direction::Sender;
        let pass = "test".to_string();
        let (mut sender, sender_msg) = Portal::init(dir, "id".to_string(), pass, None);

        receiver.derive_key(sender_msg.as_slice()).unwrap();
        sender.derive_key(receiver_msg.as_slice()).unwrap();

        // identifiers known to each party
        let id = receiver.get_id();
        let sender_info = format!("{}-{}", id, "senderinfo");
        let receiver_info = format!("{}-{}", id, "receiverinfo");

        // Perform the HKDF operations
        let h = Hkdf::<Sha256>::new(None, &sender.key.as_ref().unwrap());
        let mut sender_confirm = [0u8; 42];
        let mut receiver_confirm = [0u8; 42];
        h.expand(&sender_info.as_bytes(), &mut sender_confirm)
            .unwrap();
        h.expand(&receiver_info.as_bytes(), &mut receiver_confirm)
            .unwrap();

        // pre-send the appropriate HKDF to each stream, simulating a peer
        receiver_side.write(&sender_confirm).unwrap();
        sender_side.write(&receiver_confirm).unwrap();

        // each side should be able to confirm the other
        receiver.confirm_peer(&mut receiver_side).unwrap();
        sender.confirm_peer(&mut sender_side).unwrap();
    }

    #[test]
    fn portal_load_file() {
        let dir = Direction::Receiver;
        let pass = "test".to_string();
        let (_receiver, receiver_msg) = Portal::init(dir, "id".to_string(), pass, None);

        // sender
        let dir = Direction::Sender;
        let pass = "test".to_string();
        let (mut sender, _sender_msg) = Portal::init(dir, "id".to_string(), pass, None);

        // Confirm
        sender.derive_key(receiver_msg.as_slice()).unwrap();

        // TODO change test file
        let _file = sender.load_file("/etc/passwd").unwrap();
    }

    #[test]
    fn portalfile_chunks_iterator() {
        // receiver
        let dir = Direction::Receiver;
        let pass = "test".to_string();
        let (_receiver, receiver_msg) = Portal::init(dir, "id".to_string(), pass, None);

        // sender
        let dir = Direction::Sender;
        let pass = "test".to_string();
        let (mut sender, _sender_msg) = Portal::init(dir, "id".to_string(), pass, None);

        // Confirm
        sender.derive_key(receiver_msg.as_slice()).unwrap();

        // TODO change test file
        let file = sender.load_file("/etc/passwd").unwrap();

        let chunk_size = 10;
        for v in file.get_chunks(chunk_size) {
            assert!(v.len() <= chunk_size);
        }

        let chunk_size = 1024;
        for v in file.get_chunks(chunk_size) {
            assert!(v.len() <= chunk_size);
        }
    }

    #[test]
    fn portal_createfile() {
        // receiver
        let dir = Direction::Receiver;
        let pass = "test".to_string();
        let (mut receiver, receiver_msg) = Portal::init(dir, "id".to_string(), pass, None);

        // sender
        let dir = Direction::Sender;
        let pass = "test".to_string();
        let (mut sender, sender_msg) = Portal::init(dir, "id".to_string(), pass, None);

        // Confirm
        sender.derive_key(receiver_msg.as_slice()).unwrap();
        receiver.derive_key(sender_msg.as_slice()).unwrap();

        // TODO change test file
        let _file_dst = receiver.create_file("/tmp/passwd", 4096).unwrap();
    }

    #[test]
    fn portal_write_chunk() {
        // receiver
        let dir = Direction::Receiver;
        let pass = "test".to_string();
        let (mut receiver, receiver_msg) = Portal::init(dir, "id".to_string(), pass, None);

        // sender
        let dir = Direction::Sender;
        let pass = "test".to_string();
        let (mut sender, sender_msg) = Portal::init(dir, "id".to_string(), pass, None);

        // Confirm
        sender.derive_key(receiver_msg.as_slice()).unwrap();
        receiver.derive_key(sender_msg.as_slice()).unwrap();

        // TODO change test file
        let file_src = sender.load_file("/etc/passwd").unwrap();
        let mut file_dst = receiver.create_file("/tmp/passwd", 4096).unwrap();

        let chunk_size = 4096;
        for v in file_src.get_chunks(chunk_size) {
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
        let (portal, _msg) = Portal::init(dir, "id".to_string(), pass, None);

        // will panic due to lack of peer
        let _file_dst = portal.create_file("/tmp/passwd", 4096).unwrap();
    }

    #[test]
    #[should_panic]
    fn portal_loadfile_no_peer() {
        let dir = Direction::Sender;
        let pass = "test".to_string();
        let (portal, _msg) = Portal::init(dir, "id".to_string(), pass, None);

        // will panic due to lack of peer
        let _file_src = portal.load_file("/etc/passwd").unwrap();
    }

    #[test]
    fn test_file_trim() {
        let file = Some("/my/path/filename.txt".to_string());

        let dir = Direction::Receiver;
        let pass = "test".to_string();
        let (receiver, _receiver_msg) = Portal::init(dir, "id".to_string(), pass, file);

        let result = receiver.get_file_name().unwrap();

        assert_eq!(result, "filename.txt");
    }

    #[test]
    fn test_compressed_edwards_size() {
        // The exchanged message is the CompressedEdwardsY + 1 byte for the SPAKE direction
        let edwards_point = <spake2::Ed25519Group as spake2::Group>::Element::default();
        let compressed = edwards_point.compress();
        let msg_size: usize = std::mem::size_of_val(&compressed) + 1;

        assert_eq!(33, msg_size);
    }

    #[test]
    fn test_getters_setters() {
        let dir = Direction::Sender;
        let pass = "test".to_string();
        let (mut portal, _msg) = Portal::init(dir, "id".to_string(), pass, None);

        // get/set ID
        portal.set_id("newID".to_string());
        assert_eq!("newID", portal.get_id());

        // get/set direction
        portal.set_direction(Direction::Receiver);
        assert_eq!(portal.get_direction(), Direction::Receiver);

        // get/set direction
        portal.set_file_size(25);
        assert_eq!(portal.get_file_size(), 25);
    }

    #[test]
    fn test_serialize_deserialize() {
        let dir = Direction::Sender;
        let pass = "test".to_string();
        let (portal, _msg) = Portal::init(dir, "id".to_string(), pass, None);

        let ser = portal.serialize().unwrap();
        let res = Portal::parse(&ser).unwrap();

        // fields that should be the same
        assert_eq!(res.id, portal.id);
        assert_eq!(res.direction, portal.direction);
        assert_eq!(res.filename, portal.filename);
        assert_eq!(res.filesize, portal.filesize);

        // fields that shouldn't have been serialized
        assert_ne!(res.state, portal.state);
        assert_eq!(res.state, None);
        assert_eq!(res.key, None);
    }
}
