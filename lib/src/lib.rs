use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::fs::File;
use memmap::Mmap;
use std::fs::OpenOptions;
use std::cell::RefCell;
pub mod errors;

use errors::PortalError;


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

pub struct PortalFileImmutable {
    mmap: Mmap,
}

pub struct PortalFileMut {
    file: RefCell<File>,
    //mmap: MmapMut,
    /*len: usize,
    used: usize,
    offset: usize,*/
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

pub enum PortalFile {
    Immutable(PortalFileImmutable),
    Mutable(PortalFileMut),
}

impl PortalFile {

    fn get_bytes(&self) -> Result<&[u8]> {
        match self {
            PortalFile::Immutable(inner) => Ok(&inner.mmap[..]),
            PortalFile::Mutable(_inner) => Err(PortalError::Mutablility.into()),
        }
    }

    pub fn write(&self, data: &[u8]) -> Result<usize> {
        match self {
            PortalFile::Immutable(_inner) => Err(PortalError::Mutablility.into()),
            PortalFile::Mutable(inner) => {return inner.write(data);},
        }
    }
}



impl PortalFileMut {

    fn write(&self, data: &[u8]) -> Result<usize> {
        use std::io::Write;

        /*let remaining = self.len - self.used;
        if data.len() > remaining {
            let diff = data.len() - remaining;
            //self.file.set_len(self.len + diff)?;
            self.len += diff;
        } */

        self.file.borrow_mut().write_all(data)?;
        //(&mut self.mmap[self.offset..]).write_all(data)?;
        //self.offset += data.len();
        Ok(data.len())
    } 

}


impl Drop for PortalFileMut {

    // attempt to flush to disk
    fn drop(&mut self) {
        match self.file.get_mut().sync_all() {
            Ok(_) => {},
            Err(_) => {},
        }
    }

} 

impl Portal {
    
    /**
     * Initialize 
     */
    pub fn init(direction: Option<Direction>, id: Option<String>, pubkey: Option<String>) -> Portal {
        Portal {
            direction: direction,
            id: id,
            pubkey: pubkey,
        }
    }

    /**
     * Construct from data 
     */
    pub fn parse(data: &Vec<u8>) -> Result<Portal> {
        Ok(bincode::deserialize(data)?)
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
     * mmap's a file into memory for reading
     */
    pub fn load_file<'a>(&'a self, f: &str) -> Result<PortalFile>  {
        let file = File::open(f)?;
        let mmap = unsafe { Mmap::map(&file)?  };
        Ok(PortalFile::Immutable(PortalFileImmutable{mmap: mmap}))
    }

    /*
     * mmap's a file into memory for writing
     */
    pub fn create_file<'a>(&'a self, f: &str) -> Result<PortalFile>  {

        let file = OpenOptions::new()
                       .read(true)
                       .write(true)
                       .create(true)
                       .open(&f)?;

        //let mmap = unsafe { Mmap::map(&file)?  };
        //let writer = mmap.make_mut()?;
        Ok(PortalFile::Mutable(PortalFileMut{
            file: RefCell::new(file)}))
            //mmap: writer, 
            //len: 0,
            //used: 0,
            //offset: 0}))
    }

    /**
     * Returns an iterator over the chunks to send it over the
     * network
     */
    pub fn get_chunks<'a>(&'a self, data: &'a PortalFile, chunk_size: usize) -> PortalChunks<'a,u8> {
        
        let bytes = match data.get_bytes() {
            Ok(data) => data,
            Err(_) => &[], // iterator will be empty for writer files
        };

        PortalChunks{
            v: &bytes, // TODO: verify that this is zero-copy/move
            chunk_size: chunk_size,
            settings: &self,
        }
    }

}

#[cfg(test)]
mod tests {
    use super::{Portal,Direction};

    #[test]
    fn portalfile_iterator() {
        let dir = Some(Direction::Sender);
        let key = Some("test".to_string());
        let portal = Portal::init(dir,None,key);

        // TODO change test file
        let file = portal.load_file("/etc/passwd").unwrap();

        let chunk_size = 10;
        let chunks = portal.get_chunks(&file,chunk_size);
        for v in chunks.into_iter() {
            assert!(v.len() <= chunk_size);
        }


        let chunk_size = 1024;
        let chunks = portal.get_chunks(&file,chunk_size);
        for v in chunks.into_iter() {
            assert!(v.len() <= chunk_size);
        }

    }

    #[test]
    fn portal_createfile() {
        let dir = Some(Direction::Sender);
        let key = Some("test".to_string());
        let portal = Portal::init(dir,None,key);

        // TODO change test file
        let file_src = portal.load_file("/etc/passwd").unwrap();
        let mut file_dst = portal.create_file("/tmp/passwd").unwrap();

        let chunk_size = 1024;
        let chunks = portal.get_chunks(&file_src,chunk_size);
        for v in chunks.into_iter() {

            assert!(v.len() <= chunk_size);

            // test writing chunk
            file_dst.write(&v).unwrap();
        }

    }

}
