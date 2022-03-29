use memmap::{MmapMut, MmapOptions};
use std::convert::TryInto;
use std::error::Error;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{Read, Write};

// Key Exchange
use sha2::{Digest, Sha256};
use spake2::{Ed25519Group, Identity, Password, Spake2};

// File encryption
use chacha20poly1305::aead::NewAead;
use chacha20poly1305::{ChaCha20Poly1305, Key};

mod chunks;
use chunks::PortalChunks;

mod file;

// Allow users to access errors
pub mod errors;

/// Lower level protocol methods. Use these
/// if the higher-level Portal interface is
/// too abstract.
pub mod protocol;
use protocol::*;

use errors::PortalError::*;
use file::PortalFile;

/**
 * Arbitrary port for the Portal protocol
 */
pub const DEFAULT_PORT: u16 = 13265;

/**
 * Default chunk size
 */
pub const CHUNK_SIZE: usize = 65536;

/**
 * The primary interface into the library.
 */
#[derive(PartialEq, Debug)]
pub struct Portal {
    // Information to correlate
    // connections on the relay
    id: String,
    direction: Direction,

    // KeyExchange information
    exchange: PortalKeyExchange,

    // Crypto state used to derive the key
    // once we receive a confirmation msg from the peer
    state: Option<Spake2<Ed25519Group>>,

    // Derived session key
    key: Option<Vec<u8>>,
}

impl Portal {
    /// Initialize a new portal request
    ///
    /// # Example
    ///
    /// ```
    /// use portal_lib::{Portal,Direction};
    ///
    /// // the shared password should be secret and hard to guess/crack
    /// // see the portal-client as an example of potential usage
    /// let id = String::from("my client ID");
    /// let password = String::from("testpasswd");
    /// let portal = Portal::init(Direction::Receiver, id, password);
    /// ```
    pub fn init(
        direction: Direction,
        id: String,
        password: String,
    ) -> Result<Portal, Box<dyn Error>> {
        // hash the ID string
        let mut hasher = Sha256::new();
        hasher.update(&id);
        let id_bytes = hasher.finalize();
        let id_hash = hex::encode(&id_bytes);

        // Initialize the state
        let (s1, outbound_msg) = Spake2::<Ed25519Group>::start_symmetric(
            &Password::new(&password.as_bytes()),
            &Identity::new(&id_bytes),
        );

        Ok(Portal {
            direction,
            id: id_hash,
            exchange: outbound_msg.try_into().or(Err(CryptoError))?,
            state: Some(s1),
            key: None,
        })
    }

    /// Negotiate a secure connection over the insecure channel by performing the portal
    /// handshake. Subsequent communication will be encrypted.
    ///
    /// # Example
    ///
    /// ```
    /// use std::net::TcpStream;
    ///
    /// let portal = Portal::init(Direction::Sender,id,password,None);
    /// let mut stream = TcpStream::connect("127.0.0.1:34254")?;
    ///
    /// // conduct the handshake with the peer
    /// portal.handshake(&mut stream).unwrap();
    /// ```
    pub fn handshake<P: Read + Write>(&mut self, peer: &mut P) -> Result<(), Box<dyn Error>> {
        // Send the connection message. If the relay cannot
        // match us with a peer this will fail.
        let confirm =
            Protocol::connect(peer, &self.id, self.direction, self.exchange).or(Err(NoPeer))?;

        // after calling finish() the SPAKE2 struct will be consumed
        // so we must replace the value stored in self.state
        let state = std::mem::replace(&mut self.state, None);
        let state = state.ok_or(BadState)?;

        // Derive the session key
        let key = Protocol::derive_key(state, &confirm).or(Err(BadMsg))?;

        // confirm that the peer has the same key
        Protocol::confirm_peer(&self.id, self.direction, &key, peer)?;

        // Set key for further use
        self.key = Some(key);
        Ok(())
    }

    /// Send a given file over the portal. Must be called after performing the
    /// handshake or this method will return an error.
    ///
    /// # Example
    ///
    /// ```
    /// // The handshake must be performed first, otherwise
    /// // there is no shared key to encrypt the file with
    /// portal.handshake(&mut client);
    ///
    /// // Optional: implement a custom callback to display how much
    /// // has been transferred
    /// fn progress(transferred: usize) {
    ///     println!("sent {:?} bytes", transferred);
    /// }
    ///
    /// // Begin sending the file
    /// portal.send_file(&mut client, "/etc/passwd", Some(progress));
    /// ```
    pub fn send_file<W, D>(
        &mut self,
        peer: &mut W,
        path: &str,
        callback: Option<D>,
    ) -> Result<usize, Box<dyn Error>>
    where
        W: Write,
        D: Fn(usize),
    {
        // Check that the key exists to confirm the handshake is complete
        let key = self.key.as_ref().ok_or(NoPeer)?;

        // Obtain the file name stub from the path
        let p = std::path::Path::new(path);
        let filename = p
            .file_name()
            .ok_or(BadFileName)?
            .to_str()
            .ok_or(BadFileName)?;

        // Map the file into memory
        let mut mmap = self.map_readable_file(path)?;

        // Create the metatada object
        let metadata = Metadata {
            filesize: mmap.len() as u64,
            filename: filename.as_bytes().to_vec(),
        };

        // Write the file metadata over the encrypted channel
        Protocol::encrypt_and_write_object(peer, key, &metadata)?;

        // Encrypt the file in-place & send the header
        Protocol::encrypt_and_write_header_only(peer, key, &mut mmap[..])?;

        // Establish an iterator over the encrypted region
        let chunks = PortalChunks::init(&mmap[..], CHUNK_SIZE);

        // Send the encrypted region in chunks
        let mut total_sent = 0;
        for chunk in chunks.into_iter() {
            peer.write_all(chunk)?;

            // Increment and optionally invoke callback
            total_sent += chunk.len();
            callback.as_ref().map(|c| {
                c(total_sent);
            });
        }
        Ok(total_sent)
    }

    /// Receive the next file over the portal. Must be called after performing
    /// the handshake or this method will return an error.
    ///
    /// # Example
    ///
    /// ```
    /// // The handshake must be performed first, otherwise
    /// // there is no shared key to encrypt the file with
    /// portal.handshake(&mut client);
    ///
    /// // Optional: User callback to confirm/deny a transfer. If
    /// // none is provided, this will default accept the incoming file.
    /// // Return true to accept, false to reject the transfer.
    /// fn confirm_download(path: &str, size: u64) -> bool { true }
    ///
    /// // Optional: implement a custom callback to display how much
    /// // has been transferred
    /// fn progress(transferred: u64) {
    ///     println!("received {:?} bytes", transferred);
    /// }
    ///
    /// // Begin receiving the file
    /// portal.recv_file(&mut client, Some(verify_callback), Some(display_callback));
    /// ```
    pub fn recv_file<R, V, D>(
        &mut self,
        peer: &mut R,
        verify: Option<V>,
        display: Option<D>,
    ) -> Result<(), Box<dyn Error>>
    where
        R: Read,
        V: Fn(&str, u64) -> bool,
        D: Fn(u64),
    {
        unimplemented!()
    }

    /// Helper: mmap's a file into memory for reading
    fn map_readable_file<'a>(&'a self, f: &str) -> Result<MmapMut, Box<dyn Error>> {
        let file = File::open(f)?;
        let mmap = unsafe { MmapOptions::new().map_copy(&file)? };
        Ok(mmap)
    }

    /// Helper: mmap's a file into memory for writing
    fn map_writeable_file<'a>(&'a self, f: &str, size: u64) -> Result<MmapMut, Box<dyn Error>> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&f)?;

        file.set_len(size)?;
        let mmap = unsafe { MmapOptions::new().map_mut(&file)? };
        Ok(mmap)
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
    use crate::file::tests::MockTcpStream;
    use crate::{errors::PortalError, Direction, Portal, StateMetadata};
    use hkdf::Hkdf;
    use rand::Rng;
    use sha2::Sha256;
    use std::io::Write;

    #[test]
    fn metadata_roundtrip() {
        let fsize = 1337;
        let fname = "filename".to_string();

        // receiver
        let dir = Direction::Receiver;
        let pass = "test".to_string();
        let (mut receiver, receiver_msg) = Portal::init(dir, "id".to_string(), pass, None);

        // sender
        let dir = Direction::Sender;
        let pass = "test".to_string();
        let (mut sender, sender_msg) =
            Portal::init(dir, "id".to_string(), pass, Some(fname.clone()));
        sender.set_file_size(fsize);

        // we need a key to be able to encrypt & decrypt
        receiver.derive_key(sender_msg.as_slice()).unwrap();
        sender.derive_key(receiver_msg.as_slice()).unwrap();

        // Mock channel
        let mut stream = MockTcpStream {
            data: Vec::with_capacity(crate::CHUNK_SIZE),
        };

        // Send metadata
        sender.write_metadata_to(&mut stream).unwrap();

        // Recv metadata
        receiver.read_metadata_from(&mut stream).unwrap();

        // Verify both peers now share the same metadata
        assert_eq!(fsize, receiver.get_file_size());
        assert_eq!(fname, receiver.get_file_name().unwrap());
        assert_eq!(
            sender.get_file_name().unwrap(),
            receiver.get_file_name().unwrap()
        );
        assert_eq!(sender.get_file_size(), receiver.get_file_size());
    }

    #[test]
    fn fail_decrypt_metadata() {
        let fsize = 1337;
        let fname = "filename".to_string();

        // receiver
        let dir = Direction::Receiver;
        let pass = "test".to_string();
        let (mut receiver, receiver_msg) = Portal::init(dir, "id".to_string(), pass, None);

        // sender
        let dir = Direction::Sender;
        let pass = "test".to_string();
        let (mut sender, sender_msg) =
            Portal::init(dir, "id".to_string(), pass, Some(fname.clone()));
        sender.set_file_size(fsize);

        // we need a key to be able to encrypt & decrypt
        receiver.derive_key(sender_msg.as_slice()).unwrap();
        sender.derive_key(receiver_msg.as_slice()).unwrap();

        // Mock channel
        let mut stream = MockTcpStream {
            data: Vec::with_capacity(crate::CHUNK_SIZE),
        };

        // Send garbage state
        let mut garbage = bincode::serialize(&StateMetadata::default()).unwrap();
        garbage.extend_from_slice(&bincode::serialize(&vec![0u8]).unwrap());
        stream.write_all(&garbage).unwrap();

        // Verify error is BadState
        let res = receiver.read_metadata_from(&mut stream);
        assert!(res.is_err());
        let _ = res.map_err(|e| match e.downcast_ref::<PortalError>() {
            Some(PortalError::BadState) => anyhow::Ok(()),
            _ => panic!("Unexpected error"),
        });

        // Send garbage metadata
        let state = StateMetadata {
            nonce: rand::thread_rng().gen::<[u8; 12]>().to_vec(),
            tag: rand::thread_rng().gen::<[u8; 16]>().to_vec(),
        };
        let mut garbage = bincode::serialize(&state).unwrap();
        garbage.extend_from_slice(&bincode::serialize(&vec![0u8]).unwrap());
        stream.write_all(&garbage).unwrap();

        // Verify error is DecryptError
        let res = receiver.read_metadata_from(&mut stream);
        assert!(res.is_err());
        let _ = res.map_err(|e| match e.downcast_ref::<PortalError>() {
            Some(PortalError::DecryptError) => anyhow::Ok(()),
            _ => panic!("Unexpected error"),
        });
    }

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
        assert_eq!(res.metadata.filename, portal.metadata.filename);
        assert_eq!(res.metadata.filesize, portal.metadata.filesize);

        // fields that shouldn't have been serialized
        assert_ne!(res.state, portal.state);
        assert_eq!(res.state, None);
        assert_eq!(res.key, None);
    }
}
