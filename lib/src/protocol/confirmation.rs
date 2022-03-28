use serde::{Deserialize, Serialize};

/// A data format exchanged by each peer to derive
/// the shared session key
pub type PortalConfirmation = [u8; 33];

/*
* Receive the bytes necessary for a confirmation message
* from a stream that implements std::io::Read, consuming the bytes
*
pub fn read_confirmation_from<R>(mut reader: R) -> Result<PortalConfirmation, Box<dyn Error>>
where
    R: std::io::Read,
{
    let mut res: PortalConfirmation = [0u8; 33];
    reader.read_exact(&mut res)?;
    Ok(res)
}*/
