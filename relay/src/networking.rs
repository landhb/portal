use mio::net::TcpStream;
use std::io::{self,Read,Write};
use anyhow::{Result,Error};


use portal::errors::PortalError;

fn would_block(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::WouldBlock
}

fn interrupted(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::Interrupted
}


pub fn recv_generic(connection: &mut TcpStream, received_data: &mut Vec<u8>) -> Result<usize> {
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

pub fn send_generic(connection: &mut TcpStream, data: &Vec<u8>) -> Result<()> {

    match connection.write(data) {
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
