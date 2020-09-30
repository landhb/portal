//#[macro_use]
//extern crate lazy_static;
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
use std::time::SystemTime;

mod handlers;
mod networking;
mod logging;

// Some tokens to allow us to identify which event is for which socket.
const SERVER: Token = Token(0);

#[derive(Debug)]
pub struct Endpoint {
    id: String,
    dir: portal::Direction,
    stream: TcpStream,
    writable: bool,
    peer_writer: Option<PipeWriter>,
    peer_reader: Option<PipeReader>,
    peer_token: Option<Token>,
    time_added: SystemTime,
}


// increment the polling token by one
// for each new client connection
fn next(current: &mut Token) -> Token {
    let next = current.0;
    current.0 += 1;
    Token(next)
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
    poll.register(&mut server, SERVER, Ready::readable(), PollOpt::level())?;

    let mut unique_token = Token(SERVER.0+1);

    // Use to reference existing endpoints and their polling tokens
    let endpoints: Rc<RefCell<HashMap<Token, Endpoint>>> = Rc::new(RefCell::new(HashMap::new()));
    let mut lookup_token: HashMap<String, Token> = HashMap::new();

    
    // Start an event loop.
    loop {

        // Poll Mio for events, blocking until we get an event.
        poll.poll(&mut events, None)?;

        // Process each event.
        for event in events.iter() {


            match event.token() {

                SERVER => loop {

                    // Clear old entries before accepting, will keep
                    // connections < 15 min old
                    endpoints.borrow_mut().retain(|_, v| 
                        v.time_added.elapsed().unwrap().as_secs() < 
                        std::time::Duration::from_secs(60*15).as_secs());

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

                    
                    let mut received_data = Vec::with_capacity(1024);
                    while received_data.len() == 0 {
                        match networking::recv_generic(&mut connection,&mut received_data) {
                            Ok(v) if v < 0 => {break;}, // done recieving
                            Ok(_) => {},
                            Err(_) => {
                                break;
                            }
                        }
                    }

                    log!("{:?}", received_data.len());

                    // attempt to recieve a portal request
                    let req: Portal = match Portal::parse(&received_data) {
                            Ok(r) => r,
                            Err(_e) => {
                                log!("{:?}", _e);
                                continue;
                            },
                    };

                    log!("req: {:?}", req);

                    match req.get_direction() {

                        Some(portal::Direction::Receiver) => {

                            // Lookup Sender token
                            let id = req.get_id();
                            let ref_endpoints = endpoints.borrow();
                            let search = ref_endpoints.iter()
                            .find_map(|(key, val)| if *val.id == *id { Some(key) } else { None });

                            let peer_token  = match search {
                                Some(t) => t.clone(),
                                None => {continue;}
                            };
                            
                            // drop the immutable reference because we need a mutable one
                            drop(ref_endpoints);

                            let mut ref_endpoints = endpoints.borrow_mut();
                            let mut peer = match ref_endpoints.get_mut(&peer_token) {
                                Some(p) => p,
                                None => {continue;},
                            };


                            // if the peer already has a connection, disregard this one
                            if !peer.peer_token.is_none() {
                                let _ = connection.shutdown(std::net::Shutdown::Both);
                                continue;
                            }
                            
                            // assign token since the peer is valid
                            let token = next(&mut unique_token);
                            
                            // create the pipes for this transfer
                            let (reader2, mut writer2) = pipe().unwrap();
                            
                            // write the acknowledgement response to both pipe endpoints
                            let resp = req.serialize()?;
                            writer2.write_all(&resp)?;

                            log!("Finished writing to pipes");

                            // update the peer with the pipe information
                            let old_reader = std::mem::replace(&mut peer.peer_reader, Some(reader2));
                            peer.peer_token = Some(token);
                            
                            // set socket to WRITABLE-interest initially to drain the pipes we just
                            // wrote the acknowledgment messages to
                            poll.register(&mut connection, token, Ready::readable()|Ready::writable(),PollOpt::level())?;
                            poll.register(&mut peer.stream, peer_token, Ready::readable()|Ready::writable(),PollOpt::level())?;

                            // create this endpoint
                            let endpoint = Endpoint {
                                id: id.to_string(),
                                dir: req.get_direction().unwrap(),
                                stream: connection,
                                writable: true,
                                peer_reader: old_reader,
                                peer_writer: Some(writer2), //None,
                                peer_token: Some(peer_token),
                                time_added: SystemTime::now(),
                            };

                            log!("Added sender {:?}", endpoints);

                            ref_endpoints.entry(token).or_insert(endpoint);
                            lookup_token.entry(id.to_string()).or_insert(token);

                        }
                        Some(portal::Direction::Sender) => {


                            // We need to register for READABLE events to detect a closed connection
                            let token = next(&mut unique_token);

                            let (reader, mut writer) = pipe().unwrap();

                            let resp = req.serialize()?;
                            writer.write_all(&resp)?;

                            let endpoint = Endpoint {
                                id: req.get_id().to_string(),
                                dir: req.get_direction().unwrap(),
                                stream: connection,
                                writable: true,
                                peer_writer: Some(writer),
                                peer_reader: Some(reader),
                                peer_token: None,
                                time_added: SystemTime::now(),
                            };

                            log!("{:?}", endpoint);
                            
                            endpoints.borrow_mut().entry(token).or_insert(endpoint);

                        }

                        _ => {continue;}

                    }
                    
                }
                token => {
                    
                    
                    let mut ref_endpoints = endpoints.borrow_mut();

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

                    log!("event {:?} on token {:?}", event, token);

                    // save the session id
                    let id = client.id.clone();

                    // perform the action
                    let (done,trx) = handlers::handle_client_event(token,&poll, client, &event)?;

                    log!("handler finished {:?}", done); 


                    // If this is the Reciever, and we've received the last message
                    // to be read, we're done sending to our peer Sender after the next
                    // writeable event
                    if client.dir == portal::Direction::Receiver && event.readiness().is_readable() {
                        if let Some(peer) = ref_endpoints.get_mut(&peer_token) {
                            peer.writable = false;
                        }
                    }


                    drop(ref_endpoints);

                    // If this connection is finished, or our peer has disconnected
                    // shutdown the connection
                    if done || (trx <= 0 && !endpoints.borrow().contains_key(&peer_token)) {
                        log!("Removing endpoint for {:?}", token);
                        lookup_token.remove(&id);
                        if let Some(mut client) = endpoints.borrow_mut().remove(&token) {
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
