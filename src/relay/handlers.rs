extern crate portal_lib as portal;


use mio::Registry;
use crate::Endpoint;
use mio::event::Event;
use mio::Interest;
use anyhow::Result;
use std::os::unix::io::AsRawFd;

pub fn handle_client_event (
    registry: &Registry,
    endpoint: &mut Endpoint,
    event: &Event) -> Result<bool> {


    if event.is_writable() {
        // We can (likely) write to the socket without blocking.
        println!("reading for {:?}", endpoint);
        let reader = match &endpoint.peer_reader {
            Some(p) => p,
            None => {
                // end this connection if there is no peer pipe
                //registry.deregister(&mut endpoint.stream)?;
                return Ok(false);
            }
        };


        let dst_fd = endpoint.stream.as_raw_fd();
        let src_fd = reader.as_raw_fd();

        let read;
        unsafe {
            read = libc::splice(src_fd, 0 as *mut libc::loff_t, dst_fd, 0 as *mut libc::loff_t, 2048, libc::SPLICE_F_NONBLOCK);    
        }


        // check if connection is closed
        if read < 0 {
            return Ok(true);
        }

        // if we drained the pipe, deregister the stream
        // will be interested again when something is written
        if read == 0 {
            registry.deregister(&mut endpoint.stream)?;
        }

        println!("read {} bytes from pipe", read);


        
    }

    if event.is_readable() {
        // We can (likely) read from the socket without blocking.

        let writer = match &endpoint.peer_writer {
            Some(p) => p,
            None => {
                // end this connection if there is no peer pipe
                registry.deregister(&mut endpoint.stream)?;
                return Ok(true);
            }
        };

        let src_fd = endpoint.stream.as_raw_fd();
        let dst_fd = writer.as_raw_fd();

        let sent;
        unsafe {
            sent = libc::splice(src_fd, 0 as *mut libc::loff_t, dst_fd, 0 as *mut libc::loff_t, 2048, libc::SPLICE_F_NONBLOCK);  

        }

        // check if connection is closed
        if sent <= 0 {
            return Ok(true);
        }

        println!("wrote {} bytes to pipe", sent);


        // We are now interested in WRITEABLE events for our peer
        //registry.register(&mut peer.stream, *endpoint.peer_token.as_ref().unwrap(),Interest::WRITABLE)?;

        //libc::syscall(libc::SYS_copy_file_range, fd_in, off_in, fd_out, off_out, len, flags)

        // After we've written something we'll reregister the connection
        /* to only respond to readable events.
        registry.reregister(connection, event.token(), Interest::READABLE)? */
    }

    Ok(false)
}