use crate::errors::PortalError::*;
use serde::{Deserialize, Serialize};
use std::error::Error;

// Nonce generation
use rand::Rng;

// Encryption
use chacha20poly1305::aead::NewAead;
use chacha20poly1305::{aead::AeadInPlace, ChaCha20Poly1305, Key, Nonce, Tag};

/// All encrypted messages must have associated state data (nonce, tag)
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
pub struct EncryptedMessage {
    /// Provides storage for chacha20poly1305::Nonce
    pub nonce: [u8; 12],
    /// Provides storage for chacha20poly1305::Tag
    pub tag: [u8; 16],
    /// Length of follow-on data. Data is not owned
    /// directly to prevent copies
    pub len: usize,
}

impl EncryptedMessage {
    /// Create an encrypted message out of an arbitrary serializable
    /// type
    pub fn encrypt(key: &[u8], data: &mut [u8]) -> Result<Self, Box<dyn Error>> {
        // Init state to send
        let mut state = Self::default();

        // Generate random nonce
        let mut rng = rand::thread_rng();
        state.nonce = rng.gen::<[u8; 12]>();
        let nonce = Nonce::from_slice(&state.nonce);

        // Obtain the cipher from the key
        let cha_key = Key::from_slice(&key[..]);
        let cipher = ChaCha20Poly1305::new(cha_key);

        // Set the length
        state.len = data.len();

        // Encrypt the data in-place
        let tag = cipher
            .encrypt_in_place_detached(nonce, b"", data)
            .or(Err(EncryptError))?;

        // Save the tag in our current state
        state.tag = tag.into();
        Ok(state)
    }

    /// Decrypt the provided data in-place
    pub fn decrypt(&mut self, key: &[u8], data: &mut [u8]) -> Result<usize, Box<dyn Error>> {
        // Obtain the cipher from the key
        let cha_key = Key::from_slice(&key[..]);
        let cipher = ChaCha20Poly1305::new(cha_key);

        // The nonce & tag are self contained
        let nonce = Nonce::from_slice(&self.nonce);
        let tag = Tag::from_slice(&self.tag);

        // Decrypt the data in place
        cipher
            .decrypt_in_place_detached(&nonce, b"", data, &tag)
            .or(Err(DecryptError))?;

        Ok(data.len())
    }
}
