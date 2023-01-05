extern crate portal_lib as portal;

use env_logger::Env;
use lazy_static::lazy_static;
use mio::net::{TcpListener, TcpStream};
use mio::{Events, Poll, PollOpt, Ready, Token};
use mio_extras::channel::channel;
use os_pipe::{PipeReader, PipeWriter};
use portal::Direction;
use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::fs::OpenOptions;
use std::rc::Rc;
use std::sync::Mutex;
use std::time::SystemTime;
use structopt::StructOpt;
use threadpool::ThreadPool;

mod errors;
mod ffi;
mod tunnel;
use tunnel::Tunnel;

mod networking;

extern crate env_logger;

mod protocol;

// Some tokens to allow us to identify which event is for which socket.
const SERVER: Token = Token(0);
const CHANNEL: Token = Token(1);

/// Pipe Size for Splicing
///
/// From the cloudfare blog:
/// There is no "good" splice buffer size. Anecdotical evidence
/// says that it should be no larger than 512KiB since this is
/// the max we can expect realistically to fit into cpu
/// cache.
const MAX_SPLICE_SIZE: usize = 512 * 1024;

lazy_static! {
    /// Map of pending endpoints to correlate, keyed by ID
    static ref PENDING_ENDPOINTS: Mutex<HashMap<String, Endpoint>> = Mutex::new(HashMap::new());
}

#[derive(Debug, StructOpt)]
#[structopt(name = "portal-relay", about = "A relay for Portal.")]
struct Opt {
    /// Activate daemon mode
    /// short and long flags (-b, --background)
    #[structopt(short, long)]
    background: bool,
}

#[derive(Debug)]
pub struct Endpoint {
    id: String,
    dir: portal::Direction,
    stream: TcpStream,
    peer_writer: Option<PipeWriter>,
    peer_reader: Option<PipeReader>,
    has_peer: bool,
    time_added: SystemTime,
}

/// An established pair of endpoints.
///
/// When fully constructed the two endpoints have been matched
/// and each has an active connection to the relay.
#[derive(Debug)]
pub struct EndpointPair {
    /// The sender and active mio Token
    sender: Endpoint,
    sender_token: Token,
    /// The receiver and it's active mio Token
    receiver: Endpoint,
    receiver_token: Token,
}

fn daemonize() -> Result<(), daemonize::DaemonizeError> {
    use daemonize::Daemonize;

    let stdout = OpenOptions::new()
        .append(true)
        .create(true)
        .open("/tmp/relay.out")
        .unwrap();
    let stderr = OpenOptions::new()
        .append(true)
        .create(true)
        .open("/tmp/relay.err")
        .unwrap();

    let daemonize = Daemonize::new()
        .pid_file("/tmp/relay.pid")
        .chown_pid_file(false)
        .working_directory("/tmp")
        .umask(0o777)
        .stdout(stdout) // Redirect stdout to `/tmp/relay.out`.
        .stderr(stderr); // Redirect stderr to `/tmp/relay.err`.

    daemonize.start()
}

// increment the polling token by one
// for each new client connection
pub fn next(current: &mut Token) -> Token {
    let next = current.0;
    current.0 += 1;
    Token(next)
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();

    // Only daemonize if given --background
    if opt.background {
        daemonize()?;
    }

    // Initialize logging
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .default_format()
        .format_target(false)
        .init();

    log::info!("Starting portal relay");

    // Create a poll instance.
    let poll = Poll::new()?;

    // Create storage for events.
    let mut events = Events::with_capacity(128);

    // Setup the server socket.
    let addr = format!("0.0.0.0:{}", portal::DEFAULT_PORT).parse()?;
    let server = TcpListener::bind(&addr)?;

    log::info!("Listening on {}", addr);

    // Start listening for incoming connections.
    poll.register(&server, SERVER, Ready::readable(), PollOpt::edge())?;

    // Pre-allocate a few registration threads
    let thread_pool = ThreadPool::new(4);

    // Create a channel to receive pairs from threads
    let (tx, rx) = channel::<EndpointPair>();
    poll.register(&rx, CHANNEL, Ready::readable(), PollOpt::edge())?;

    // Active endpoint pairs
    let id_lookup: Rc<RefCell<HashMap<Token, String>>> = Rc::new(RefCell::new(HashMap::new()));
    let endpoints: Rc<RefCell<HashMap<String, EndpointPair>>> =
        Rc::new(RefCell::new(HashMap::new()));

    let mut unique_token = Token(CHANNEL.0 + 1);

    // Start an event loop.
    loop {
        // Poll Mio for events, blocking until we get an event.
        poll.poll(&mut events, None)?;

        // Process each event.
        for event in events.iter() {
            match event.token() {
                // New incoming connections:
                //
                // When receiving an incoming connection, use the threadpool to accept
                // Portal requests without blocking the main loop
                SERVER => loop {
                    // If this is an event for the server, it means a connection
                    // is ready to be accepted.
                    let (connection, addr) = match server.accept() {
                        Ok((s, addr)) => (s, addr),
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            // go back to polling for connections
                            break;
                        }
                        Err(e) => {
                            return Err(Box::new(e));
                        }
                    };

                    log::debug!("[+] New connection from {:?}", addr);

                    // Begin brokering the two endpoints
                    let broker = protocol::ConnectionBroker::new(addr, connection, tx.clone());

                    // Let the thread pool handle the pairing. TODO set RECV_TIMEO
                    thread_pool.execute(move || match broker.establish() {
                        Ok(_) => {}
                        Err(_e) => {
                            log::error!("Error creating portal: {}", _e);
                        }
                    });
                },
                // New pair created:
                //
                // When a worker thread has completed pairing two peers, the EndpointPair
                // will be sent over an MPSC channel to be added to the list of file descriptors
                // we're polling
                CHANNEL => {
                    while let Ok(mut pair) = rx.try_recv() {
                        pair.sender_token = next(&mut unique_token);
                        pair.receiver_token = next(&mut unique_token);

                        poll.register(
                            &pair.sender.stream,
                            pair.sender_token,
                            Ready::readable() | Ready::writable(),
                            PollOpt::edge(),
                        )?;
                        poll.register(
                            &pair.receiver.stream,
                            pair.receiver_token,
                            Ready::readable(),
                            PollOpt::level(),
                        )?;

                        id_lookup
                            .borrow_mut()
                            .entry(pair.sender_token)
                            .or_insert_with(|| pair.sender.id.clone());
                        id_lookup
                            .borrow_mut()
                            .entry(pair.receiver_token)
                            .or_insert_with(|| pair.sender.id.clone());
                        endpoints
                            .borrow_mut()
                            .entry(pair.sender.id.clone())
                            .or_insert_with(|| pair);
                    }
                }
                // I/O events for established pairs:
                //
                // Any other events indicate there is data we need to channel between two TCP
                // connections at this time we primarily use splice() to do that.
                token => {
                    let mut ref_endpoints = endpoints.borrow_mut();
                    let lookup = id_lookup.borrow();

                    let id = match lookup.get(&token) {
                        Some(id) => id,
                        None => {
                            continue;
                        }
                    }
                    .clone();

                    // get the EndpointPair that generated the event
                    let pair = match ref_endpoints.get_mut(&id) {
                        Some(p) => p,
                        None => {
                            continue;
                        }
                    };

                    drop(lookup);

                    // determine which Endpoint triggered the event
                    let (side, endpoint, peer) = match token {
                        x if x == pair.sender_token => {
                            (Direction::Sender, &mut pair.sender, &mut pair.receiver)
                        }
                        x if x == pair.receiver_token => {
                            (Direction::Receiver, &mut pair.receiver, &mut pair.sender)
                        }
                        _ => {
                            continue;
                        }
                    };

                    log::debug!("[{:.6}] {:?} Event: {:?}", id, side, event);

                    let mut tunnel = Tunnel::new(endpoint, peer)?;

                    let mut done = false;

                    // if we received data on this endpoint, splice it to the peer
                    if event.readiness().is_readable() {
                        done = tunnel.transfer_until_blocked()?;
                    }

                    // if we got a writable event, then there is pending data in the intermediary pipe
                    if event.readiness().is_writable() {
                        done = tunnel.transfer_until_blocked()?;

                        // Turn off writable notifications for the Sender if on, this is only used
                        // to kick off the initial message exchange by draining the initial pipe
                        if side == Direction::Sender {
                            poll.reregister(
                                &endpoint.stream,
                                token,
                                Ready::readable(),
                                PollOpt::level(),
                            )?;
                        }
                    }

                    log::debug!("[{:.6}] Handler finished. Done: {:?}", id, done);

                    // If this connection is finished, or our peer has disconnected
                    // shutdown the connection
                    if done {
                        // There may still be some data in the Receiver's pipe, drain it
                        // before closing the peer connection. We must register for writeable
                        // events in case the Receiver's socket is still blocking
                        if side == Direction::Sender {
                            match poll.reregister(
                                &peer.stream,
                                pair.receiver_token,
                                Ready::writable(),
                                PollOpt::edge(),
                            ) {
                                Ok(_) => {}
                                Err(e) => {
                                    log::error!("[{:.6}] Error: {:?}", id, e);
                                }
                            }
                        }

                        log::info!(
                            "[{:.6}] Removing {:?} connection",
                            endpoint.id,
                            endpoint.dir
                        );

                        // Shutdown this endpoint
                        poll.deregister(&endpoint.stream)?;
                        let id = id_lookup.borrow_mut().remove(&token);
                        if endpoint.stream.shutdown(std::net::Shutdown::Both).is_ok() {} // ignore shutdown errors

                        // close the write end of the pipe, otherwise splice() will continually
                        // return EWOULDBLOCK intead of knowing when there is no data left
                        let old_writer = std::mem::replace(&mut endpoint.peer_writer, None);
                        drop(old_writer);

                        // indicate to the peer that this endpoint is gone
                        peer.has_peer = false;

                        // If our peer is also gone, remove the entire EndpointPair
                        if !endpoint.has_peer {
                            let _ = ref_endpoints.remove(&id.unwrap_or_else(|| "none".to_string()));
                        }
                    }
                }
            }
        }
    }
}
