//#[macro_use]
//extern crate lazy_static;
extern crate portal_lib as portal;
use std::collections::HashMap;
use std::error::Error;
use std::cell::{RefCell};
use std::rc::Rc;

// use another way to generate the ID
// ideally more human readable/shareable
// + easy to type
use uuid::Uuid;
//use std::sync::Mutex;
use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Token};
use os_pipe::{pipe,PipeReader,PipeWriter};

mod handlers;

// Some tokens to allow us to identify which event is for which socket.
const SERVER: Token = Token(0);

#[derive(Debug)]
pub struct Endpoint {

    id: Option<String>,
    dir: portal::Direction,
    pubkey: Option<String>,
    stream: TcpStream,
    peer_writer: Option<PipeWriter>,
    peer_reader: Option<PipeReader>,
    peer_token: Option<Token>,
}


// increment the polling token by one
// for each new client connection
fn next(current: &mut Token) -> Token {
    let next = current.0;
    current.0 += 1;
    Token(next)
}

fn main() -> Result<(), Box<dyn Error>> {
    
    // Create a poll instance.
    let mut poll = Poll::new()?;

    // Create storage for events.
    let mut events = Events::with_capacity(128);

    // Setup the server socket.
    let addr = "127.0.0.1:13265".parse()?;
    let mut server = TcpListener::bind(addr)?;

    // Start listening for incoming connections.
    poll.registry()
        .register(&mut server, SERVER, Interest::READABLE)?;


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
            // We can use the token we previously provided to `register` to
            // determine for which socket the event is.
            match event.token() {

                SERVER => loop {

                    // If this is an event for the server, it means a connection
                    // is ready to be accepted.
                    //
                    // Accept the connection and drop it immediately. This will
                    // close the socket and notify the client of the EOF.
                    let (mut connection, addr) = match server.accept() {
                        Ok((s, addr)) => (s,addr),
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            // go back to polling for connections
                            break;
                        }
                        Err(e) => {
                            return Err(Box::new(e));
                        }
                    };

                    println!("[+] Got connection from {:?}", addr);

                    // add the connection to the endpoint registry
                    let req = portal::portal_get_request(&mut connection)?;

                    println!("req: {:?}", req);

                    match req.direction {

                        portal::Direction::Sender => {

                            // ensure ID was provided
                            if req.id == None  {
                                continue;
                            }

                            // create the pipe for this transfer
                            let (reader, writer) = pipe().unwrap();

                            // lookup reciever's ID
                            let peer_token = match lookup_token.get(req.id.as_ref().unwrap()) {
                                Some(p) => p,
                                None => {continue;},
                            };


                            let mut ref_endpoints = endpoints.borrow_mut();
                            
                            let mut peer = match ref_endpoints.get_mut(&peer_token) {
                                Some(p) => p,
                                None => {
                                    lookup_token.remove(req.id.as_ref().unwrap());
                                    continue;
                                },
                            };

                            // send the peer's public key to the sender
                            match portal::portal_send_response(&mut connection, 
                                                peer.id.as_ref().unwrap().clone(), 
                                                Some(peer.pubkey.as_ref().unwrap().clone())) {
                                Ok(_) => {},
                                Err(e) => {
                                    println!("error while sending resp {:?}", e);
                                    continue;
                                }
                            }

                            // assign token since the peer is valid
                            let token = next(&mut unique_token);

                            // update the peer with the pipe information
                            peer.peer_writer = None;
                            peer.peer_reader = Some(reader);


                            // set socket to READABLE-interest only, after we confirm the existence
                            // of the receiver, this client will only be sending
                            //registry.reregister(connection, event.token(), Interest::READABLE)?
                            poll.registry().register(&mut connection, token,Interest::READABLE)?;


                            // create this endpoint
                            let endpoint = Endpoint {
                                id: req.id,
                                dir: req.direction,
                                pubkey: peer.pubkey.clone(),
                                stream: connection,
                                peer_reader: None,
                                peer_writer: Some(writer),
                                peer_token: Some(*peer_token),
                                
                            };

                            println!("Added sender {:?}", endpoints);

                            ref_endpoints.entry(token).or_insert(endpoint);

                        }
                        portal::Direction::Reciever => {

                            // check that pubkey was provided & send the unique ID
                            // associated with this key upload
                            if req.pubkey == None {
                                continue;
                            }

                            // send confirmation of registration and unique ID
                            let uuid = Uuid::new_v4().to_hyphenated().to_string();
                            let key_field = req.pubkey.clone();
                            match portal::portal_send_response(&mut connection, uuid.clone(), req.pubkey) {
                                Ok(_) => {},
                                Err(e) => {
                                    println!("error while sending resp {:?}", e);
                                    continue;
                                }
                            }

                            
                            let endpoint = Endpoint {
                                id: Some(uuid.clone()),
                                dir: req.direction,
                                pubkey: key_field,
                                stream: connection,
                                peer_writer: None,
                                peer_reader: None,
                                peer_token: None,
                            };

                            println!("{:?}", endpoint);
                            let token = next(&mut unique_token);
                            endpoints.borrow_mut().entry(token).or_insert(endpoint);
                            lookup_token.entry(uuid).or_insert(token);
                        }
                    }
                }
                token => {
                    println!("event {:?} on token {:?}", event, token);

                    let mut ref_endpoints = endpoints.borrow_mut();

                    // get the client that will be performing the read/write
                    let client = match ref_endpoints.get_mut(&token) {
                        Some(p) => p,
                        None => {
                            continue;
                        },
                    };


                    // perform the action
                    let done = handlers::handle_client_event(poll.registry(), client, event)?;

                    // if we read in new data
                    // we are now interested in WRITEABLE events for our peer 
                    if event.is_readable() {

                        let token_val = client.peer_token.as_ref().unwrap().0;

                        // get the corresponding peer
                        let peer = match ref_endpoints.get_mut(&Token(token_val)) {
                            Some(p) => p,
                            None => {
                                continue;
                            },
                        };

                        match poll.registry().register(&mut peer.stream, Token(token_val),Interest::WRITABLE) {
                            Ok(_) => {},
                            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                                poll.registry().reregister(&mut peer.stream, Token(token_val),Interest::WRITABLE)?;
                            },
                            Err(e) => {panic!("{:?}",e);},
                        }
                    }

                    println!("finished handler, got {}", done);
                    if done {
                        println!("Removing endpoint for {:?}", token);
                        ref_endpoints.remove(&token);
                    }
                }
            }
        }
    }
}