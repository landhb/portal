extern crate portal_lib as portal;

use portal::Direction;
use std::collections::HashMap;
use std::error::Error;
use std::cell::{RefCell};
use std::rc::Rc;
use mio::net::{TcpListener, TcpStream};
use mio::{Events, Ready, Poll, Token, PollOpt}; 
use os_pipe::{PipeReader,PipeWriter};
use std::sync::Mutex;
use threadpool::ThreadPool;
use std::time::SystemTime;
use mio_extras::channel::channel;

#[macro_use]
extern crate lazy_static;

mod handlers;
mod networking;

#[macro_use]
mod logging;
mod protocol;

use protocol::register;

// Some tokens to allow us to identify which event is for which socket.
const SERVER: Token = Token(0);
const CHANNEL: Token = Token(1);

lazy_static! {
    static ref PENDING_ENDPOINTS: Mutex<HashMap<String, Endpoint>> = Mutex::new(HashMap::new());
}

#[derive(Debug)]
pub struct Endpoint {
    id: String,
    dir: portal::Direction,
    stream: TcpStream,
    peer_writer: Option<PipeWriter>,
    peer_reader: Option<PipeReader>,
    token: Option<Token>,
    time_added: SystemTime,
}

#[derive(Debug)]
pub struct EndpointPair {
    sender: Endpoint,
    receiver: Endpoint,
}

#[cfg(not(debug_assertions))]
fn daemonize() -> Result<(),daemonize::DaemonizeError> {
    use daemonize::Daemonize;


    let stdout = std::fs::File::create("/tmp/relay.out").unwrap();
    let stderr = std::fs::File::create("/tmp/relay.err").unwrap();

    let daemonize = Daemonize::new()
        .pid_file("/tmp/relay.pid") 
        .chown_pid_file(false)      
        .working_directory("/tmp") 
        .umask(0o777)   
        .stdout(stdout)   // Redirect stdout to `/tmp/relay.out`.
        .stderr(stderr);  // Redirect stderr to `/tmp/relay.err`.

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

    // Only daemonize in release
    #[cfg(not(debug_assertions))]
    daemonize()?;
    
    // Create a poll instance.
    let poll = Poll::new()?;

    // Create storage for events.
    let mut events = Events::with_capacity(128);

    // Setup the server socket.
    let addr = format!("0.0.0.0:{}",portal::DEFAULT_PORT).parse()?;
    let mut server = TcpListener::bind(&addr)?;

    // Start listening for incoming connections.
    poll.register(&mut server, SERVER, Ready::readable(), PollOpt::edge())?;

    // Pre-allocate a few registration threads
    let thread_pool = ThreadPool::new(4);

    // Create a channel to receive pairs from threads
    let (tx, mut rx) = channel::<EndpointPair>();
    poll.register(&mut rx, CHANNEL, Ready::readable(), PollOpt::edge())?;

    // Active endpoint pairs
    let id_lookup: Rc<RefCell<HashMap<Token, String>>> = Rc::new(RefCell::new(HashMap::new()));
    let endpoints: Rc<RefCell<HashMap<String, EndpointPair>>> = Rc::new(RefCell::new(HashMap::new()));


    let mut unique_token = Token(CHANNEL.0+1);

    // Start an event loop.
    loop {

        // Poll Mio for events, blocking until we get an event.
        poll.poll(&mut events, None)?;

        // Process each event.
        for event in events.iter() {


            match event.token() {

                /*
                 * When receiving an incoming connection, use the threadpool to accept
                 * Portal requests without blocking the main loop
                 */
                SERVER => loop {

                    // If this is an event for the server, it means a connection
                    // is ready to be accepted.
                    let (connection, _addr) = match server.accept() {
                        Ok((s, addr)) => (s,addr),
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            // go back to polling for connections
                            break;
                        }
                        Err(e) => {
                            return Err(Box::new(e));
                        }
                    };

                    log!("[+] Got connection from {:?}", _addr);

                    
                    // TODO set RECV_TIMEO
                    let tx_new = tx.clone();
                    thread_pool.execute(move || {
                        match register(connection,tx_new) {
                            Ok(_) => {},
                            Err(_e) => {
                                log!("{}",_e);
                            }
                        }
                    });
                }
                /*
                 * When a worker thread has completed pairing two peers, the EndpointPair
                 * will be send over an MPSC channel to be added to the list of file descriptors
                 * we're polling 
                 */
                CHANNEL => {
                    let mut pair = match rx.try_recv() {
                        Ok(p) => p,
                        Err(_) => {continue;},
                    };

                    println!("ADDING PAIR {:?}", pair);

                    let sender_token = next(&mut unique_token);
                    let receiver_token = next(&mut unique_token);

                    pair.sender.token = Some(sender_token);
                    pair.receiver.token = Some(receiver_token);

                    poll.register(&mut pair.sender.stream, sender_token, Ready::readable()|Ready::writable(),PollOpt::edge())?;
                    poll.register(&mut pair.receiver.stream, receiver_token, Ready::readable(),PollOpt::level())?;

                    id_lookup.borrow_mut().entry(sender_token).or_insert(pair.sender.id.clone());
                    id_lookup.borrow_mut().entry(receiver_token).or_insert(pair.sender.id.clone());
                    endpoints.borrow_mut().entry(pair.sender.id.clone()).or_insert(pair);

                    println!("SUCCESS");
                }
                /*
                 * Any other events indicate there is data we need to channel between two TCP connections
                 * at this time we primarily use splice() to do that
                 */
                token => {

                    let mut ref_endpoints = endpoints.borrow_mut();
                    let lookup = id_lookup.borrow();

                    let id = match lookup.get(&token) {
                        Some(id) => id,
                        None => {
                            continue;
                        },
                    };

                    // get the EndpointPair that generated the event
                    let pair = match ref_endpoints.get_mut(id) {
                        Some(p) => p,
                        None => {
                            continue;
                        },
                    };

                    // determine which Endpoint is readable
                    let (side,stream,endpoint) = match token {
                        x if Some(x) == pair.sender.token => {(Direction::Sender,&pair.sender.stream, &pair.sender)},
                        x if Some(x) == pair.receiver.token => {(Direction::Receiver,&pair.receiver.stream, &pair.receiver)},
                        _ => {continue;},
                    };


                    log!("event {:?} on token {:?}, side: {:?}", event, token, side);

                    // Turn off writable notifications if on, this is only used to kick off the 
                    // initial message exchange by draining on of the peer's pipes
                    if event.readiness().is_writable() {
                        handlers::drain_pipe(endpoint)?;
                        poll.reregister(stream, token, Ready::readable(),PollOpt::level())?;
                    }

                    // perform the action
                    let done = handlers::tcp_splice(side, pair)?;

                    log!("handler finished {:?}", done); 

                    // If this connection is finished, or our peer has disconnected
                    // shutdown the connection
                    if done {
                        log!("Removing endpoint for {:?}", pair);
                        poll.deregister(&mut pair.sender.stream)?;
                        poll.deregister(&mut pair.receiver.stream)?;
                        match pair.sender.stream.shutdown(std::net::Shutdown::Both) {
                                Ok(_) => {},
                                Err(_) => {},
                        }
                        match pair.receiver.stream.shutdown(std::net::Shutdown::Both) {
                                Ok(_) => {},
                                Err(_) => {},
                        }
                    } 

                }
            }
        }
    }
}
