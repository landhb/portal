//! Provides primary tests for the PortalFile abstraction
//!
use crate::{errors::PortalError, Direction, Portal};
use crate::{NO_PROGRESS_CALLBACK, NO_VERIFY_CALLBACK};
use mockstream::SyncMockStream;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use tempdir::TempDir;

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
        while self.waiting_for_write.load(Ordering::Relaxed) == 0 {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // Perform the read
        let res = self.readbuf.read(buf)?;

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

        // Update the amount to read
        self.write_done.fetch_add(buf.len(), Ordering::SeqCst);

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
        let senderstream = MockTcpStream {
            id: Direction::Sender,
            waiting_for_write: senderbool.clone(),
            readbuf: senderbuf.clone(),
            write_done: recvbool.clone(),
            writebuf: receiverbuf.clone(),
        };

        let receiverstream = MockTcpStream {
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
    // Create test file
    let tmp_dir = TempDir::new("test_recv_file_bad_outdir").unwrap();
    let file_path = tmp_dir.path().join("randomfile.txt");
    let file_path_str = file_path.to_str().unwrap().to_owned();
    let mut tmp_file = File::create(file_path).unwrap();
    writeln!(tmp_file, "Test File").unwrap();

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

        // Send the file
        let result = sender.send_file(&mut senderstream, &file_path_str, NO_PROGRESS_CALLBACK);
        assert!(result.is_ok());
        result.unwrap()
    });

    // Complete handshake
    receiver.handshake(&mut receiverstream).unwrap();

    // Receive the file
    let metadata = receiver
        .recv_file(
            &mut receiverstream,
            tmp_dir.path(),
            NO_VERIFY_CALLBACK,
            NO_PROGRESS_CALLBACK,
        )
        .unwrap();

    // Wait for sending to complete
    let sent_size = sender_thread.join().unwrap();

    // Compare sizes
    assert_eq!(metadata.filesize, sent_size as u64);
}

#[test]
fn portal_send_file_no_peer() {
    let dir = Direction::Sender;
    let pass = "test".to_string();
    let mut portal = Portal::init(dir, "id".to_string(), pass).unwrap();

    // will return error
    let mut stream = SyncMockStream::new();
    let result = portal.send_file(&mut stream, "/tmp/passwd", NO_PROGRESS_CALLBACK);
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

    // will panic due to lack of peer
    let mut stream = SyncMockStream::new();
    let result = portal.recv_file(
        &mut stream,
        Path::new("/tmp"),
        NO_VERIFY_CALLBACK,
        NO_PROGRESS_CALLBACK,
    );
    assert!(result.is_err());
    assert_err!(
        result.err().unwrap().downcast_ref::<PortalError>(),
        Some(PortalError::NoPeer)
    );
}

#[test]
fn test_recv_file_bad_outdir() {
    // Create test file
    let tmp_dir = TempDir::new("test_recv_file_bad_outdir").unwrap();
    let file_path = tmp_dir.path().join("randomfile.txt");
    let file_path_str = file_path.to_str().unwrap().to_owned();
    let mut tmp_file = File::create(file_path).unwrap();
    writeln!(tmp_file, "Test File").unwrap();

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

        // Send the file
        let result = sender.send_file(&mut senderstream, &file_path_str, NO_PROGRESS_CALLBACK);
        assert!(result.is_ok());
        result.unwrap()
    });

    // Complete handshake
    receiver.handshake(&mut receiverstream).unwrap();

    let result = receiver.recv_file(
        &mut receiverstream,
        Path::new("/tmp/test.txt"),
        NO_VERIFY_CALLBACK,
        NO_PROGRESS_CALLBACK,
    );
    assert!(result.is_err());
    assert_err!(
        result.err().unwrap().downcast_ref::<PortalError>(),
        Some(PortalError::BadDirectory)
    );

    sender_thread.join().unwrap();
}

#[test]
fn test_recv_file_cancel() {
    // Create test file
    let tmp_dir = TempDir::new("test_recv_file_bad_outdir").unwrap();
    let file_path = tmp_dir.path().join("randomfile.txt");
    let file_path_str = file_path.to_str().unwrap().to_owned();
    let mut tmp_file = File::create(file_path).unwrap();
    writeln!(tmp_file, "Test File").unwrap();

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

        // Send the file
        let result = sender.send_file(&mut senderstream, &file_path_str, NO_PROGRESS_CALLBACK);
        assert!(result.is_ok());
        result.unwrap()
    });

    // Complete handshake
    receiver.handshake(&mut receiverstream).unwrap();

    // VerifyCallback that cancels every download
    fn cancel_all(_path: &str, _size: u64) -> bool {
        false
    }

    let result = receiver.recv_file(
        &mut receiverstream,
        tmp_dir.path(),
        Some(cancel_all),
        NO_PROGRESS_CALLBACK,
    );
    assert!(result.is_err());
    assert_err!(
        result.err().unwrap().downcast_ref::<PortalError>(),
        Some(PortalError::Cancelled)
    );

    sender_thread.join().unwrap();
}

#[test]
fn test_incomplete_transfer() {}

#[test]
fn test_failed_decryption() {}

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
