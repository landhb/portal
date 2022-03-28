use crate::errors::PortalError::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::error::Error;

// Nonce generation
use rand::Rng;

// Encryption
use chacha20poly1305::aead::NewAead;
use chacha20poly1305::{aead::AeadInPlace, ChaCha20Poly1305, Key, Nonce, Tag};

/// All encrypted messages must have associated state data (nonce, tag)
/// as well as the data itself
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
pub struct EncryptedMessage {
    pub nonce: [u8; 12], //Nonce,
    pub tag: [u8; 16],   //Tag,
    pub data: Vec<u8>,
}

impl EncryptedMessage {
    /// Create an encrypted message out of an arbitrary serializable
    /// type
    pub fn encrypt_and_serialize<S: Serialize>(
        key: &[u8],
        object: S,
    ) -> Result<Self, Box<dyn Error>> {
        // Init state to send
        let mut state = Self::default();

        // Generate random nonce
        let mut rng = rand::thread_rng();
        state.nonce = rng.gen::<[u8; 12]>();
        let nonce = Nonce::from_slice(&state.nonce);

        // Obtain the cipher from the key
        //let key = self.key.as_ref().ok_or(NoPeer)?;
        let cha_key = Key::from_slice(&key[..]);
        let cipher = ChaCha20Poly1305::new(cha_key);

        // Serialize all the metadata
        state.data = bincode::serialize(&object)?;

        // Encrypt the metadata in-place
        let tag = match cipher.encrypt_in_place_detached(nonce, b"", &mut state.data) {
            Ok(tag) => tag,
            Err(_e) => return Err(EncryptError.into()),
        };
        state.tag = tag.into();
        Ok(state)
    }

    /// Decrypt and deserialize the contained object from the data in this instance
    pub fn decrypt_and_deserialize<D: DeserializeOwned + Sized>(
        &mut self,
        key: &[u8],
    ) -> Result<D, Box<dyn Error>> {
        // Obtain the cipher from the key
        let cha_key = Key::from_slice(&key[..]);
        let cipher = ChaCha20Poly1305::new(cha_key);

        // The nonce & tag are self contained
        let nonce = Nonce::from_slice(&self.nonce);
        let tag = Tag::from_slice(&self.tag);

        // Decrypt the data in place
        match cipher.decrypt_in_place_detached(&nonce, b"", &mut self.data, &tag) {
            Ok(_) => {}
            Err(_e) => return Err(DecryptError.into()),
        }

        // Return the deserialized object
        bincode::deserialize(&self.data).or(Err(BadMsg.into()))
    }
}
