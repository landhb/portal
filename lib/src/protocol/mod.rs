use crate::errors::PortalError::*;
use crate::ProgressCallback;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::convert::TryInto;
use std::error::Error;
use std::io::{Read, Write};

// Crypto
use hkdf::Hkdf;
use sha2::Sha256;
use spake2::{Ed25519Group, Spake2};

// Exchange message types
mod exchange;
pub use exchange::*;

// Encrypted message types
mod encrypted;
pub use encrypted::*;

#[cfg(test)]
mod tests;

/// Lower-level abstraction around the protocol. Use this
/// directly if you'd like more control than what the
/// higher-level Portal interface provides
pub struct Protocol;

/// An enum to describe the direction of each file transfer
/// participant (i.e Sender/Receiver)
#[derive(Serialize, Deserialize, PartialEq, Debug, Copy, Clone)]
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
    pub filename: Vec<u8>,
}

/// The wrapped message type for every exchanged message
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum PortalMessage {
    /// Provide enough information to the relay to pair
    /// you with the peer.
    Connect(ConnectMessage),

    /// SPAKE2 Key Derivation Information
    KeyExchange(PortalKeyExchange),

    /// SPAKE2 Key Confirmation Information
    Confirm(PortalConfirmation),

    /// All other messages are encrypted. This
    /// can be either metadata or a file chunk
    EncryptedDataHeader(EncryptedMessage),
}

impl PortalMessage {
    /// Send an arbitrary PortalMessage
    pub fn send<W: Write>(&mut self, writer: &mut W) -> Result<usize, Box<dyn Error>> {
        let data = bincode::serialize(&self).or(Err(SerializeError))?;
        writer.write_all(&data).or(Err(IOError))?;
        Ok(data.len())
    }

    /// Receive an arbitrary PortalMessage
    pub fn recv<R: Read>(reader: &mut R) -> Result<Self, Box<dyn Error>> {
        Ok(bincode::deserialize_from::<&mut R, PortalMessage>(reader)?)
    }

    /// Deserialize from existing data
    pub fn parse(data: &[u8]) -> Result<Self, Box<dyn Error>> {
        Ok(bincode::deserialize(&data)?)
    }
}

impl Protocol {
    /// Connect to a peer & receive the initial exchange data
    pub fn connect<P: Read + Write>(
        peer: &mut P,
        id: &str,
        direction: Direction,
        msg: PortalKeyExchange,
    ) -> Result<PortalKeyExchange, Box<dyn Error>> {
        // Send the connect message.
        let _ = PortalMessage::Connect(ConnectMessage {
            id: id.to_owned(),
            direction,
        })
        .send(peer)?;

        // Recv the peer's equivalent peering/connect message
        // TODO: currently nothing is done with this, however
        // this may be useful for future P2P protocols
        let _info = PortalMessage::recv(peer)?;

        // Send the exchange data
        let _ = PortalMessage::KeyExchange(msg).send(peer)?;

        // Recv the peer's data
        match PortalMessage::recv(peer).or(Err(IOError))? {
            PortalMessage::KeyExchange(data) => Ok(data.try_into().or(Err(BadMsg))?),
            _ => Err(Box::new(BadMsg)),
        }
    }

    /// Derive a shared key with the exchanged PortalConfirmation data.
    /// After this point in the exchange we have not verified that our peer
    /// has derived the same key as us, just derived the key for ourselves.
    pub fn derive_key(
        state: Spake2<Ed25519Group>,
        peer_data: &PortalKeyExchange,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        match state.finish(peer_data.into()) {
            Ok(res) => Ok(res),
            Err(_) => {
                return Err(BadMsg.into());
            }
        }
    }

    /// Use the derived session key to verify that our peer has derived
    /// the same key as us. After this the peer will be fully confirmed.
    pub fn confirm_peer<P: Read + Write>(
        peer: &mut P,
        id: &str,
        direction: Direction,
        key: &[u8],
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
        peer.write_all(&to_send)?;

        // Receive the peer's version
        peer.read_exact(&mut peer_msg)?;

        /// Helper method to compair arbitrary &[u8] slices, used internally
        /// to compare key confirmation data
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

    /// Read an encrypted owned & deserialize-able object from the peer.
    pub fn read_encrypted_from<R, D>(reader: &mut R, key: &[u8]) -> Result<D, Box<dyn Error>>
    where
        R: Read,
        D: DeserializeOwned,
    {
        // Create temporary storage for the object
        let mut storage = [0u8; 2048];

        // Receive the message into the storage region
        Protocol::read_encrypted_zero_copy(reader, key, &mut storage, None::<fn(usize)>)?;

        // Deserialize the result
        bincode::deserialize(&storage).or(Err(BadMsg.into()))
    }

    /// Read an encrypted message from the peer, writing the resulting
    /// decrypted data into the provided storage region. This allows for
    /// the ability to receive an encrypted chunk and decrypt it entirely
    /// in-place without extra copies.
    pub fn read_encrypted_zero_copy<R>(
        reader: &mut R,
        key: &[u8],
        storage: &mut [u8],
        callback: Option<ProgressCallback>,
    ) -> Result<usize, Box<dyn Error>>
    where
        R: Read,
    {
        // Receive the message header, return error if not EncryptedDataHeader
        let mut msg = match PortalMessage::recv(reader).or(Err(IOError))? {
            PortalMessage::EncryptedDataHeader(inner) => inner,
            _ => return Err(BadMsg.into()),
        };

        // Check that the storage region has enough room
        if storage.len() < msg.len {
            return Err(BufferTooSmall.into());
        }

        // Use the length field to read directly into the storage region
        let mut pos = 0;
        while pos < msg.len {
            match reader.read(&mut storage[pos..msg.len]) {
                Ok(0) => break,
                Ok(len) => {
                    pos += len;
                    callback.as_ref().map(|c| {
                        c(pos);
                    })
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e.into()),
            };
        }

        // Decrypt the region in-place
        msg.decrypt(key, &mut storage[..pos])
    }

    /// Encrypt & send an EncryptedDataHeader + the entire object to the peer
    pub fn encrypt_and_write_object<W, S>(
        writer: &mut W,
        key: &[u8],
        msg: &S,
    ) -> Result<usize, Box<dyn Error>>
    where
        W: Write,
        S: Serialize,
    {
        // Serialize the object
        let mut data = bincode::serialize(msg)?;

        // Encrypt the data
        let encmsg = EncryptedMessage::encrypt(key, &mut data)?;

        // Wrap and send the header
        PortalMessage::EncryptedDataHeader(encmsg).send(writer)?;

        // Send the data
        writer.write_all(&data).or(Err(IOError))?;

        Ok(data.len())
    }

    /// Encrypt & send the EncryptedDataHeader to the peer
    pub fn encrypt_and_write_header_only<W>(
        writer: &mut W,
        key: &[u8],
        data: &mut [u8],
    ) -> Result<usize, Box<dyn Error>>
    where
        W: Write,
    {
        // Encrypt the entire region in-place
        let header = EncryptedMessage::encrypt(key, data)?;

        // Send the EncryptedMessage header
        PortalMessage::EncryptedDataHeader(header).send(writer)
    }
}
