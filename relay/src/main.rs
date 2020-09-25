//#[macro_use]
//extern crate lazy_static;
extern crate portal_lib as portal;

use portal::Portal;
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
use mio::{Events, Ready, Poll, Token, PollOpt}; //Interest, Poll, Token};
use os_pipe::{pipe,PipeReader,PipeWriter};

mod handlers;
mod networking;

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
    let poll = Poll::new()?;

    // Create storage for events.
    let mut events = Events::with_capacity(128);

    // Setup the server socket.
    let addr = "127.0.0.1:13265".parse()?;
    let mut server = TcpListener::bind(&addr)?;

    // Start listening for incoming connections.
    //poll.registry()
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

                    // If this is an event for the server, it means a connection
                    // is ready to be accepted.
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

                    let mut received_data = Vec::with_capacity(4096);
                    while received_data.len() == 0 {
                        match networking::recv_generic(&mut connection,&mut received_data) {
                            Ok(_) => {},
                            Err(_) => {
                                break;
                            }
                        }
                    }

                    println!("{:?}", received_data.len());

                    // attempt to recieve a portal request
                    let mut req = match Portal::parse(&received_data) {
                        Ok(r) => r,
                        Err(e) => {
                            println!("{:?}", e);
                            continue;
                        },
                    };

                    println!("req: {:?}", req);

                    match req.get_direction() {

                        Some(portal::Direction::Sender) => {

                            // ensure ID was provided
                            if req.get_id().is_none()  {
                                continue;
                            }

                            // create the pipe for this transfer
                            let (reader, writer) = pipe().unwrap();

                            // lookup reciever's ID
                            let peer_token = match lookup_token.get(&req.get_id().unwrap()) {
                                Some(p) => p,
                                None => {
                                    connection.shutdown(std::net::Shutdown::Both)?;
                                    continue;
                                },
                            };


                            let mut ref_endpoints = endpoints.borrow_mut();
                            
                            let mut peer = match ref_endpoints.get_mut(&peer_token) {
                                Some(p) => p,
                                None => {
                                    lookup_token.remove(&req.get_id().unwrap());
                                    connection.shutdown(std::net::Shutdown::Both)?;
                                    continue;
                                },
                            };

                            // if the peer already has a connection, disregard this one
                            if !peer.peer_reader.is_none() {
                                connection.shutdown(std::net::Shutdown::Both)?;
                                continue;
                            }
                            

                            // send the peer's public key to the sender
                            req.set_pubkey(Some(peer.pubkey.as_ref().unwrap().clone()));
                            let resp = req.serialize()?;
                            match networking::send_generic(&mut connection, resp) {
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
                            //poll.registry().register(&mut connection, token,Interest::READABLE)?;
                            poll.register(&mut connection, token, Ready::readable(),PollOpt::level())?;

                            // create this endpoint
                            let endpoint = Endpoint {
                                id: req.get_id(),
                                dir: req.get_direction().unwrap(),
                                pubkey: peer.pubkey.clone(),
                                stream: connection,
                                peer_reader: None,
                                peer_writer: Some(writer),
                                peer_token: Some(*peer_token),
                                
                            };

                            println!("Added sender {:?}", endpoints);

                            ref_endpoints.entry(token).or_insert(endpoint);

                        }
                        Some(portal::Direction::Reciever) => {

                            // check that pubkey was provided & send the unique ID
                            // associated with this key upload
                            if req.get_pubkey() == None {
                                continue;
                            }

                            // Generate a unique ID for this reciever
                            let uuid = Uuid::new_v4().to_hyphenated().to_string();
                            let key_field = req.get_pubkey();

                            // send confirmation of registration and unique ID
                            req.set_id(uuid.clone());
                            let resp = req.serialize()?;
                            match networking::send_generic(&mut connection, resp) {
                                Ok(_) => {},
                                Err(e) => {
                                    println!("error while sending resp {:?}", e);
                                    continue;
                                }
                            }

                            println!("[+] Sent response {:?}", req);

                            // We need to register for READABLE events to detect a closed connection
                            let token = next(&mut unique_token);
                            //poll.registry().register(&mut connection, token,Interest::READABLE)?;
                            poll.register(&mut connection, token, Ready::readable(),PollOpt::level())?;
                            
                            let endpoint = Endpoint {
                                id: Some(uuid.clone()),
                                dir: req.get_direction().unwrap(),
                                pubkey: key_field,
                                stream: connection,
                                peer_writer: None,
                                peer_reader: None,
                                peer_token: None,
                            };

                            println!("{:?}", endpoint);
                            
                            endpoints.borrow_mut().entry(token).or_insert(endpoint);
                            lookup_token.entry(uuid).or_insert(token);

                        }

                        _ => {continue;}

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

                    // save the session id
                    let id = client.id.as_ref().unwrap().to_string();

                    // perform the action
                    let done = handlers::handle_client_event(&poll, client, &event)?;

                    println!("handler finished {:?}", done);

                    // if we read in new data from the sender
                    // we are now interested in WRITEABLE events for our reciever 
                    if event.readiness().is_readable() && client.dir == portal::Direction::Sender {

                        let token_val = client.peer_token.as_ref().unwrap().0;

                        // get the corresponding peer
                        let peer = match ref_endpoints.get_mut(&Token(token_val)) {
                            Some(p) => p,
                            None => {
                                continue;
                            },
                        };

                        match poll.register(&mut peer.stream, Token(token_val),Ready::writable(),PollOpt::level()) {
                            Ok(_) => {},
                            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                                poll.reregister(&mut peer.stream, Token(token_val),Ready::writable(),PollOpt::level())?;
                            },
                            Err(e) => {panic!("{:?}",e);},
                        }

                    } 

                    // only the reciever should remove the lookup token on close
                    if done {
                        println!("Removing endpoint for {:?}", token);
                        lookup_token.remove(&id);
                        ref_endpoints.remove(&token);
                    }
                    
                }
            }
        }
    }
}