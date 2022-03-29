use super::{Direction, Protocol};
use crate::tests::MockTcpStream;
use crate::Portal;
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

/*

#[test]
fn test_key_confirmation() {
    let mut receiver_side = MockTcpStream {
        data: Vec::with_capacity(crate::CHUNK_SIZE),
    };

    let mut sender_side = MockTcpStream {
        data: Vec::with_capacity(crate::CHUNK_SIZE),
    };

    // receiver
    let dir = Direction::Receiver;
    let pass = "test".to_string();
    let (mut receiver, receiver_msg) = Portal::init(dir, "id".to_string(), pass, None);

    // sender
    let dir = Direction::Sender;
    let pass = "test".to_string();
    let (mut sender, sender_msg) = Portal::init(dir, "id".to_string(), pass, None);

    receiver.derive_key(sender_msg.as_slice()).unwrap();
    sender.derive_key(receiver_msg.as_slice()).unwrap();

    // identifiers known to each party
    let id = receiver.get_id();
    let sender_info = format!("{}-{}", id, "senderinfo");
    let receiver_info = format!("{}-{}", id, "receiverinfo");

    // Perform the HKDF operations
    let h = Hkdf::<Sha256>::new(None, &sender.key.as_ref().unwrap());
    let mut sender_confirm = [0u8; 42];
    let mut receiver_confirm = [0u8; 42];
    h.expand(&sender_info.as_bytes(), &mut sender_confirm)
        .unwrap();
    h.expand(&receiver_info.as_bytes(), &mut receiver_confirm)
        .unwrap();

    // pre-send the appropriate HKDF to each stream, simulating a peer
    receiver_side.write(&sender_confirm).unwrap();
    sender_side.write(&receiver_confirm).unwrap();

    // each side should be able to confirm the other
    receiver.confirm_peer(&mut receiver_side).unwrap();
    sender.confirm_peer(&mut sender_side).unwrap();
} */
