
## Portal

Magically transport your files.

### Directory structure:

```
lib/        # implementation of the protocol
relay/      # relay server source
client/     # client source
```

### Run with:

```
# window 1
cargo run --bin portal-relay

# window 2
cargo run --bin portal -- recv

# window 3
cargo run --bin portal -- send -i [ID]
```