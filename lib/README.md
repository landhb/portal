# portal-lib

A small Protocol Library for [Portal](https://github.com/landhb/portal) - An encrypted file transfer utility 

This crate enables a consumer to: 

- Create/serialize/deserialize Portal request/response messages.
- Negoticate a symmetric key with a peer using [SPAKE2](https://docs.rs/spake2/0.2.0/spake2) 
- Encrypt files with [Chacha20poly1305](https://blog.cloudflare.com/it-takes-two-to-chacha-poly/) using the [RustCrypto implementation](https://docs.rs/chacha20poly1305)
- Send/receive files through a Portal relay

The library is broken up into two abstractions:

- A higher level API, exposted via the `Portal` struct, to facilitate automating transfers easily
- A lower level API, exposed via the `Protocol` struct, if you need access to lower-level facilities

### Higher Level API - Example of Sending a file:

```rust
use std::net::TcpStream;
use portal_lib::{Portal, Direction};

// Connect to the relay
let mut stream = TcpStream::connect("127.0.0.1:34254").unwrap();

// Init a portal
let mut portal = Portal::init(Direction::Sender,"id".into(), "password".into()).unwrap();

// The handshake must be performed first, otherwise
// there is no shared key to encrypt the file with
portal.handshake(&mut stream).unwrap();

// Optional: implement a custom callback to display how much
// has been transferred
fn progress(transferred: usize) {
    println!("sent {:?} bytes", transferred);
}

// Begin sending the file
portal.send_file(&mut stream, "/etc/passwd", Some(progress));
```

### Higher Level API - Example of Receiving a file:

```rust
use std::path::Path;
use std::net::TcpStream;
use portal_lib::{Portal,Direction};

let mut portal = Portal::init(Direction::Sender,"id".into(), "password".into()).unwrap();
let mut stream = TcpStream::connect("127.0.0.1:34254").unwrap();

// The handshake must be performed first, otherwise
// there is no shared key to encrypt the file with
portal.handshake(&mut stream).unwrap();

// Optional: User callback to confirm/deny a transfer. If
// none is provided, this will default accept the incoming file.
// Return true to accept, false to reject the transfer.
fn confirm_download(path: &str, size: u64) -> bool { true }

// Optional: implement a custom callback to display how much
// has been transferred
fn progress(transferred: usize) {
    println!("received {:?} bytes", transferred);
}

// Begin receiving the file into /tmp
portal.recv_file(&mut stream, Path::new("/tmp"), Some(confirm_download), Some(progress));
```

### Lower Level API - Example of SPAKE2 key negotiation:

```rust
use spake2::{Ed25519Group, Identity, Password, Spake2};

// Securely receive/derive your id & password for this session
let channel_id = String::from("myid");
let password = String::from("mysecurepassword");

// Init a Spake2 context
let (state, outbound_msg) = Spake2::<Ed25519Group>::start_symmetric(
    &Password::new(&password.as_bytes()),
    &Identity::new(&channel_id.as_bytes()),
);

// Connect to the relay
let mut stream = TcpStream::connect("127.0.0.1:34254").unwrap();

// Send the connection message to the relay. If the relay cannot
// match us with a peer this will fail.
let confirm =
    Protocol::connect(&mut stream, &channel_id, Direction::Sender, outbound_msg).unwrap();

// Derive the shared session key
let key = Protocol::derive_key(state, &confirm).unwrap();

// confirm that the peer has the same key
Protocol::confirm_peer(&mut stream, &channel_id, Direction::Sender, &key)?;
```

You can use the confirm_peer() method to verify that a remote peer has derived the same key as you, as long as the communication stream implements the std::io::Read and std::io::Write traits.
