extern crate portal_lib as portal;

use mio::{Poll,Token, Ready, PollOpt};
use crate::Endpoint;
use mio::event::Event;
use anyhow::Result;
use std::os::unix::io::AsRawFd;
use crate::log;

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
    event: &Event) -> Result<(bool,isize)> {

    let mut trx = 0;
    
    // Readable events will be file data from the Sender
    if event.readiness().is_readable() {

        let writer = match &endpoint.peer_writer {
            Some(p) => p,
            None => {
                // end this connection if there is no peer pipe
                return Ok((true,0));
            }
        };

        let src_fd = endpoint.stream.as_raw_fd();
        let dst_fd = writer.as_raw_fd();

        unsafe { let errno = libc::__errno_location(); *errno = 0;}
        loop {
            unsafe {
                trx = libc::splice(src_fd, 0 as *mut libc::loff_t, dst_fd, 0 as *mut libc::loff_t, 65535, libc::SPLICE_F_NONBLOCK);  
            }

            // check if connection is closed
            let errno = std::io::Error::last_os_error().raw_os_error();
            if trx < 0 && errno != Some(0) && errno != Some(libc::EWOULDBLOCK) && errno != Some(libc::EAGAIN) {
                return Ok((true,trx));
            }

            log!("got {} bytes from {:?}, errno: {:?}", trx, endpoint.dir,errno);

            // break if blocking
            if trx < 0 && (errno == Some(libc::EWOULDBLOCK) || errno == Some(libc::EAGAIN)) {
                break;
            }

            // Done sending
            if trx == 0 {
                return Ok((true,trx));
            } 
        }
    }

    // Writeable events will mean data is ready to be forwarded to the Reciever
    if event.readiness().is_writable() {

        let reader = match &endpoint.peer_reader {
            Some(p) => p,
            None => {
                // end this connection if there is no peer pipe
                return Ok((true,0));
            }
        };


        let dst_fd = endpoint.stream.as_raw_fd();
        let src_fd = reader.as_raw_fd();

        unsafe { let errno = libc::__errno_location(); *errno = 0;}
        loop  { 
            unsafe {
                trx = libc::splice(src_fd, 0 as *mut libc::loff_t, dst_fd, 0 as *mut libc::loff_t, 65535, libc::SPLICE_F_NONBLOCK);    
            }

            let errno = std::io::Error::last_os_error().raw_os_error().unwrap();

            // check for errors
            if trx < 0  && errno != 0 && errno != libc::EWOULDBLOCK && errno != libc::EAGAIN {
                println!("exiting due to trx: {:?} errno {:?}", trx, errno);
                return Ok((true,trx));
            }

            log!("sent {} bytes to {:?}, errno: {:?}", trx, endpoint.dir, errno);

            // break if blocking
            if trx < 0 && (errno == libc::EWOULDBLOCK || errno == libc::EAGAIN) {
                break;
            }

            if trx == 0 {
                break;
            } 

        }
        
    }


    Ok((false,trx))
}
