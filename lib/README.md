# portal-lib

A small Protocol Library for [Portal](https://github.com/landhb/portal) - An encrypted file transfer utility 

This crate enables a consumer to: 

- Create/serialize/deserialize Portal request/response messages.
- Negoticate a symmetric key with a peer using [SPAKE2](https://docs.rs/spake2/0.2.0/spake2) 
- Encrypt files with [Chacha20poly1305](https://blog.cloudflare.com/it-takes-two-to-chacha-poly/) using the [RustCrypto implementation](https://github.com/rusticata/tls-parser)
- Send/receive files through a Portal relay


### Example of SPAKE2 key negotiation:

```rust
// receiver
let dir = Some(Direction::Receiver);
let pass ="test".to_string();
let (mut receiver,receiver_msg) = Portal::init(dir,"id".to_string(),pass,None);

// sender
let dir = Some(Direction::Sender);
let pass ="test".to_string();
let (mut sender,sender_msg) = Portal::init(dir,"id".to_string(),pass,None);

receiver.confirm_peer(&sender_msg).unwrap();
sender.confirm_peer(&receiver_msg).unwrap();

assert_eq!(receiver.key,sender.key);
```

### Example of Sending a file:

```rust
// open file read-only for sending
let mut file = portal.load_file(fpath)?;

// Encrypt the file and share state 
file.encrypt()?;
file.sync_file_state(&mut client)?;

// Get an iterator over the file in chunks
let chunks = portal.get_chunks(&file,portal::CHUNK_SIZE);

// Iterate over the chunks sending the via the client TcpStream 
for data in chunks.into_iter() {
    client.write_all(&data)?;
    total += data.len(); 
}
```

### Example of Receiving a file:

```rust
// create outfile
let mut file = portal.create_file(&fname, fsize)?;

// Receive until connection is done
let len = match file.download_file(&client,|x| {pb.set_position(x)})?;

assert_eq!(len as u64, fsize);

// Decrypt the file
file.decrypt()?;
```

