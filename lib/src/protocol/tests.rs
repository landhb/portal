use super::{Direction, Protocol};
use crate::protocol::{ConnectMessage, PortalMessage};
use crate::tests::MockTcpStream;
use crate::Portal;
use hkdf::Hkdf;
use sha2::Sha256;
use std::thread;

#[test]
fn test_connect() {
    // receiver
    let pass = "test".to_string();
    let mut receiver = Portal::init(Direction::Receiver, "id".to_string(), pass).unwrap();

    // sender
    let pass = "test".to_string();
    let mut sender = Portal::init(Direction::Sender, "id".to_string(), pass).unwrap();

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
    let senderexchange = sender.exchange.clone();
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
    let senderexchange = sender.exchange.clone();
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
