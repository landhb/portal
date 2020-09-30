use rand::Rng;
use chacha20poly1305::Nonce; 
use chacha20poly1305::aead::Aead;

use crate::file::{PortalFile,PortalChunk};

pub struct PortalChunks<'a, T: 'a> {
    v: &'a [T],
    chunk_size: usize,
    settings: &'a PortalFile,
}

impl<'a, T: 'a> PortalChunks<'a, T> {
    pub fn init(data: &'a [T], chunk_size: usize, settings: &'a PortalFile) -> PortalChunks<'a,T> {
        PortalChunks{
            v: data, // TODO: verify that this is zero-copy/move
            chunk_size: chunk_size,
            settings: settings,
        }
    }
}


impl<'a> Iterator for PortalChunks<'a,u8> 
{
    type Item = Vec<u8>; //&'a [u8];

    // The return type is `Option<T>`:
    //     * When the `Iterator` is finished, `None` is returned.
    //     * Otherwise, the next value is wrapped in `Some` and returned.
    fn next(&mut self) -> Option<Self::Item> {

        // return up to the next chunk size
        if self.v.is_empty() {
            return None;
        }

        let cipher = match self.settings {
            PortalFile::Immutable(inner) => &inner.state.cipher,
            PortalFile::Mutable(_) => {return None;},
        };

        let chunksz = std::cmp::min(self.v.len(), self.chunk_size);
        let (beg,end) = self.v.split_at(chunksz);

        // update next slice 
        self.v = end; 

        // Generate random nonce
        let mut rng = rand::thread_rng();
        let rbytes  = rng.gen::<[u8;12]>();
        let nonce = Nonce::from_slice(&rbytes); // 128-bits; unique per chunk

        // TODO: encrypt in place instead of returning new Vec
        let data = cipher.encrypt(nonce,beg).expect("encryption failure!");
        let chunk = PortalChunk {
            nonce: nonce.as_slice().to_owned(),
            data: data,
        };
        match bincode::serialize(&chunk) {
            Ok(v) => Some(v),
            Err(_) => None,
        } 
    }
} 
