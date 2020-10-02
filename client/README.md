# portal-client

This crate contains the client-side application for [Portal](https://github.com/landhb/portal) - An encrypted file transfer utility.

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

