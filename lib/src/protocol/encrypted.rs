use serde::{Deserialize, Serialize};

/// All encrypted messages must have associated state data (nonce, tag)
/// as well as the data itself
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct EncryptedMessage {
    pub nonce: [u8; 12], //Nonce,
    pub tag: [u8; 16],   //Tag,
    pub data: Vec<u8>,
}

impl EncryptedMessage {
    /// Create an encrypted message out of an arbitrary serializable
    /// type
    pub fn new<S: Serialize>(object: S) -> Self {
        unimplemented!()
    }
}
