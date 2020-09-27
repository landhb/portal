extern crate portal_lib as portal;

use mio::{Poll,Token, Ready, PollOpt};
use crate::Endpoint;
use mio::event::Event;
use anyhow::Result;
use std::os::unix::io::AsRawFd;

/**
 *  Handles events without utilizing a userpace intermediary buffer
 *  Utilizing splice(2)
 *
 *  READABLE: Transfer data from Sender socket -> pipe
 *  WRITEABLE: Transfer data from pipe -> Reciever socket
 *
 *  The data will be transfered from:
 *   
 *  Sender socket -> Pipe -> Reciever Socket
 */
pub fn handle_client_event (
    token: Token,
    registry: &Poll,
    endpoint: &mut Endpoint,
    event: &Event) -> Result<bool> {


    // Writeable events will mean data is ready to be forwarded to the Reciever
    if event.readiness().is_writable() {

        let reader = match &endpoint.peer_reader {
            Some(p) => p,
            None => {
                // end this connection if there is no peer pipe
                //registry.deregister(&mut endpoint.stream)?;
                return Ok(true);
            }
        };


        let dst_fd = endpoint.stream.as_raw_fd();
        let src_fd = reader.as_raw_fd();

        let read;
        unsafe {
            read = libc::splice(src_fd, 0 as *mut libc::loff_t, dst_fd, 0 as *mut libc::loff_t, 65535, libc::SPLICE_F_NONBLOCK);    
        }


        // check if connection is closed
        let errno = std::io::Error::last_os_error().raw_os_error();
        if read <= 0 && (errno != Some(libc::EWOULDBLOCK) || errno != Some(libc::EAGAIN)) {
            registry.deregister(&mut endpoint.stream)?;
            return Ok(true);
        }

        if read <=0 && (errno == Some(libc::EWOULDBLOCK) || errno == Some(libc::EAGAIN)) {
            registry.reregister(&mut endpoint.stream,token,Ready::readable(),PollOpt::level())?;
        }

        println!("read {} bytes from pipe", read);
        
    }

    
    // Readable events will be file data from the Sender
    if event.readiness().is_readable() {

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
            sent = libc::splice(src_fd, 0 as *mut libc::loff_t, dst_fd, 0 as *mut libc::loff_t, 65535, libc::SPLICE_F_NONBLOCK);  
        }

        // check if connection is closed
        let errno = std::io::Error::last_os_error().raw_os_error();
        if sent <= 0 && (errno != Some(libc::EWOULDBLOCK) || errno != Some(libc::EAGAIN)) {
            registry.deregister(&mut endpoint.stream)?;
            return Ok(true);
        }

        println!("wrote {} bytes to pipe", sent);

    }

    /* Check for closed connections before returning
    if event.is_error() || event.is_read_closed() || event.is_write_closed() {
        registry.deregister(&mut endpoint.stream)?;
        return Ok(true);
    } */

    Ok(false)
}