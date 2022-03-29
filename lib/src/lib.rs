use memmap::{MmapMut, MmapOptions};
use std::convert::TryInto;
use std::error::Error;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{Read, Write};

// Key Exchange
use sha2::{Digest, Sha256};
use spake2::{Ed25519Group, Identity, Password, Spake2};

#[cfg(test)]
mod tests;

mod chunks;
use chunks::PortalChunks;

// Allow users to access errors
pub mod errors;
use errors::PortalError::*;

/// Lower level protocol methods. Use these
/// if the higher-level Portal interface is
/// too abstract.
pub mod protocol;
use protocol::*;

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

pub type VerifyCallback = fn(&str, u64) -> bool;
pub type ProgressCallback = fn(usize);

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
    pub fn send_file<W>(
        &mut self,
        peer: &mut W,
        path: &str,
        callback: Option<ProgressCallback>,
    ) -> Result<usize, Box<dyn Error>>
    where
        W: Write,
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
    /// fn progress(transferred: usize) {
    ///     println!("received {:?} bytes", transferred);
    /// }
    ///
    /// // Begin receiving the file
    /// portal.recv_file(&mut client, Some(verify_callback), Some(display_callback));
    /// ```
    pub fn recv_file<R>(
        &mut self,
        peer: &mut R,
        verify: Option<VerifyCallback>,
        display: Option<ProgressCallback>,
    ) -> Result<Metadata, Box<dyn Error>>
    where
        R: Read,
    {
        // Check that the key exists to confirm the handshake is complete
        let key = self.key.as_ref().ok_or(NoPeer)?;

        // Receive the metadata
        let metadata: Metadata = Protocol::read_encrypted_from(peer, key)?;

        // Attempt to convert the filename to valid utf8
        let name = match std::str::from_utf8(&metadata.filename) {
            Ok(s) => s,
            _ => return Err(NoneError.into()),
        };

        // Process the verify callback if applicable
        match verify
            .as_ref()
            .map_or(true, |c| c(&name, metadata.filesize))
        {
            true => {}
            false => return Err(Cancelled.into()),
        }

        // Map the region into memory for writing
        let mut mmap = self.map_writeable_file(&name, metadata.filesize)?;

        // Receive and decrypt the file
        match Protocol::read_encrypted_zero_copy(peer, &key, &mut mmap[..], display)? {
            x if x == metadata.filesize as usize => {}
            _ => return Err(Incomplete.into()),
        }
        Ok(metadata)
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

/*
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
} */
