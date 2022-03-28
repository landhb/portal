# portal-relay

This crate contains the relay application for [Portal](https://github.com/landhb/portal) - An encrypted file transfer utility.

For the client utility go to:

- [portal-client](https://crates.io/crates/portal-client)

### Installation

```sh
cargo install portal-relay
```

When run the binary listend on TCP port 13265 to broker connections between clients.

### Diagram of Key Derivation

![Demo](https://raw.githubusercontent.com/landhb/portal/master/img/key-derivation.png?raw=true)


### Creating a Service on Alpine

First build a static relay binary and transfer it to the alpine system, then add a user for the service:

```sh
cross build --bin portal-relay --target x86_64-unknown-linux-musl --release

# upload and copy the binary to /sbin/portal-relay
chmod +x /sbin/portal-relay
useradd relay -M -N --system -s /sbin/nologin
```

On alpine linux you can setup a simple service file `vi /etc/init.d/relay`:

```sh
#!/sbin/openrc-run

command=/sbin/portal-relay
command_user="relay"
supervisor="supervise-daemon"

depend() {
        need net localmount
}
```

Then add the service to the default run-level to start on boot:

```sh
chmod +x /etc/init.d/relay
rc-update add relay
```

List services to verify the relay was enabled:

```sh
rc-status
```

Then start the service:

```sh
/etc/init.d/relay start
```