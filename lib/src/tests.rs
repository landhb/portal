//! Provides primary tests for the PortalFile abstraction
//!
use crate::{errors::PortalError, Direction, Portal};
use mockstream::SyncMockStream;
use rand::Rng;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::thread;

pub struct MockTcpStream {
    pub id: Direction,
    pub waiting_for_write: Arc<AtomicUsize>,
    pub readbuf: SyncMockStream,
    pub write_done: Arc<AtomicUsize>,
    pub writebuf: SyncMockStream,
}

impl Read for MockTcpStream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        // Blocking read, wait until data is available
        let mut count = 0;
        while self.waiting_for_write.load(Ordering::Relaxed) == 0 {
            if count > 20 {
                break;
            }
            count += 1;
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // Perform the read
        let res = self.readbuf.read(buf)?;

        /*println!(
            "{:?} read {:?} bytes, left {:?}",
            self.id,
            res,
            self.waiting_for_write.load(Ordering::Relaxed)
        ); */

        // Subtract the amount read from the atomic
        if self.waiting_for_write.load(Ordering::Relaxed) > res {
            self.waiting_for_write.fetch_sub(res, Ordering::SeqCst);
        } else {
            self.waiting_for_write.store(0, Ordering::SeqCst);
        }
        Ok(res)
    }
}

impl Write for MockTcpStream {
    fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
        // Push data onto the peer's buffer
        self.writebuf.push_bytes_to_read(buf);

        //println!("{:?} sent {:?} bytes", self.id, buf.len());

        // If they are blocked, signal that data is ready
        if self.write_done.load(Ordering::Relaxed) == 0 {
            self.write_done.fetch_add(buf.len(), Ordering::SeqCst);
        }

        // Return the amount written
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        self.writebuf.flush()
    }
}

impl MockTcpStream {
    pub fn channel() -> (Self, Self) {
        // Backing buffers
        let senderbuf = SyncMockStream::new();
        let receiverbuf = SyncMockStream::new();

        // Backing bools
        let senderbool = Arc::new(AtomicUsize::default());
        let recvbool = Arc::new(AtomicUsize::default());
        senderbool.store(0, Ordering::Relaxed);
        recvbool.store(0, Ordering::Relaxed);

        // Wrap in Mock type
        let mut senderstream = MockTcpStream {
            id: Direction::Sender,
            waiting_for_write: senderbool.clone(),
            readbuf: senderbuf.clone(),
            write_done: recvbool.clone(),
            writebuf: receiverbuf.clone(),
        };

        let mut receiverstream = MockTcpStream {
            id: Direction::Receiver,
            waiting_for_write: recvbool,
            readbuf: receiverbuf,
            write_done: senderbool,
            writebuf: senderbuf,
        };

        (senderstream, receiverstream)
    }
}

macro_rules! assert_err {
    ($expression:expr, $($pattern:tt)+) => {
        match $expression {
            $($pattern)+ => (),
            ref e => panic!("expected `{}` but got `{:?}`", stringify!($($pattern)+), e),
        }
    }
}

#[test]
fn handshake_suceeds() {
    let fsize = 1337;
    let fname = "filename".to_string();

    // receiver
    let dir = Direction::Receiver;
    let pass = "test".to_string();
    let mut receiver = Portal::init(dir, "id".to_string(), pass).unwrap();

    // sender
    let dir = Direction::Sender;
    let pass = "test".to_string();
    let mut sender = Portal::init(dir, "id".to_string(), pass).unwrap();

    // mock channel
    let (mut senderstream, mut receiverstream) = MockTcpStream::channel();

    let sender_thread = thread::spawn(move || {
        sender.handshake(&mut senderstream).unwrap();
    });

    receiver.handshake(&mut receiverstream).unwrap();
    sender_thread.join().unwrap();
}

#[test]
fn test_file_roundtrip() {
    let fsize = 1337;
    let fname = "filename".to_string();

    // receiver
    let dir = Direction::Receiver;
    let pass = "test".to_string();
    let mut receiver = Portal::init(dir, "id".to_string(), pass).unwrap();

    // sender
    let dir = Direction::Sender;
    let pass = "test".to_string();
    let mut sender = Portal::init(dir, "id".to_string(), pass).unwrap();

    // mock channel
    let (mut senderstream, mut receiverstream) = MockTcpStream::channel();

    let sender_thread = thread::spawn(move || {
        // Complete handshake
        sender.handshake(&mut senderstream).unwrap();

        println!("COMPLETED HANDSHAKE");

        // Send the file
        let result = sender.send_file(&mut senderstream, "/tmp/passwd", None);
        println!("FINISHED SENDING");
        assert!(result.is_ok());
        result.unwrap()
    });

    // Complete handshake
    receiver.handshake(&mut receiverstream).unwrap();

    //std::thread::sleep(std::time::Duration::new(5,0));

    // Wait for sending to complete
    let sent_size = sender_thread.join().unwrap();
    println!("TOTAL SENT {:?}", sent_size);

    // Receive the file
    let metadata = receiver.recv_file(&mut receiverstream, None, None).unwrap();

    // Compare sizes

    assert_eq!(metadata.filesize, sent_size as u64);
}

#[test]
fn portal_send_file_no_peer() {
    let dir = Direction::Sender;
    let pass = "test".to_string();
    let mut portal = Portal::init(dir, "id".to_string(), pass).unwrap();

    // handshake is skipped

    // will return error
    let mut stream = SyncMockStream::new();
    let result = portal.send_file(&mut stream, "/tmp/passwd", None::<fn(usize)>);
    assert!(result.is_err());
    assert_err!(
        result.err().unwrap().downcast_ref::<PortalError>(),
        Some(PortalError::NoPeer)
    );
}

#[test]
fn portal_recv_file_no_peer() {
    let dir = Direction::Sender;
    let pass = "test".to_string();
    let mut portal = Portal::init(dir, "id".to_string(), pass).unwrap();

    // handshake is skipped

    // will panic due to lack of peer
    let mut stream = SyncMockStream::new();
    let result = portal.recv_file(&mut stream, None, None);
    assert!(result.is_err());
    assert_err!(
        result.err().unwrap().downcast_ref::<PortalError>(),
        Some(PortalError::NoPeer)
    );
}

#[test]
fn filemetadata() {}

#[test]
fn test_compressed_edwards_size() {
    // The exchanged message is the CompressedEdwardsY + 1 byte for the SPAKE direction
    let edwards_point = <spake2::Ed25519Group as spake2::Group>::Element::default();
    let compressed = edwards_point.compress();
    let msg_size: usize = std::mem::size_of_val(&compressed) + 1;

    assert_eq!(33, msg_size);
}

#[test]
fn test_getters_setters() {
    let dir = Direction::Sender;
    let pass = "test".to_string();
    let mut portal = Portal::init(dir, "id".to_string(), pass).unwrap();

    // get/set ID
    portal.set_id("newID".to_string());
    assert_eq!("newID", portal.get_id());

    // get/set direction
    portal.set_direction(Direction::Receiver);
    assert_eq!(portal.get_direction(), Direction::Receiver);
}
/*

#[test]
fn test_failed_decryption() {
    // receiver
    let dir = Direction::Receiver;
    let pass = "test".to_string();
    let (mut receiver, receiver_msg) = Portal::init(dir, "id".to_string(), pass, None);

    // sender
    let dir = Direction::Sender;
    let pass = "test".to_string();
    let (mut sender, sender_msg) = Portal::init(dir, "id".to_string(), pass, None);

    // we need a key to be able to encrypt
    receiver.derive_key(sender_msg.as_slice()).unwrap();
    sender.derive_key(receiver_msg.as_slice()).unwrap();

    // encrypt the file
    let mut file = sender.load_file("/etc/passwd").unwrap();
    file.encrypt().unwrap();

    // Test incorrect tag length path
    let old_tag = file.state.tag.clone();
    file.state.tag.truncate(0);
    let result = file.decrypt();
    assert!(result.is_err());
    let _ = result.map_err(|e| match e.downcast_ref::<PortalError>() {
        Some(PortalError::DecryptError) => anyhow::Ok(()),
        _ => panic!("Unexpected error"),
    });

    // Test failed decryption path
    file.state.tag = old_tag;
    file.state.tag[0] += 1; // alter tag
    let result = file.decrypt();
    assert!(result.is_err());
    let _ = result.map_err(|e| match e.downcast_ref::<PortalError>() {
        Some(PortalError::DecryptError) => anyhow::Ok(()),
        _ => panic!("Unexpected error"),
    });
}

#[test]
fn test_sync_file_download_file() {
    // receiver
    let dir = Direction::Receiver;
    let pass = "test".to_string();
    let (mut receiver, receiver_msg) = Portal::init(dir, "id".to_string(), pass, None);

    // sender
    let dir = Direction::Sender;
    let pass = "test".to_string();
    let (mut sender, sender_msg) = Portal::init(dir, "id".to_string(), pass, None);

    // we need a key to be able to encrypt
    receiver.derive_key(sender_msg.as_slice()).unwrap();
    sender.derive_key(receiver_msg.as_slice()).unwrap();

    // encrypt the file
    let mut file = sender.load_file("/etc/passwd").unwrap();
    file.encrypt().unwrap();

    let mut stream = MockTcpStream {
        data: Vec::with_capacity(crate::CHUNK_SIZE),
    };

    // communicate the necessary state info
    // for the peer to be able to decrypt the file
    file.sync_file_state(&mut stream).unwrap();

    // send the file over the stream
    for data in file.get_chunks(crate::CHUNK_SIZE) {
        stream.write(&data).unwrap();
    }

    // use download_file to read in the file data
    let mut new_file = receiver
        .create_file("/tmp/passwd", file.mmap[..].len() as u64)
        .unwrap();
    new_file
        .download_file(&mut stream, |x| println!("{:?}", x))
        .unwrap();

    // compare the state of the two files
    assert_eq!(&file.state.tag, &new_file.state.tag);
    assert_eq!(&file.state.nonce, &new_file.state.nonce);
    assert_eq!(&file.mmap[..], &new_file.mmap[..]);

    new_file.decrypt().unwrap(); // should not panic
    stream.flush().unwrap(); // just for coverage reporting, does nothing
}


#[test]
fn test_encrypt_decrypt() {
    // receiver
    let dir = Direction::Receiver;
    let pass = "test".to_string();
    let (mut receiver, receiver_msg) = Portal::init(dir, "id".to_string(), pass, None);

    // sender
    let dir = Direction::Sender;
    let pass = "test".to_string();
    let (mut sender, sender_msg) = Portal::init(dir, "id".to_string(), pass, None);

    // we need a key to be able to encrypt
    receiver.derive_key(sender_msg.as_slice()).unwrap();
    sender.derive_key(receiver_msg.as_slice()).unwrap();

    let mut file = sender.load_file("/etc/passwd").unwrap();

    let file_before = String::from_utf8((&file.mmap[..]).to_vec());
    file.encrypt().unwrap();
    let file_encrypted = String::from_utf8((&file.mmap[..]).to_vec());
    file.decrypt().unwrap();
    let file_after = String::from_utf8((&file.mmap[..]).to_vec());

    assert_ne!(file_before, file_encrypted);
    assert_eq!(file_before, file_after);
}


#[test]
fn fail_decrypt_metadata() {
    let fsize = 1337;
    let fname = "filename".to_string();

    // receiver
    let dir = Direction::Receiver;
    let pass = "test".to_string();
    let (mut receiver, receiver_msg) = Portal::init(dir, "id".to_string(), pass, None);

    // sender
    let dir = Direction::Sender;
    let pass = "test".to_string();
    let (mut sender, sender_msg) =
        Portal::init(dir, "id".to_string(), pass, Some(fname.clone()));
    sender.set_file_size(fsize);

    // we need a key to be able to encrypt & decrypt
    receiver.derive_key(sender_msg.as_slice()).unwrap();
    sender.derive_key(receiver_msg.as_slice()).unwrap();

    // Mock channel
    let mut stream = MockTcpStream {
        data: Vec::with_capacity(crate::CHUNK_SIZE),
    };

    // Send garbage state
    let mut garbage = bincode::serialize(&StateMetadata::default()).unwrap();
    garbage.extend_from_slice(&bincode::serialize(&vec![0u8]).unwrap());
    stream.write_all(&garbage).unwrap();

    // Verify error is BadState
    let res = receiver.read_metadata_from(&mut stream);
    assert!(res.is_err());
    let _ = res.map_err(|e| match e.downcast_ref::<PortalError>() {
        Some(PortalError::BadState) => anyhow::Ok(()),
        _ => panic!("Unexpected error"),
    });

    // Send garbage metadata
    let state = StateMetadata {
        nonce: rand::thread_rng().gen::<[u8; 12]>().to_vec(),
        tag: rand::thread_rng().gen::<[u8; 16]>().to_vec(),
    };
    let mut garbage = bincode::serialize(&state).unwrap();
    garbage.extend_from_slice(&bincode::serialize(&vec![0u8]).unwrap());
    stream.write_all(&garbage).unwrap();

    // Verify error is DecryptError
    let res = receiver.read_metadata_from(&mut stream);
    assert!(res.is_err());
    let _ = res.map_err(|e| match e.downcast_ref::<PortalError>() {
        Some(PortalError::DecryptError) => anyhow::Ok(()),
        _ => panic!("Unexpected error"),
    });
}




#[test]
fn portal_load_file() {
    let dir = Direction::Receiver;
    let pass = "test".to_string();
    let (_receiver, receiver_msg) = Portal::init(dir, "id".to_string(), pass, None);

    // sender
    let dir = Direction::Sender;
    let pass = "test".to_string();
    let (mut sender, _sender_msg) = Portal::init(dir, "id".to_string(), pass, None);

    // Confirm
    sender.derive_key(receiver_msg.as_slice()).unwrap();

    // TODO change test file
    let _file = sender.load_file("/etc/passwd").unwrap();
}

#[test]
fn portalfile_chunks_iterator() {
    // receiver
    let dir = Direction::Receiver;
    let pass = "test".to_string();
    let (_receiver, receiver_msg) = Portal::init(dir, "id".to_string(), pass, None);

    // sender
    let dir = Direction::Sender;
    let pass = "test".to_string();
    let (mut sender, _sender_msg) = Portal::init(dir, "id".to_string(), pass, None);

    // Confirm
    sender.derive_key(receiver_msg.as_slice()).unwrap();

    // TODO change test file
    let file = sender.load_file("/etc/passwd").unwrap();

    let chunk_size = 10;
    for v in file.get_chunks(chunk_size) {
        assert!(v.len() <= chunk_size);
    }

    let chunk_size = 1024;
    for v in file.get_chunks(chunk_size) {
        assert!(v.len() <= chunk_size);
    }
}

#[test]
fn portal_createfile() {
    // receiver
    let dir = Direction::Receiver;
    let pass = "test".to_string();
    let (mut receiver, receiver_msg) = Portal::init(dir, "id".to_string(), pass, None);

    // sender
    let dir = Direction::Sender;
    let pass = "test".to_string();
    let (mut sender, sender_msg) = Portal::init(dir, "id".to_string(), pass, None);

    // Confirm
    sender.derive_key(receiver_msg.as_slice()).unwrap();
    receiver.derive_key(sender_msg.as_slice()).unwrap();

    // TODO change test file
    let _file_dst = receiver.create_file("/tmp/passwd", 4096).unwrap();
}

#[test]
fn portal_write_chunk() {
    // receiver
    let dir = Direction::Receiver;
    let pass = "test".to_string();
    let (mut receiver, receiver_msg) = Portal::init(dir, "id".to_string(), pass, None);

    // sender
    let dir = Direction::Sender;
    let pass = "test".to_string();
    let (mut sender, sender_msg) = Portal::init(dir, "id".to_string(), pass, None);

    // Confirm
    sender.derive_key(receiver_msg.as_slice()).unwrap();
    receiver.derive_key(sender_msg.as_slice()).unwrap();

    // TODO change test file
    let file_src = sender.load_file("/etc/passwd").unwrap();
    let mut file_dst = receiver.create_file("/tmp/passwd", 4096).unwrap();

    let chunk_size = 4096;
    for v in file_src.get_chunks(chunk_size) {
        assert!(v.len() <= chunk_size);

        // test writing chunk
        file_dst.write_given_chunk(&v).unwrap();
    }
}

#[test]
#[should_panic]
fn portal_createfile_no_peer() {
    let dir = Direction::Sender;
    let pass = "test".to_string();
    let (portal, _msg) = Portal::init(dir, "id".to_string(), pass, None);

    // will panic due to lack of peer
    let _file_dst = portal.create_file("/tmp/passwd", 4096).unwrap();
}

#[test]
#[should_panic]
fn portal_loadfile_no_peer() {
    let dir = Direction::Sender;
    let pass = "test".to_string();
    let (portal, _msg) = Portal::init(dir, "id".to_string(), pass, None);

    // will panic due to lack of peer
    let _file_src = portal.load_file("/etc/passwd").unwrap();
}

#[test]
fn test_file_trim() {
    let file = Some("/my/path/filename.txt".to_string());

    let dir = Direction::Receiver;
    let pass = "test".to_string();
    let (receiver, _receiver_msg) = Portal::init(dir, "id".to_string(), pass, file);

    let result = receiver.get_file_name().unwrap();

    assert_eq!(result, "filename.txt");
} */
