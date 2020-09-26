use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::fs::File;
use memmap::Mmap;
use std::fs::OpenOptions;
use std::cell::RefCell;
use spake2::{Ed25519Group, Identity, Password, SPAKE2};
use sha2::{Sha256, Digest};

pub mod errors;
use errors::PortalError;


/**
 * The primary interface into the library
 */
#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Portal{

    // Information to correlate
    // connections on the relay
    id: String,
    direction: Option<Direction>,

    // Metadata to be exchanged
    // between peers
    filename: Option<String>,

    // Never serialized or sent to the relay
    #[serde(skip)]
    state: Option<SPAKE2<Ed25519Group>>,

    // Never serialized or sent to the relay
    #[serde(skip)]
    key: Option<Vec<u8>>,
}



#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum Direction {
    Sender,
    Reciever,
}


/**
 * A file mapping, either an immutable mmap 
 */
pub enum PortalFile {
    Immutable(PortalFileImmutable),
    Mutable(PortalFileMutable),
}

pub struct PortalFileImmutable {
    mmap: Mmap,
}

pub struct PortalFileMutable {
    file: RefCell<File>,
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



impl PortalFileMutable {

    fn write(&self, data: &[u8]) -> Result<usize> {
        use std::io::Write;
        self.file.borrow_mut().write_all(data)?;
        Ok(data.len())
    } 

}


impl Drop for PortalFileMutable {

    // attempt to flush to disk
    fn drop(&mut self) {
        let _ = self.file.get_mut().sync_all();
    }

} 

impl Portal {
    
    /**
     * Initialize 
     */
    pub fn init(direction: Option<Direction>, 
                password: String,
                filename: Option<String>) -> (Portal,Vec<u8>) {

        
        // use password to compute ID string
        let mut hasher = Sha256::new();
        hasher.update(&password);
        let id_bytes = hasher.finalize();
        let id = hex::encode(&id_bytes);

        let (s1, outbound_msg) = SPAKE2::<Ed25519Group>::start_symmetric(
           &Password::new(&password.as_bytes()),
           &Identity::new(&id_bytes));
       

        return (Portal {
            direction: direction,
            id: id,
            filename: filename,
            state: Some(s1),
            key: None,
        }, outbound_msg);
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

    pub fn get_id(&self) -> &String {
        &self.id
    }

    pub fn get_direction(&self) -> Option<Direction> {
        self.direction.clone()
    }

    pub fn set_id(&mut self, id: String) {
        self.id = id;
    }

    pub fn set_direction(&mut self, direction: Option<Direction>) {
        self.direction = direction;
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

        Ok(PortalFile::Mutable(PortalFileMutable{file: RefCell::new(file)}))
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


    pub fn confirm_peer(&mut self, msg_data: &[u8]) -> Result<()> {

        // after calling finish() the SPAKE2 struct will be consumed
        // so we must replace the value stored in self.state
        let state = std::mem::replace(&mut self.state, None);

        let state = state.ok_or_else(|| PortalError::BadState)?;

        self.key = match state.finish(msg_data) {
            Ok(res) => Some(res),
            Err(_) => {return Err(PortalError::BadMsg.into());}
        };
        Ok(())
    }

}

#[cfg(test)]
mod tests {
    use super::{Portal,Direction};

    #[test]
    fn key_derivation() {

        // receiver
        let dir = Some(Direction::Reciever);
        let pass ="test".to_string();
        let (mut receiver,receiver_msg) = Portal::init(dir,pass,None);

        // sender
        let dir = Some(Direction::Sender);
        let pass ="test".to_string();
        let (mut sender,sender_msg) = Portal::init(dir,pass,None);


        receiver.confirm_peer(&sender_msg).unwrap();
        sender.confirm_peer(&receiver_msg).unwrap();

        assert!(receiver.key == sender.key);
    }


    #[test]
    fn portalfile_iterator() {
        let dir = Some(Direction::Sender);
        let pass ="test".to_string();
        let (portal,_msg) = Portal::init(dir,pass,None);

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
        let pass = "test".to_string();
        let (portal,_msg) = Portal::init(dir,pass, None);

        // TODO change test file
        let file_src = portal.load_file("/etc/passwd").unwrap();
        let file_dst = portal.create_file("/tmp/passwd").unwrap();

        let chunk_size = 1024;
        let chunks = portal.get_chunks(&file_src,chunk_size);
        for v in chunks.into_iter() {

            assert!(v.len() <= chunk_size);

            // test writing chunk
            file_dst.write(&v).unwrap();
        }

    }

}
