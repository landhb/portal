//#[macro_use]
//extern crate lazy_static;
extern crate portal_lib as portal;
use std::collections::HashMap;
use std::error::Error;

// use another way to generate the ID
// ideally more human readable/shareable
// + easy to type
use uuid::Uuid;
//use std::sync::Mutex;
use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Token};
//use std::os::unix::io::{RawFd,AsRawFd};
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

    let mut endpoints: HashMap<Token, Endpoint> = HashMap::new();
    

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

                    let token = next(&mut unique_token);

                    println!("{:?}", req);

                    match req.direction {

                        portal::Direction::Sender => {

                            // ensure ID was provided
                            if req.id == None  {
                                continue;
                            }

                            // create the pipe for this transfer
                            let (reader, writer) = pipe().unwrap();

                            // lookup reciever's ID
                            let mut peer = None;
                            for (_token, mut client) in endpoints.iter_mut() {
                                if client.id == req.id {
                                    client.peer_writer = None;
                                    client.peer_reader = Some(reader);
                                    peer = Some(client);
                                    break;
                                }
                            }

                            if peer.is_none() {
                                continue;
                            }

                            // set socket to READABLE-interest only, after we confirm the existence
                            // of the receiver, this client will only be sending
                            //registry.reregister(connection, event.token(), Interest::READABLE)?
                            poll.registry().register(&mut connection, token,Interest::READABLE)?;


                            // create this endpoint
                            let endpoint = Endpoint {
                                id: req.id,
                                dir: req.direction,
                                pubkey: peer.unwrap().pubkey.clone(),
                                stream: connection,
                                peer_reader: None,
                                peer_writer: Some(writer),
                            };

                            println!("Added sender {:?}", endpoints);

                            endpoints.entry(token).or_insert(endpoint);

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
                      

                            // set socket to WRITEABLE-interest only, this is the reciever
                            poll.registry().register(&mut connection, token,Interest::WRITABLE)?;

                            
                            let endpoint = Endpoint {
                                id: Some(uuid),
                                dir: req.direction,
                                pubkey: key_field,
                                stream: connection,
                                peer_writer: None,
                                peer_reader: None,
                            };

                            println!("{:?}", endpoint);
                            endpoints.entry(token).or_insert(endpoint);
                        }
                    }
                }
                token => {

                    let done = if let Some(client) = endpoints.get_mut(&token) {
                        println!("calling handler for {:?}", client);
                        handlers::handle_client_event(poll.registry(), client, event)?
                    } else {
                        // Sporadic events happen, we can safely ignore them.
                        false
                    };
                    if done {
                        println!("Removing endpoint for {:?}", token);
                        endpoints.remove(&token);
                    }
                }
            }
        }
    }
}