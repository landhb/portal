use mio::net::TcpStream;
use std::error::Error;
use std::io::{self, Read};

fn would_block(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::WouldBlock
}

fn interrupted(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::Interrupted
}

pub fn recv_generic(
    connection: &mut TcpStream,
    received_data: &mut Vec<u8>,
) -> Result<isize, Box<dyn Error>> {
    loop {
        let mut buf = [0; 256];
        match connection.read(&mut buf) {
            Ok(0) => {
                // Reading 0 bytes means the other side has closed the
                // connection or is done writing, then so are we.
                //connection_closed = true;
                return Ok(-1);
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

    Ok(received_data.len() as isize)
}
