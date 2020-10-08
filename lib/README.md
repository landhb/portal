# portal-lib

A small Protocol Library for [Portal](https://github.com/landhb/portal) - An encrypted file transfer utility 

This crate enables a consumer to: 

- Create/serialize/deserialize Portal request/response messages.
- Negoticate a symmetric key with a peer using [SPAKE2](https://docs.rs/spake2/0.2.0/spake2) 
- Encrypt files with [Chacha20poly1305](https://blog.cloudflare.com/it-takes-two-to-chacha-poly/) using the [RustCrypto implementation](https://github.com/rusticata/tls-parser)
- Send/receive files through a Portal relay


### Example of SPAKE2 key negotiation:

```rust
use portal_lib::{Portal,Direction};

// receiver
let id = "id".to_string();
let pass ="test".to_string();
let (mut receiver,receiver_msg) = Portal::init(Direction::Receiver,id,pass,None);

// sender
let id = "id".to_string();
let pass ="test".to_string();
let (mut sender,sender_msg) = Portal::init(Direction::Sender,id,pass,None);

// Both clients should derive the same key
receiver.derive_key(&sender_msg).unwrap();
sender.derive_key(&receiver_msg).unwrap();
```

You can use the confirm_peer() method to verify that a remote peer has derived the same key as you, as long as the communication stream implements the std::io::Read and std::io::Write traits.

### Example of Sending a file:

```rust
use portal_lib::{Portal,Direction};
use std::net::TcpStream;
use std::io::Write;

let mut client = TcpStream::connect("127.0.0.1:34254").unwrap();

// Create portal request as the Sender
let id = "id".to_string();
let pass ="test".to_string();
let (mut portal,msg) = Portal::init(Direction::Sender,id,pass,None);

// complete key derivation + peer verification

let mut file = portal.load_file("/tmp/test").unwrap();

// Encrypt the file and share state 
file.encrypt().unwrap();
file.sync_file_state(&mut client).unwrap();

for data in file.get_chunks(portal_lib::CHUNK_SIZE) {
    client.write_all(&data).unwrap();
}
```

### Example of Receiving a file:

```rust
use portal_lib::{Portal,Direction};
use std::net::TcpStream;
use std::io::Write;

let mut client = TcpStream::connect("127.0.0.1:34254").unwrap();

// receiver
let dir = Direction::Receiver;
let pass ="test".to_string();
let (mut portal,msg) = Portal::init(dir,"id".to_string(),pass,None);

// serialize & send request
let request = portal.serialize().unwrap();
client.write_all(&request).unwrap();

// get response
let response = Portal::read_response_from(&client).unwrap();

// complete key derivation + peer verification

// create outfile
let fsize = response.get_file_size();
let mut file = portal.create_file("/tmp/test", fsize).unwrap();

let callback = |x| { println!("Received {} bytes",x); };

// Receive until connection is done
let len = file.download_file(&client,callback).unwrap();

assert_eq!(len as u64, fsize);

// Decrypt the file
file.decrypt().unwrap();
```

