use anyhow::{Result,Error};

use std::io::{self,Read,Write};
//use std::error::Error;
use mio::net::TcpStream;
use serde::{Serialize, Deserialize};

mod errors;

use errors::PortalError;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum Direction {
    Sender,
    Reciever,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Request {
    pub direction: Direction,
    pub id: Option<String>,
    pub pubkey: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Response {
    pub id: String,
    pub pubkey: String,
}

fn would_block(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::WouldBlock
}

fn interrupted(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::Interrupted
}



fn recv_generic(connection: &mut TcpStream, received_data: &mut Vec<u8>) -> Result<usize> {
    //let mut connection_closed = false;
    //let mut received_data = Vec::with_capacity(4096);
    loop {
        let mut buf = [0; 256];
        match connection.read(&mut buf) {
            Ok(0) => {
                // Reading 0 bytes means the other side has closed the
                // connection or is done writing, then so are we.
                //connection_closed = true;
                break;
            }
            Ok(n) => received_data.extend_from_slice(&buf[..n]),
            // Would block "errors" are the OS's way of saying that the
            // connection is not actually ready to perform this I/O operation.
            Err(ref err) if would_block(err) => break,
            Err(ref err) if interrupted(err) => continue,
            // Other errors we'll consider fatal.
            Err(err) => return Err(err.into()),
        }
    }

    Ok(received_data.len())
}

fn send_generic(connection: &mut TcpStream, data: Vec<u8>) -> Result<()> {

    match connection.write(&data) {
        // We want to write the entire `DATA` buffer in a single go. If we
        // write less we'll return a short write error (same as
        // `io::Write::write_all` does).
        Ok(n) if n < data.len() => return Err(Error::new(PortalError::BadRegistration)),
        Ok(_) => {
            Ok(())
        }
        // Would block "errors" are the OS's way of saying that the
        // connection is not actually ready to perform this I/O operation.
        Err(ref err) if would_block(err) => {
            return Err(PortalError::WouldBlock.into())
        }
        // Got interrupted (how rude!), we'll try again.
        Err(ref err) if interrupted(err) => {
            //return handle_connection_event(registry, connection, event)
            return Err(PortalError::Interrupted.into())
        }
        // Other errors we'll consider fatal.
        Err(err) => return Err(err.into()),
    }
}

pub fn portal_get_request(mut connection: &mut TcpStream) -> Result<Request> { 

    let mut received_data = Vec::with_capacity(4096);
    recv_generic(&mut connection,&mut received_data)?;

    let req: Request = bincode::deserialize(&received_data)?;

    Ok(req)
    
}

pub fn portal_get_response(mut connection: &mut TcpStream) -> Result<Option<Response>> { 

    
    let mut received_data = Vec::with_capacity(4096);
    recv_generic(&mut connection,&mut received_data)?;

    if received_data.len() == 0 {
        return Ok(None);
    }   

    let resp: Response = bincode::deserialize(&received_data)?;

    Ok(Some(resp))
    
}

pub fn portal_send_response(connection: &mut TcpStream, id: String, pubkey: Option<String>) -> Result<()> {
    let response = Response {
        id: id,
        pubkey: pubkey.unwrap(),
    };
    println!("{:?}", response);
    let encoded: Vec<u8> = bincode::serialize(&response)?;
    return send_generic(connection,encoded);
}


pub fn portal_send_request(connection: &mut TcpStream, request: Request) -> Result<()> {
    let encoded: Vec<u8> = bincode::serialize(&request)?;
    return send_generic(connection,encoded);
}



pub fn portal_send_data(connection: &mut TcpStream, data: Vec<u8>) -> Result<()> {

    // do encryption here

    return send_generic(connection,data);
}

pub fn portal_recv_data(mut connection: &mut TcpStream, mut data: &mut Vec<u8>) -> Result<usize> {
    //let received_data = recv_generic(&mut connection)?;


    //let mut received_data = Vec::with_capacity(4096);
    recv_generic(&mut connection,&mut data)?;
    // do decryption here
    Ok(data.len())    
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
