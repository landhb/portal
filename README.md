
## Portal 

[![crates.io](https://img.shields.io/crates/v/portal-lib.svg)](https://crates.io/crates/portal-lib)
[![Documentation](https://docs.rs/portal-lib/badge.svg)](https://docs.rs/portal-lib)
![Apache2/MIT licensed][license-image]
![Rust Version][rustc-image]

Securely & quickly transport your files.

### Client Install

```
cargo install --git https://github.com/landhb/portal portal-client
```

On the first run, a configuration file will be created in:

|Platform | Value                                 | Example                                  |
| ------- | ------------------------------------- | ---------------------------------------- |
| Linux   | `$XDG_CONFIG_HOME` or `$HOME`/.config | /home/alice/.config/portal                     |
| macOS   | `$HOME`/Library/Application Support   | /Users/Alice/Library/Application Support/portal |
| Windows | `{FOLDERID_RoamingAppData}`           | C:\Users\Alice\AppData\Roaming\portal           |


Note: The default relay is `portal-relay.landhb.dev`. Your peer must connect to the same portal-relay as you. You can change the relay to any domain/IP address in your config.


To send a file: 

```bash
portal send /path/to/file
```

To receive a file:

```bash
portal recv
```


### Relay Install

If you wish to run your own relay, you can install the binary on a server with:

```
cargo install --git https://github.com/landhb/portal portal-relay
```

Note: An example service file is included in the `relay/` directory.

### Development:

The repo is a cargo workspace with the following directory structure:

```
lib/        # implementation of the protocol
relay/      # relay server source
client/     # client source
```

You can run the binaries individually with:

```
# window 1
cargo run --bin portal-relay

# window 2
cargo run --bin portal -- recv

# window 3
cargo run --bin portal -- send [FILE]
```

### Acknowledgements:

This tool works extremely similarly to Wormhole written by [Brian Warner](https://github.com/warner), he actually also contributed a large amount to the [RustCrypto project's SPAKE2 implementation](https://github.com/RustCrypto/PAKEs/tree/master/spake2) that Portal uses. 


[//]: # (badges)
[license-image]: https://img.shields.io/badge/license-Apache2.0/MIT-blue.svg
[rustc-image]: https://img.shields.io/badge/rustc-1.41+-blue.svg
