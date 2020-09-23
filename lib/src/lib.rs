use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::fs::File;
use memmap::Mmap;

pub mod errors;

//use errors::PortalError;


/**
 * The primary interface into the library
 */
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Portal {
    direction: Option<Direction>,
    id: Option<String>,
    pubkey: Option<String>,
}


#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum Direction {
    Sender,
    Reciever,
}

pub struct PortalFile {
    mmap: Mmap,
}


#[derive(Debug)]
pub struct PortalChunks<'a, T: 'a> {
    v: &'a [T],
    chunk_size: usize,
    settings: &'a Portal,
}


impl<'a,T> Iterator for PortalChunks<'a,T> 
where T:Copy 
{
    type Item = &'a [T];

    // The return type is `Option<T>`:
    //     * When the `Iterator` is finished, `None` is returned.
    //     * Otherwise, the next value is wrapped in `Some` and returned.
    fn next(&mut self) -> Option<Self::Item> {

        // return up to the next chunk size
        if self.v.is_empty() {
            return None;
        } 

        let chunksz = std::cmp::min(self.v.len(), self.chunk_size);
        let (beg,end) = self.v.split_at(chunksz);

        // update next slice 
        self.v = end; 

        // TODO: encrypt current slice

        // return current slice
        Some(beg)
               
    }
} 



impl Portal {
    
    /**
     * Initialize 
     */
    pub fn init(direction: Option<Direction>, pubkey: Option<String>) -> Portal {
        Portal {
            direction: direction,
            id: None,
            pubkey: pubkey,
        }
    }

    /**
     * Construct from data 
     */
    pub fn parse(data: Vec<u8>) -> Result<Portal> {
        Ok(bincode::deserialize(&data)?)
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        Ok(bincode::serialize(&self)?)
    }

    pub fn get_id(&self) -> Option<String> {
        self.id.clone()
    }

    pub fn get_pubkey(&self) -> Option<String> {
        self.pubkey.clone()
    }

    pub fn get_direction(&self) -> Option<Direction> {
        self.direction.clone()
    }

    pub fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }

    pub fn set_direction(&mut self, direction: Option<Direction>) {
        self.direction = direction;
    }

    pub fn set_pubkey(&mut self, pubkey: Option<String>){
        self.pubkey = pubkey;
    }

    /*
     * mmap's a file into memory 
     */
    pub fn load_file<'a>(&'a self, f: &str) -> Result<PortalFile>  {
        let file = File::open(f)?;
        let mmap = unsafe { Mmap::map(&file)?  };
        Ok(PortalFile{mmap: mmap})
    }

    /**
     * Returns an iterator over the chunks to send it over the
     * network
     */
    pub fn get_chunks<'a>(&'a self, data: &'a PortalFile, chunk_size: usize) -> Result<PortalChunks<'a,u8>> {
        Ok(PortalChunks{
            v: &data.mmap[..], // TODO: verify that this is zero-copy/move
            chunk_size: chunk_size,
            settings: &self,
        })
    }

}

#[cfg(test)]
mod tests {
    use super::{Portal,Direction};

    #[test]
    fn portalfile_iterator() {
        let dir = Some(Direction::Sender);
        let key = Some("test".to_string());
        let portal = Portal::init(dir,key);

        // TODO change test file
        let file = portal.load_file("/etc/passwd").unwrap();

        let chunk_size = 10;
        let chunks = portal.get_chunks(&file,chunk_size).unwrap();
        for v in chunks.into_iter() {
            assert!(v.len() <= chunk_size);
        }


        let chunk_size = 1024;
        let chunks = portal.get_chunks(&file,chunk_size).unwrap();
        for v in chunks.into_iter() {
            assert!(v.len() <= chunk_size);
        }

    }
}
