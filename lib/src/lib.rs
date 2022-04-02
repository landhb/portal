//! A small Protocol Library for [Portal](https://github.com/landhb/portal) - An encrypted file transfer utility
//!
//! This crate enables a consumer to:
//!
//! - Create/serialize/deserialize Portal request/response messages.
//! - Negoticate a symmetric key with a peer using [SPAKE2](https://docs.rs/spake2/0.2.0/spake2)
//! - Encrypt files with [Chacha20-Poly1305](https://blog.cloudflare.com/it-takes-two-to-chacha-poly/) using either the
//!     [RustCrypto](https://docs.rs/chacha20poly1305) implementation or [Ring's](https://briansmith.org/rustdoc/ring/aead/index.html)
//! - Send/receive files through a Portal relay
//!
//! The library is broken up into two abstractions:
//!
//! - A higher level API, exposted via the `Portal` struct, to facilitate automating transfers easily
//! - A lower level API, exposed via the `protocol::Protocol` struct, if you need access to lower-level facilities
use memmap::{MmapMut, MmapOptions};
use std::convert::TryInto;
use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

// Key Exchange
use sha2::{Digest, Sha256};
use spake2::{Ed25519Group, Identity, Password, Spake2};

#[cfg(test)]
mod tests;

// Allow users to access errors
pub mod errors;
use errors::PortalError::*;

/// Lower level protocol methods. Use these
/// if the higher-level Portal interface is
/// too abstract.
pub mod protocol;
pub use protocol::*;

/**
 * Arbitrary port for the Portal protocol
 */
pub const DEFAULT_PORT: u16 = 13265;

/**
 * Default chunk size
 */
pub const CHUNK_SIZE: usize = 65536;

/// None constant for optional verify callbacks - Helper
pub const NO_VERIFY_CALLBACK: Option<fn(&TransferInfo) -> bool> = None::<fn(&TransferInfo) -> bool>;

/// None constant for optional progress callbacks - Helper
pub const NO_PROGRESS_CALLBACK: Option<fn(usize)> = None::<fn(usize)>;

/**
 * The primary interface into the library.
 */
#[derive(PartialEq, Debug)]
pub struct Portal {
    // Information to correlate
    // connections on the relay
    pub(crate) id: String,
    pub(crate) direction: Direction,

    // KeyExchange information
    pub(crate) exchange: PortalKeyExchange,

    // A nonce sequence that must be used for
    // the entire session to ensure no re-use
    nseq: NonceSequence,

    // Crypto state used to derive the key
    // once we receive a confirmation msg from the peer
    pub(crate) state: Option<Spake2<Ed25519Group>>,

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
    /// let portal = Portal::init(Direction::Receiver, id, password).unwrap();
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
            nseq: NonceSequence::new(),
            state: Some(s1),
            key: None,
        })
    }

    /// Negotiate a secure connection over the insecure channel by performing the portal
    /// handshake. Subsequent communication will be encrypted.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::net::TcpStream;
    /// use portal_lib::{Portal,Direction};
    ///
    /// let mut portal = Portal::init(Direction::Sender, "id".into(), "password".into()).unwrap();
    /// let mut stream = TcpStream::connect("127.0.0.1:34254").unwrap();
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
        let state = self.state.take().ok_or(BadState)?;

        // Derive the session key
        let key = Protocol::derive_key(state, &confirm).or(Err(BadMsg))?;

        // confirm that the peer has the same key
        Protocol::confirm_peer(peer, &self.id, self.direction, &key)?;

        // Set key for further use
        self.key = Some(key);
        Ok(())
    }

    /// As the sender, communicate a TransferInfo struct to the receiver
    /// so that they may confirm/deny the transfer. Returns an iterator
    /// over the fullpath + Metadata to pass to send_file(). Allows the user
    /// to send multiple files in one session.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::path::Path;
    /// use std::error::Error;
    /// use std::net::TcpStream;
    /// use portal_lib::{Portal, Direction, TransferInfoBuilder};
    ///
    /// fn my_send() -> Result<(), Box<dyn Error>> {
    ///     // Securely generate/exchange ID & Password with peer out-of-band
    ///     let id = String::from("id");
    ///     let password = String::from("password");
    ///
    ///     // Connect to the relay
    ///     let mut portal = Portal::init(Direction::Sender,"id".into(), "password".into())?;
    ///     let mut stream = TcpStream::connect("127.0.0.1:34254")?;
    ///
    ///     // The handshake must be performed first, otherwise
    ///     // there is no shared key to encrypt the file with
    ///     portal.handshake(&mut stream)?;
    ///
    ///     // Add any files/directories
    ///     let info = TransferInfoBuilder::new()
    ///         .add_file(Path::new("/etc/passwd"))?
    ///         .finalize();
    ///
    ///     // Optional: implement a custom callback to display how much
    ///     // has been transferred
    ///     fn progress(transferred: usize) {
    ///         println!("sent {:?} bytes", transferred);
    ///     }
    ///
    ///     // Send every file in TransferInfo
    ///     for (fullpath, metadata) in portal.outgoing(&mut stream, &info)? {
    ///         portal.send_file(&mut stream, fullpath, Some(progress))?;
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn outgoing<'a, W>(
        &mut self,
        peer: &mut W,
        info: &'a TransferInfo,
    ) -> Result<impl Iterator<Item = (&'a PathBuf, &'a Metadata)>, Box<dyn Error>>
    where
        W: Write,
    {
        // Check that the key exists to confirm the handshake is complete
        let key = self.key.as_ref().ok_or(NoPeer)?;

        // Send all TransferInfo for peer to confirm
        Protocol::encrypt_and_write_object(peer, key, &mut self.nseq, info)?;

        // Return an iterator that returns metadata for each outgoing file
        Ok(info.localpaths.iter().zip(info.all.iter()))
    }

    /// As the receiver, receive a TransferInfo struct which will be passed
    /// to your optional verify callback. And may be used to confirm/deny
    /// the transfer. Returns an iterator over the Metadata of incoming files.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::path::Path;
    /// use std::error::Error;
    /// use std::net::TcpStream;
    /// use portal_lib::{Portal, Direction, TransferInfo};
    ///
    /// fn my_recv() -> Result<(), Box<dyn Error>> {
    ///
    ///     // Securely generate/exchange ID & Password with peer out-of-band
    ///     let id = String::from("id");
    ///     let password = String::from("password");
    ///
    ///     // Connect to the relay
    ///     let mut portal = Portal::init(Direction::Sender, id, password)?;
    ///     let mut stream = TcpStream::connect("127.0.0.1:34254")?;
    ///
    ///     // The handshake must be performed first, otherwise
    ///     // there is no shared key to encrypt the file with
    ///     portal.handshake(&mut stream)?;
    ///
    ///     // Optional: User callback to confirm/deny a transfer. If
    ///     // none is provided, this will default accept the incoming file.
    ///     // Return true to accept, false to reject the transfer.
    ///     fn confirm_download(_info: &TransferInfo) -> bool { true }
    ///
    ///     // Optional: implement a custom callback to display how much
    ///     // has been transferred
    ///     fn progress(transferred: usize) {
    ///         println!("received {:?} bytes", transferred);
    ///     }
    ///
    ///     // Decide where downloads should go
    ///     let my_downloads = Path::new("/tmp");
    ///
    ///     // Receive every file in TransferInfo
    ///     for metadata in portal.incoming(&mut stream, Some(confirm_download))? {
    ///         portal.recv_file(&mut stream, my_downloads, Some(&metadata), Some(progress))?;
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn incoming<R, V>(
        &mut self,
        peer: &mut R,
        verify: Option<V>,
    ) -> Result<impl Iterator<Item = Metadata>, Box<dyn Error>>
    where
        R: Read,
        V: Fn(&TransferInfo) -> bool,
    {
        // Check that the key exists to confirm the handshake is complete
        let key = self.key.as_ref().ok_or(NoPeer)?;

        // Receive the TransferInfo
        let info: TransferInfo = Protocol::read_encrypted_from(peer, key)?;

        // Process the verify callback if applicable
        match verify.as_ref().map_or(true, |c| c(&info)) {
            true => {}
            false => return Err(Cancelled.into()),
        }

        // Return an iterator that returns metadata for each incoming file
        Ok(info.all.into_iter())
    }

    /// Send a given file over the portal. Must be called after performing the
    /// handshake or this method will return an error.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::path::Path;
    /// use std::net::TcpStream;
    /// use portal_lib::{Portal,Direction};
    ///
    /// let mut portal = Portal::init(Direction::Sender,"id".into(), "password".into()).unwrap();
    /// let mut stream = TcpStream::connect("127.0.0.1:34254").unwrap();
    ///
    /// // The handshake must be performed first, otherwise
    /// // there is no shared key to encrypt the file with
    /// portal.handshake(&mut stream);
    ///
    /// // Optional: implement a custom callback to display how much
    /// // has been transferred
    /// fn progress(transferred: usize) {
    ///     println!("sent {:?} bytes", transferred);
    /// }
    ///
    /// // Begin sending the file
    /// let file = Path::new("/etc/passwd").to_path_buf();
    /// portal.send_file(&mut stream, &file, Some(progress));
    /// ```
    pub fn send_file<W, D>(
        &mut self,
        peer: &mut W,
        path: &PathBuf,
        callback: Option<D>,
    ) -> Result<usize, Box<dyn Error>>
    where
        W: Write,
        D: Fn(usize),
    {
        // Check that the key exists to confirm the handshake is complete
        let key = self.key.as_ref().ok_or(NoPeer)?;

        // Obtain the file name stub from the path
        let filename = path
            .file_name()
            .ok_or(BadFileName)?
            .to_str()
            .ok_or(BadFileName)?;

        // Map the file into memory
        let mut mmap = self.map_readable_file(&path)?;

        // Create the metatada object
        let metadata = Metadata {
            filesize: mmap.len() as u64,
            filename: filename.to_string(),
        };

        // Write the file metadata over the encrypted channel
        Protocol::encrypt_and_write_object(peer, key, &mut self.nseq, &metadata)?;

        // Send the encrypted region in chunks
        let mut total_sent = 0;
        for chunk in mmap[..].chunks_mut(CHUNK_SIZE) {
            // Encrypt the chunk in-place & send the header
            Protocol::encrypt_and_write_header_only(peer, key, &mut self.nseq, chunk)?;

            // Write the entire chunk
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
    /// ```no_run
    /// use std::path::Path;
    /// use std::net::TcpStream;
    /// use portal_lib::{Portal,Direction};
    ///
    /// let mut portal = Portal::init(Direction::Sender,"id".into(), "password".into()).unwrap();
    /// let mut stream = TcpStream::connect("127.0.0.1:34254").unwrap();
    ///
    /// // The handshake must be performed first, otherwise
    /// // there is no shared key to encrypt the file with
    /// portal.handshake(&mut stream);
    ///
    /// // Optional: implement a custom callback to display how much
    /// // has been transferred
    /// fn progress(transferred: usize) {
    ///     println!("received {:?} bytes", transferred);
    /// }
    ///
    /// // Begin receiving the file into /tmp
    /// portal.recv_file(&mut stream, Path::new("/tmp"), None, Some(progress));
    /// ```
    pub fn recv_file<R, D>(
        &mut self,
        peer: &mut R,
        outdir: &Path,
        expected: Option<&Metadata>,
        display: Option<D>,
    ) -> Result<Metadata, Box<dyn Error>>
    where
        R: Read,
        D: Fn(usize),
    {
        // Check that the key exists to confirm the handshake is complete
        let key = self.key.as_ref().ok_or(NoPeer)?;

        // Verify the outdir is valid
        if !outdir.is_dir() {
            return Err(BadDirectory.into());
        }

        // Receive the metadata
        let metadata: Metadata = Protocol::read_encrypted_from(peer, key)?;

        // Verify the metadata is expected, if a comparison is provided
        if expected.map_or(false, |exp| metadata != *exp) {
            return Err(BadMsg.into());
        }

        // Ensure the filename is only the name component
        let path = match Path::new(&metadata.filename).file_name() {
            Some(s) => outdir.join(s),
            _ => return Err(BadFileName.into()),
        };

        // Map the region into memory for writing
        let mut mmap = self.map_writeable_file(&path, metadata.filesize)?;

        let mut total = 0;
        for chunk in mmap[..].chunks_mut(CHUNK_SIZE) {
            // Receive the entire chunk in-place
            Protocol::read_encrypted_zero_copy(peer, &key, chunk)?;

            // Increment and optionally invoke callback
            total += chunk.len();
            display.as_ref().map(|c| {
                c(total);
            });
        }

        // Check for incomplete transfers
        if total != metadata.filesize as usize {
            return Err(Incomplete.into());
        }
        Ok(metadata)
    }

    /// Helper: mmap's a file into memory for reading
    fn map_readable_file<'a>(&'a self, f: &PathBuf) -> Result<MmapMut, Box<dyn Error>> {
        let file = File::open(f)?;
        let mmap = unsafe { MmapOptions::new().map_copy(&file)? };
        Ok(mmap)
    }

    /// Helper: mmap's a file into memory for writing
    fn map_writeable_file<'a>(&'a self, f: &PathBuf, size: u64) -> Result<MmapMut, Box<dyn Error>> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&f)?;

        file.set_len(size)?;
        let mmap = unsafe { MmapOptions::new().map_mut(&file)? };
        Ok(mmap)
    }

    /// Returns a copy of the Portal::Direction associated with
    /// this Portal request
    pub fn get_direction(&self) -> Direction {
        self.direction.clone()
    }

    /// Sets the Portal::Direction associated with this Poral request
    pub fn set_direction(&mut self, direction: Direction) {
        self.direction = direction;
    }

    /// Returns a reference to the ID associated with this
    /// Portal request
    pub fn get_id(&self) -> &String {
        &self.id
    }

    /// Sets the ID associated with this Poral request
    pub fn set_id(&mut self, id: String) {
        self.id = id;
    }

    /// Returns a reference to the key associated with this
    /// Portal request
    pub fn get_key(&self) -> &Option<Vec<u8>> {
        &self.key
    }

    /// Sets the ID associated with this Poral request
    pub fn set_key(&mut self, key: Vec<u8>) {
        self.key = Some(key);
    }
}
