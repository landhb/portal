extern crate portal_lib as portal;

use portal::Portal;
use std::collections::HashMap;
use std::error::Error;
use std::cell::{RefCell};
use std::rc::Rc;
use std::io::Write;
use mio::net::{TcpListener, TcpStream};
use mio::{Events, Ready, Poll, Token, PollOpt}; 
use os_pipe::{pipe,PipeReader,PipeWriter};
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
    static ref ENDPOINTS: Mutex<HashMap<Token, Endpoint>> = Mutex::new(HashMap::new());
    static ref UNIQUE_TOKEN: Mutex<Token> = Mutex::new(Token(CHANNEL.0+1));
}

#[derive(Debug)]
pub struct Endpoint {
    id: String,
    dir: portal::Direction,
    stream: TcpStream,
    peer_writer: Option<PipeWriter>,
    peer_reader: Option<PipeReader>,
    peer_token: Option<Token>,
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

    // Start an event loop.
    loop {

        // Poll Mio for events, blocking until we get an event.
        poll.poll(&mut events, None)?;

        // Process each event.
        for event in events.iter() {


            match event.token() {

                SERVER => loop {

                    // If this is an event for the server, it means a connection
                    // is ready to be accepted.
                    let (mut connection, _addr) = match server.accept() {
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
                CHANNEL => {
                    let pair = match rx.try_recv() {
                        Ok(p) => p,
                        Err(_) => {continue;},
                    };

                    println!("ADDING PAIR {:?}", pair);
                }
                token => {

                    let mut ref_endpoints = ENDPOINTS.lock().unwrap(); //endpoints.borrow_mut();

                    // get the client that will be performing the read/write
                    let client = match ref_endpoints.get_mut(&token) {
                        Some(p) => p,
                        None => {
                            continue;
                        },
                    };

                    // check that the client has a peer first
                    let peer_token = match client.peer_token {
                        Some(v) => v,
                        None => {continue;},
                    };


                    log!("event {:?} on token {:?}, peer: {:?}", event, token, peer_token);


                    // perform the action
                    let (done,trx) = handlers::handle_client_event(token,&poll, client, &event)?;

                    log!("handler finished {:?}", done); 

                    // If this connection is finished, or our peer has disconnected
                    // shutdown the connection
                    if done || (trx <= 0 && !ref_endpoints.contains_key(&peer_token)) {
                        log!("Removing endpoint for {:?}", token);
                        if let Some(mut client) = ref_endpoints.remove(&token) {
                            poll.deregister(&mut client.stream)?;
                            match client.stream.shutdown(std::net::Shutdown::Both) {
                                Ok(_) => {},
                                Err(_) => {},
                            }
                        }
                    }

                }
            }
        }
    }
}
