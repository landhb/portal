use crate::errors::PortalError::*;
use serde::{Deserialize, Serialize};
use std::error::Error;

// Nonce generation
use rand::Rng;

// Encryption
#[cfg(not(feature = "ring-backend"))]
use chacha20poly1305::{aead::AeadInPlace, aead::NewAead, ChaCha20Poly1305, Key, Nonce, Tag};

#[cfg(feature = "ring-backend")]
use ring::aead::{Aad, LessSafeKey, Nonce, Tag, UnboundKey, CHACHA20_POLY1305};

#[cfg(feature = "ring-backend")]
use std::convert::TryInto;

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

#[cfg(not(feature = "ring-backend"))]
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

#[cfg(feature = "ring-backend")]
impl EncryptedMessage {
    /// Create an encrypted message out of an arbitrary serializable
    /// type
    pub fn encrypt(key: &[u8], data: &mut [u8]) -> Result<Self, Box<dyn Error>> {
        // Init state to send
        let mut state = Self::default();

        // Init the key
        let ring_key_chacha20 =
            LessSafeKey::new(UnboundKey::new(&CHACHA20_POLY1305, key).or(Err(CryptoError))?);

        // Generate random nonce
        let mut rng = rand::thread_rng();
        state.nonce = rng.gen::<[u8; 12]>();
        let ring_nonce = Nonce::assume_unique_for_key(state.nonce);

        // Set the length
        state.len = data.len();

        // Encrypt the data in-place. For ring we must append the tag after
        let tag = ring_key_chacha20
            .seal_in_place_separate_tag(ring_nonce, Aad::empty(), data)
            .or(Err(EncryptError))?;

        // Save the tag in our current state
        state.tag = tag.as_ref().try_into().or(Err(EncryptError))?;
        Ok(state)
    }

    /// Decrypt the provided data in-place
    pub fn decrypt(&mut self, key: &[u8], data: &mut [u8]) -> Result<usize, Box<dyn Error>> {
        // Init the key
        let ring_key_chacha20 =
            LessSafeKey::new(UnboundKey::new(&CHACHA20_POLY1305, key).or(Err(CryptoError))?);

        // The nonce & tag are self contained
        let ring_tag: Tag = self.tag.try_into().or(Err(DecryptError))?;
        let ring_nonce = Nonce::assume_unique_for_key(self.nonce);

        // Decrypt the data in place
        ring_key_chacha20
            .open_in_place_separate_tag(ring_nonce, Aad::empty(), ring_tag, data, 0..)
            .or(Err(DecryptError))?;

        Ok(data.len())
    }
}
