use mio::net::TcpStream;
use portal_lib::Portal;
use std::error::Error;
use std::io::Write;

use std::time::SystemTime;
use os_pipe::{pipe,PipeReader,PipeWriter};
use mio::{Events, Ready, Poll, Token, PollOpt}; 


use crate::{ENDPOINTS,UNIQUE_TOKEN,Endpoint};
use crate::{networking,logging};

// increment the polling token by one
// for each new client connection
fn next(current: &mut Token) -> Token {
    let next = current.0;
    current.0 += 1;
    Token(next)
}

/**
 * Attempt to parse a Portal request from the client and match it 
 * with a peer. If matched, the pair will be added to an event loop 
 */
pub fn register(mut connection: TcpStream)  -> Result<(), Box<dyn Error>>  {

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
            Err(e) => {
                log!("{:?}", e);
                return Err(e.into());
            },
    };

    log!("req: {:?}", req);

    // Clear old entries before accepting, will keep
    // connections < 15 min old
    let mut ref_endpoints = ENDPOINTS.lock().unwrap();
    ref_endpoints.retain(|_, v| 
        !v.peer_token.is_none() || (
        v.time_added.elapsed().unwrap().as_secs() < 
        std::time::Duration::from_secs(60*15).as_secs())); 

    // Lookup existing endpoint with this ID
    let id = req.get_id();
    let search = ref_endpoints.iter()
            .find_map(|(key, val)| if *val.id == *id  { Some(key) } else { None });

    match req.get_direction() {

        portal::Direction::Receiver => {

            // Look for peer with identical ID
            let peer_token  = match search {
                Some(t) => t.clone(),
                None => {return Ok(());}
            };
            

            //let mut ref_endpoints = endpoints.borrow_mut();
            let mut peer = match ref_endpoints.get_mut(&peer_token) {
                Some(p) => p,
                None => {return Ok(());},
            };


            // if the peer already has a connection, disregard this one
            if !peer.peer_token.is_none() {
                let _ = connection.shutdown(std::net::Shutdown::Both);
                return Ok(());
            }
            
            // assign token since the peer is valid
            let token = next(&mut UNIQUE_TOKEN.lock().unwrap());
            
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
            //poll.register(&mut connection, token, Ready::readable()|Ready::writable(),PollOpt::level())?;
            //poll.register(&mut peer.stream, peer_token, Ready::readable()|Ready::writable(),PollOpt::level())?;

            // create this endpoint
            let endpoint = Endpoint {
                id: id.to_string(),
                dir: req.get_direction(),
                stream: connection,
                peer_reader: old_reader,
                peer_writer: Some(writer2), //None,
                peer_token: Some(peer_token),
                time_added: SystemTime::now(),
            };

            log!("Added Receiver {:?}", endpoint);

            ref_endpoints.entry(token).or_insert(endpoint);

        }
        portal::Direction::Sender => {

            // Kill the connection if this ID is being used by another sender
            match search {
                Some(_) => {return Ok(());}
                None => {}
            };

            let token = next(&mut UNIQUE_TOKEN.lock().unwrap());

            let (reader, mut writer) = pipe().unwrap();

            let resp = req.serialize()?;
            writer.write_all(&resp)?;

            let endpoint = Endpoint {
                id: req.get_id().to_string(),
                dir: req.get_direction(),
                stream: connection,
                peer_writer: Some(writer),
                peer_reader: Some(reader),
                peer_token: None,
                time_added: SystemTime::now(),
            };

            log!("Added Sender: {:?}", endpoint);
            
            ref_endpoints.entry(token).or_insert(endpoint);

        }

    }
    Ok(())
                    
}
