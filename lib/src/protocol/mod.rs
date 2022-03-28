use crate::errors::PortalError::*;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::io::{Read, Write};

// Crypto
use chacha20poly1305::{Nonce, Tag};
use hkdf::Hkdf;
use sha2::{Digest, Sha256};
use spake2::{Ed25519Group, Identity, Password, Spake2};

mod confirmation;
pub use confirmation::*;

mod encrypted;
pub use encrypted::*;

/// Lower-level abstraction around the protocol. Use this
/// directly if you'd like more control than what the
/// higher-level Portal interface provides
pub struct Protocol;

/// An enum to describe the direction of each file transfer
/// participant (i.e Sender/Receiver)
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum Direction {
    Sender,
    Receiver,
}

/// Information to correlate
/// connections on the relay
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct ConnectMessage {
    pub id: String,
    pub direction: Direction,
}

/// Metadata about the transfer to be exchanged
/// between peers after key derivation (encrypted)
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
pub struct Metadata {
    pub filesize: u64,
    pub filename: Option<Vec<u8>>,
}

/// The wrapped message type for every exchanged message
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum PortalMessage {
    /// Provide enough information to the relay to pair
    /// you with the peer.
    Connect(ConnectMessage),

    /// SPAKE2 Key Derivation Information
    KeyExchange(PortalConfirmation),

    /// All other messages are encrypted. This
    /// can be either metadata or a file chunk
    EncryptedData(EncryptedMessage),
}

impl PortalMessage {
    /// Send an arbitrary PortalMessage
    pub fn send<W: Write>(&mut self, mut writer: W) -> Result<usize, Box<dyn Error>> {
        let data = bincode::serialize(&self).or(Err(SerializeError))?;
        writer.write_all(&data).or(Err(IOError))?;
        Ok(data.len())
    }

    /// Receive an arbitrary PortalMessage
    pub fn recv<R: Read>(mut reader: R) -> Result<Self, Box<dyn Error>> {
        Ok(bincode::deserialize_from::<R, PortalMessage>(reader)?)
    }

    /// Deserialize from existing data
    pub fn parse(data: &[u8]) -> Result<Self, Box<dyn Error>> {
        Ok(bincode::deserialize(&data)?)
    }
}

impl Protocol {
    /// Connect to a peer & receive the initial message
    pub fn connect<P: Read + Write>(
        mut peer: P,
        id: String,
        direction: Direction,
    ) -> Result<(), Box<dyn Error>> {
        // Send the connect message
        let sent = PortalMessage::Connect(ConnectMessage { id, direction }).send(peer)?;

        // Recv the peer's equivalent message
        // TODO: currently nothing is done with this, however
        // this may be useful for future P2P protocols
        let _info = match PortalMessage::recv(peer) {};
        Ok(())
    }

    /// Derive a shared key with the exchanged PortalConfirmation data.
    /// After this point in the exchange we have not verified that our peer
    /// has derived the same key as us, just derived the key for ourselves.
    pub fn derive_key(
        state: Spake2<Ed25519Group>,
        peer_data: &PortalConfirmation,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        match state.finish(peer_data) {
            Ok(res) => Ok(res),
            Err(_) => {
                return Err(BadMsg.into());
            }
        }
    }

    /// Use the derived session key to verify that our peer has derived
    /// the same key as us. After this the peer will be fully confirmed.
    pub fn confirm_peer<P: Read + Write>(
        id: &str,
        direction: Direction,
        key: &[u8],
        mut client: P,
    ) -> Result<(), Box<dyn Error>> {
        // Arbitrary info that both sides can derive
        let sender_info = format!("{}-{}", id, "senderinfo");
        let receiver_info = format!("{}-{}", id, "receiverinfo");

        // Perform key confirmation step via HKDF
        let h = Hkdf::<Sha256>::new(None, key);
        let mut peer_msg = [0u8; 42];
        let mut sender_confirm = [0u8; 42];
        let mut receiver_confirm = [0u8; 42];
        h.expand(&sender_info.as_bytes(), &mut sender_confirm)
            .unwrap();
        h.expand(&receiver_info.as_bytes(), &mut receiver_confirm)
            .unwrap();

        // Determine our vs their message based on direction
        let (to_send, expected) = match direction {
            Direction::Sender => (sender_confirm, receiver_confirm),
            Direction::Receiver => (receiver_confirm, sender_confirm),
        };

        // Send our data
        client.write_all(&to_send)?;

        // Receive the peer's version
        client.read_exact(&mut peer_msg)?;

        /// Helper method to compair arbitrary &[u8] slices, used internally
        /// to compare key exchange and derivation data
        fn compare_key_derivations(a: &[u8], b: &[u8]) -> std::cmp::Ordering {
            for (ai, bi) in a.iter().zip(b.iter()) {
                match ai.cmp(&bi) {
                    std::cmp::Ordering::Equal => continue,
                    ord => return ord,
                }
            }

            // if every single element was equal, compare length
            a.len().cmp(&b.len())
        }

        // Compare their version to the expected result
        if compare_key_derivations(&peer_msg, &expected) != std::cmp::Ordering::Equal {
            return Err(BadMsg.into());
        }

        // If they match, the peer is confirmed
        Ok(())
    }

    /// Read an encrypted metadata message from the peer
    pub fn read_metadata_from<R: Read>(
        &mut self,
        mut reader: R,
    ) -> Result<Metadata, Box<dyn Error>> {
        unimplemented!()
    }

    /// Write an encrypted metadata message to the peer
    pub fn write_metadata_to<W: Write>(&mut self, mut writer: W) -> Result<usize, Box<dyn Error>> {
        unimplemented!()
    }
}
