use super::{Direction, Protocol};
use crate::errors::PortalError;
use crate::protocol::{
    ConnectMessage, EncryptedMessage, NonceSequence, PortalConfirmation, PortalMessage,
    TransferInfo, TransferInfoBuilder,
};
use crate::tests::MockTcpStream;
use crate::Portal;
use mockstream::SyncMockStream;
use std::convert::TryInto;
use std::path::Path;
use std::thread;

macro_rules! assert_err {
    ($expression:expr, $($pattern:tt)+) => {
        match $expression {
            $($pattern)+ => (),
            ref e => panic!("expected `{}` but got `{:?}`", stringify!($($pattern)+), e),
        }
    }
}

#[test]
fn test_nonce() {
    let mut n = NonceSequence::new();
    let mut old = [0u8; 12];
    for _ in 0..5_000_000 {
        let new = n.next_unique().unwrap();
        // test that every nonce is greater than the last
        // which means it is larger & different than all previous
        assert!(new > old);
        old = new;
    }
}

#[test]
fn test_connect() {
    // receiver
    let pass = "test".to_string();
    let receiver = Portal::init(Direction::Receiver, "id".to_string(), pass).unwrap();

    // sender
    let pass = "test".to_string();
    let sender = Portal::init(Direction::Sender, "id".to_string(), pass).unwrap();

    let (mut senderstream, mut receiverstream) = MockTcpStream::channel();

    // Save sender.exchange before move
    let senderexchange = sender.exchange.clone();
    let handle = thread::spawn(move || {
        Protocol::connect(
            &mut senderstream,
            sender.get_id(),
            sender.get_direction(),
            sender.exchange,
        )
        .unwrap()
    });

    let receiver_got = Protocol::connect(
        &mut receiverstream,
        receiver.get_id(),
        receiver.get_direction(),
        receiver.exchange,
    )
    .unwrap();

    let sender_got = handle.join().unwrap();
    assert_eq!(sender_got, receiver.exchange);
    assert_eq!(receiver_got, senderexchange);
}

#[test]
fn test_key_derivation() {
    // receiver
    let pass = "test".to_string();
    let mut receiver = Portal::init(Direction::Receiver, "id".to_string(), pass).unwrap();

    // sender
    let pass = "test".to_string();
    let mut sender = Portal::init(Direction::Sender, "id".to_string(), pass).unwrap();

    let (mut senderstream, mut receiverstream) = MockTcpStream::channel();

    // Save sender.exchange before move
    let handle = thread::spawn(move || {
        let msg = Protocol::connect(
            &mut senderstream,
            sender.get_id(),
            sender.get_direction(),
            sender.exchange,
        )
        .unwrap();

        // after calling finish() the SPAKE2 struct will be consumed
        // so we must replace the value stored in self.state
        let state = sender.state.take().unwrap();
        Protocol::derive_key(state, &msg).unwrap()
    });

    let receiver_got = Protocol::connect(
        &mut receiverstream,
        receiver.get_id(),
        receiver.get_direction(),
        receiver.exchange,
    )
    .unwrap();

    // Sender key
    let skey = handle.join().unwrap();

    // Dervice recevier key
    let state = receiver.state.take().unwrap();
    let rkey = Protocol::derive_key(state, &receiver_got).unwrap();
    assert_eq!(rkey, skey);
}

#[test]
fn test_key_confirmation() {
    // receiver
    let pass = "test".to_string();
    let mut receiver = Portal::init(Direction::Receiver, "id".to_string(), pass).unwrap();

    // sender
    let pass = "test".to_string();
    let mut sender = Portal::init(Direction::Sender, "id".to_string(), pass).unwrap();

    let (mut senderstream, mut receiverstream) = MockTcpStream::channel();

    // Save sender.exchange before move
    let handle = thread::spawn(move || {
        let msg = Protocol::connect(
            &mut senderstream,
            sender.get_id(),
            sender.get_direction(),
            sender.exchange,
        )
        .unwrap();

        // after calling finish() the SPAKE2 struct will be consumed
        // so we must replace the value stored in self.state
        let state = sender.state.take().unwrap();
        let skey = Protocol::derive_key(state, &msg).unwrap();

        // Perform the confirmation step
        (
            Protocol::confirm_peer(
                &mut senderstream,
                sender.get_id(),
                sender.get_direction(),
                &skey,
            )
            .unwrap(),
            skey,
        )
    });

    // Receiver connect
    let receiver_got = Protocol::connect(
        &mut receiverstream,
        receiver.get_id(),
        receiver.get_direction(),
        receiver.exchange,
    )
    .unwrap();

    // Derive recevier key
    let state = receiver.state.take().unwrap();
    let rkey = Protocol::derive_key(state, &receiver_got).unwrap();

    // Receiver confirm
    let receiver_result = Protocol::confirm_peer(
        &mut receiverstream,
        receiver.get_id(),
        receiver.get_direction(),
        &rkey,
    )
    .unwrap();

    // Join sender
    let (sender_result, skey) = handle.join().unwrap();

    // Assert key and confirm result are equal
    assert_eq!(rkey, skey);
    assert_eq!(sender_result, receiver_result);
}

#[test]
fn test_serialize_deserialize_message() {
    let values = ConnectMessage {
        id: "id".to_string(),
        direction: Direction::Sender,
    };

    let message = PortalMessage::Connect(values.clone());

    let ser = bincode::serialize(&message).unwrap();
    let res = PortalMessage::parse(&ser).unwrap();

    // Retreive inner message
    let res = match res {
        PortalMessage::Connect(inner) => inner,
        _ => panic!("Incorrect message type"),
    };

    // fields that should be the same
    assert_eq!(res.id, values.id);
    assert_eq!(res.direction, values.direction);
}

#[test]
fn test_connect_badmsg() {
    let id = "id".to_string();
    let mut stream = SyncMockStream::new();

    // Serialize and push a Connect message
    let values = ConnectMessage {
        id: id.clone(),
        direction: Direction::Sender,
    };
    let message = PortalMessage::Connect(values);
    stream.push_bytes_to_read(&bincode::serialize(&message).unwrap());

    // Serialize and push another Connect message
    // when the peer expects a KeyExchange message
    // this will cause connect() to return BadMsg
    stream.push_bytes_to_read(&bincode::serialize(&message).unwrap());

    // Call the function under test
    let handle = thread::spawn(move || {
        Protocol::connect(
            &mut stream,
            &id,
            Direction::Receiver,
            vec![0u8; 33].try_into().unwrap(),
        )
        .unwrap_err()
        .downcast::<PortalError>()
        .unwrap()
    });

    // Retreive and verify the result
    let result = handle.join().unwrap();
    assert_eq!(*result, PortalError::BadMsg);
}

#[test]
fn test_confirm_peer_badmsg() {
    let id = "id".to_string();
    let mut stream = SyncMockStream::new();

    // Serialize and push a Connect message
    // when the peer expects a Confirm message
    // this will cause confirm_peer() to return BadMsg
    let values = ConnectMessage {
        id: id.clone(),
        direction: Direction::Sender,
    };
    let message = PortalMessage::Connect(values);
    stream.push_bytes_to_read(&bincode::serialize(&message).unwrap());

    // Call the function under test
    let handle = thread::spawn(move || {
        Protocol::confirm_peer(&mut stream, &id, Direction::Receiver, &[0u8; 32])
            .unwrap_err()
            .downcast::<PortalError>()
            .unwrap()
    });

    // Retreive and verify the result
    let result = handle.join().unwrap();
    assert_eq!(*result, PortalError::BadMsg);
}

#[test]
fn test_confirm_peer_unexpected_hkdf() {
    let id = "id".to_string();
    let mut stream = SyncMockStream::new();

    // Serialize and push a properly formatted Confirm
    // message that doesn't match what we should send
    // if we know the key
    let values = PortalConfirmation { 0: [1u8; 42] };
    let message = PortalMessage::Confirm(values);
    stream.push_bytes_to_read(&bincode::serialize(&message).unwrap());

    // Call the function under test
    let handle = thread::spawn(move || {
        Protocol::confirm_peer(&mut stream, &id, Direction::Receiver, &[0u8; 32])
            .unwrap_err()
            .downcast::<PortalError>()
            .unwrap()
    });

    // Retreive and verify the result
    let result = handle.join().unwrap();
    assert_eq!(*result, PortalError::PeerKeyMismatch);
}

#[test]
fn test_read_encrypted_zero_copy_badmsg() {
    let id = "id".to_string();
    let mut stream = SyncMockStream::new();

    // Serialize and push a Connect message
    // when the peer expects a EncryptedDataHeader message
    // this will cause read_encrypted_zero_copy() to return BadMsg
    let values = ConnectMessage {
        id: id.clone(),
        direction: Direction::Sender,
    };
    let message = PortalMessage::Connect(values);
    stream.push_bytes_to_read(&bincode::serialize(&message).unwrap());

    // Call the function under test
    let mut storage = vec![0u8; 1024];
    let handle = thread::spawn(move || {
        Protocol::read_encrypted_zero_copy(&mut stream, &[0u8; 32], &mut storage)
            .unwrap_err()
            .downcast::<PortalError>()
            .unwrap()
    });

    // Retreive and verify the result
    let result = handle.join().unwrap();
    assert_eq!(*result, PortalError::BadMsg);
}

#[test]
fn test_read_encrypted_zero_copy_buffertoosmall() {
    let mut stream = SyncMockStream::new();

    // Allocate a small amount of storage
    let mut storage = vec![0u8; 1024];

    // Serialize and push a EncryptedDataHeader message
    // but with an extremely large msg.len that exceeds
    // our storage size.
    let mut values = EncryptedMessage::default();
    values.len = 1_000_000;
    let message = PortalMessage::EncryptedDataHeader(values);
    stream.push_bytes_to_read(&bincode::serialize(&message).unwrap());

    // Call the function under test
    let handle = thread::spawn(move || {
        Protocol::read_encrypted_zero_copy(&mut stream, &[0u8; 32], &mut storage)
            .unwrap_err()
            .downcast::<PortalError>()
            .unwrap()
    });

    // Retreive and verify the result
    let result = handle.join().unwrap();
    assert_eq!(*result, PortalError::BufferTooSmall);
}

#[test]
fn transferinfo_strips_paths() {
    let info = TransferInfoBuilder::new()
        .add_file(Path::new("/etc/passwd"))
        .unwrap()
        .finalize();

    let ser = bincode::serialize(&info).unwrap();
    let other: TransferInfo = bincode::deserialize(&ser).unwrap();

    // Assert localpaths are stripped
    assert!(other.localpaths.is_empty());

    // Metadata should be identical
    assert_eq!(info.all, other.all);

    // Metadata should not contain the folder "/etc"
    for file in info.all {
        assert!(!file.filename.contains("etc"));
    }
}

#[test]
fn transferinfo_add_bad_path() {
    let result = TransferInfoBuilder::new().add_file(Path::new("/etc/.."));
    assert!(result.is_err());
    assert_err!(
        result.err().unwrap().downcast_ref::<PortalError>(),
        Some(PortalError::BadFileName)
    );
}
