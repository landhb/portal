
## Portal  
[![cargo-badge-lib][]][cargo-lib] [![docs-badge-lib][]][docs-lib] [![license-badge][]][license] [![rust-version-badge][]][rust-version]  

Securely & quickly transport your files.


### Client Install 
[![cargo-badge-client][]][cargo-client] 

The client binary can be installed via Cargo:

```
cargo install portal-client
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
[license-badge]: https://img.shields.io/badge/license-MIT/Apache--2.0-lightgray.svg?style=flat-square
[license]: #license
[rust-version-badge]: https://img.shields.io/badge/rust-latest%20stable-blue.svg?style=flat-square
[rust-version]: #rust-version-policy

[cargo-badge-client]: https://img.shields.io/crates/v/portal-client.svg?style=flat-square
[cargo-client]: https://crates.io/crates/portal-client
[cargo-badge-lib]: https://img.shields.io/crates/v/portal-lib.svg?style=flat-square
[cargo-lib]: https://crates.io/crates/portal-lib

[docs-badge-client]: https://docs.rs/portal-client/badge.svg?style=flat-square
[docs-client]: https://docs.rs/portal-client
[docs-badge-lib]: https://docs.rs/portal-lib/badge.svg?style=flat-square
[docs-lib]: https://docs.rs/portal-lib
