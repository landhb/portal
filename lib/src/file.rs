//! Provides an interface into the PortalFile abstraction
//!

use anyhow::Result;
use chacha20poly1305::ChaCha20Poly1305;
use chacha20poly1305::{aead::AeadInPlace, Nonce, Tag};
use memmap::MmapMut;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::io::Write;

use crate::chunks::PortalChunks;
use crate::errors::PortalError;

/**
 * A file mapping that contains state to
 * encrypt/decrypt files in memory
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
#[derive(Serialize, Deserialize, PartialEq, Default, Debug)]
pub struct StateMetadata {
    pub nonce: Vec<u8>,
    pub tag: Vec<u8>,
}

impl PortalFile {
    pub fn init(mmap: MmapMut, cipher: ChaCha20Poly1305) -> PortalFile {
        PortalFile {
            mmap,
            cipher,
            pos: 0,
            state: StateMetadata {
                nonce: Vec::new(),
                tag: Vec::new(),
            },
        }
    }

    /**
     * Encrypts the current PortalFile, by encrypting the mmap'd memory in-place
     */
    pub fn encrypt(&mut self) -> Result<()> {
        // Generate random nonce
        let mut rng = rand::thread_rng();
        let rbytes = rng.gen::<[u8; 12]>();
        let nonce = Nonce::from_slice(&rbytes); // 128-bits; Used once for entire file
        self.state.nonce.extend(nonce);

        let tag = match self
            .cipher
            .encrypt_in_place_detached(nonce, b"", &mut self.mmap[..])
        {
            Ok(tag) => tag,
            Err(_e) => return Err(PortalError::EncryptError.into()),
        };
        self.state.tag.extend(tag);
        Ok(())
    }

    /**
     * Decrypts the current PortalFile, by decrypting the mmap'd memory in-place
     */
    pub fn decrypt(&mut self) -> Result<()> {
        // Verify nonce & tag lengths
        if self.state.nonce.len() != std::mem::size_of::<Nonce>()
            || self.state.tag.len() != std::mem::size_of::<Tag>()
        {
            return Err(PortalError::DecryptError.into());
        }

        let nonce = Nonce::from_slice(&self.state.nonce);
        let tag = Tag::from_slice(&self.state.tag);
        match self
            .cipher
            .decrypt_in_place_detached(nonce, b"", &mut self.mmap[..], &tag)
        {
            Ok(_) => Ok(()),
            Err(_e) => Err(PortalError::DecryptError.into()),
        }
    }

    /**
     * Writes the nonce and tag for this file to the provided writer. Use
     * after encrypting a file to communicate state data to the peer that will
     * decrypt the file
     */
    pub fn sync_file_state<W>(&mut self, writer: &mut W) -> Result<usize>
    where
        W: std::io::Write,
    {
        let data: Vec<u8> = bincode::serialize(&self.state)?;
        writer.write_all(&data)?;
        Ok(data.len())
    }

    /**
     * Downloads a file, first by retrieving the Tag and Nonce communicated by
     * sync_file_state() and then reading in the file until EOF
     *
     * ```ignore
     * Peer A                  Peer B
     * encrypt()               download_file()
     * sync_file_state()       decrypt()
     * // send chunks
     * ```
     */
    pub fn download_file<R, F>(&mut self, mut reader: R, callback: F) -> Result<u64>
    where
        R: std::io::Read,
        F: Fn(u64),
    {
        let mut buf = vec![0u8; crate::CHUNK_SIZE];

        // First deserialize the Nonce + Tag
        let remote_state: StateMetadata = bincode::deserialize_from(&mut reader)?;
        self.state.nonce.extend(&remote_state.nonce);
        self.state.tag.extend(&remote_state.tag);

        // Anything else is file data
        loop {
            let len = match reader.read(&mut buf) {
                Ok(0) => {
                    return Ok(self.pos as u64);
                }
                Ok(len) => len,
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e.into()),
            };

            (&mut self.mmap[self.pos..]).write_all(&buf[..len])?;
            self.pos += len;
            callback(self.pos as u64);
        }
    }

    /**
     * Returns an iterator over the chunks to send it over the
     * network
     *
     * # Examples
     *     
     * ```ignore
     * for data in file.get_chunks(portal::CHUNK_SIZE) {
     *      client.write_all(&data)?
     *      total += data.len();
     *      pb.set_position(total as u64);
     * }
     * ```
     */
    pub fn get_chunks<'a>(
        &'a self,
        chunk_size: usize,
    ) -> impl std::iter::Iterator<Item = &'a [u8]> {
        PortalChunks::init(&self.mmap[..], chunk_size)
    }

    /**
     * Writes the provided data to the file in-memory at the current position
     */
    pub fn write_given_chunk(&mut self, data: &[u8]) -> Result<u64> {
        (&mut self.mmap[self.pos..]).write_all(&data)?;
        self.pos += data.len();
        Ok(data.len() as u64)
    }
}

#[cfg(test)]
pub mod tests {
    use crate::errors::PortalError;
    use crate::{Direction, Portal};
    use std::io::{Read, Write};

    pub struct MockTcpStream {
        pub data: Vec<u8>,
    }

    impl Read for MockTcpStream {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
            let size: usize = std::cmp::min(self.data.len(), buf.len());
            buf[..size].copy_from_slice(&self.data[..size]);
            self.data.drain(0..size);
            Ok(size)
        }
    }

    impl Write for MockTcpStream {
        fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
            self.data.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> Result<(), std::io::Error> {
            Ok(())
        }
    }

    #[test]
    fn test_failed_decryption() {
        // receiver
        let dir = Direction::Receiver;
        let pass = "test".to_string();
        let (mut receiver, receiver_msg) = Portal::init(dir, "id".to_string(), pass, None);

        // sender
        let dir = Direction::Sender;
        let pass = "test".to_string();
        let (mut sender, sender_msg) = Portal::init(dir, "id".to_string(), pass, None);

        // we need a key to be able to encrypt
        receiver.derive_key(sender_msg.as_slice()).unwrap();
        sender.derive_key(receiver_msg.as_slice()).unwrap();

        // encrypt the file
        let mut file = sender.load_file("/etc/passwd").unwrap();
        file.encrypt().unwrap();

        // Test incorrect tag length path
        let old_tag = file.state.tag.clone();
        file.state.tag.truncate(0);
        let result = file.decrypt();
        assert!(result.is_err());
        let _ = result.map_err(|e| match e.downcast_ref::<PortalError>() {
            Some(PortalError::DecryptError) => anyhow::Ok(()),
            _ => panic!("Unexpected error"),
        });

        // Test failed decryption path
        file.state.tag = old_tag;
        file.state.tag[0] += 1; // alter tag
        let result = file.decrypt();
        assert!(result.is_err());
        let _ = result.map_err(|e| match e.downcast_ref::<PortalError>() {
            Some(PortalError::DecryptError) => anyhow::Ok(()),
            _ => panic!("Unexpected error"),
        });
    }

    #[test]
    fn test_sync_file_download_file() {
        // receiver
        let dir = Direction::Receiver;
        let pass = "test".to_string();
        let (mut receiver, receiver_msg) = Portal::init(dir, "id".to_string(), pass, None);

        // sender
        let dir = Direction::Sender;
        let pass = "test".to_string();
        let (mut sender, sender_msg) = Portal::init(dir, "id".to_string(), pass, None);

        // we need a key to be able to encrypt
        receiver.derive_key(sender_msg.as_slice()).unwrap();
        sender.derive_key(receiver_msg.as_slice()).unwrap();

        // encrypt the file
        let mut file = sender.load_file("/etc/passwd").unwrap();
        file.encrypt().unwrap();

        let mut stream = MockTcpStream {
            data: Vec::with_capacity(crate::CHUNK_SIZE),
        };

        // communicate the necessary state info
        // for the peer to be able to decrypt the file
        file.sync_file_state(&mut stream).unwrap();

        // send the file over the stream
        for data in file.get_chunks(crate::CHUNK_SIZE) {
            stream.write(&data).unwrap();
        }

        // use download_file to read in the file data
        let mut new_file = receiver
            .create_file("/tmp/passwd", file.mmap[..].len() as u64)
            .unwrap();
        new_file
            .download_file(&mut stream, |x| println!("{:?}", x))
            .unwrap();

        // compare the state of the two files
        assert_eq!(&file.state.tag, &new_file.state.tag);
        assert_eq!(&file.state.nonce, &new_file.state.nonce);
        assert_eq!(&file.mmap[..], &new_file.mmap[..]);

        new_file.decrypt().unwrap(); // should not panic
        stream.flush().unwrap(); // just for coverage reporting, does nothing
    }

    #[test]
    fn test_encrypt_decrypt() {
        // receiver
        let dir = Direction::Receiver;
        let pass = "test".to_string();
        let (mut receiver, receiver_msg) = Portal::init(dir, "id".to_string(), pass, None);

        // sender
        let dir = Direction::Sender;
        let pass = "test".to_string();
        let (mut sender, sender_msg) = Portal::init(dir, "id".to_string(), pass, None);

        // we need a key to be able to encrypt
        receiver.derive_key(sender_msg.as_slice()).unwrap();
        sender.derive_key(receiver_msg.as_slice()).unwrap();

        let mut file = sender.load_file("/etc/passwd").unwrap();

        let file_before = String::from_utf8((&file.mmap[..]).to_vec());
        file.encrypt().unwrap();
        let file_encrypted = String::from_utf8((&file.mmap[..]).to_vec());
        file.decrypt().unwrap();
        let file_after = String::from_utf8((&file.mmap[..]).to_vec());

        assert_ne!(file_before, file_encrypted);
        assert_eq!(file_before, file_after);
    }
}
