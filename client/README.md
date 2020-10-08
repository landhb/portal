# portal-client

This crate contains the client-side application for [Portal](https://github.com/landhb/portal) - An encrypted file transfer utility.

- Negoticate a symmetric key with a peer using [SPAKE2](https://docs.rs/spake2/0.2.0/spake2) 
- Encrypt files with [Chacha20poly1305](https://blog.cloudflare.com/it-takes-two-to-chacha-poly/) using the [RustCrypto implementation](https://docs.rs/chacha20poly1305)
- Send/receive files through a Portal relay

Note: The peer must connect to the same portal-relay as you.  The default relay is `portal-relay.landhb.dev` but can be changed in your config. On linux the config is most commonly located at `~/.config/portal/portal.toml`.

### Installation

```bash
cargo install portal-client
```

### Send a file


```bash
portal send /path/to/file
```

### Recv a file


```bash
portal recv
```

### Diagram of Key Derivation

![Demo](https://raw.githubusercontent.com/landhb/portal/master/img/key-derivation.png?raw=true)