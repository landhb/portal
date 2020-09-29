extern crate portal_lib as portal;

use mio::{Poll,Token, Ready, PollOpt};
use crate::Endpoint;
use mio::event::Event;
use anyhow::Result;
use std::os::unix::io::AsRawFd;

//use crate::logging::*;

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
        while let Some(errno) = std::io::Error::last_os_error().raw_os_error() {

            unsafe {
                trx = libc::splice(src_fd, 0 as *mut libc::loff_t, dst_fd, 0 as *mut libc::loff_t, 65535, libc::SPLICE_F_NONBLOCK);    
            }

            if trx < 0 && errno != 0 && errno != libc::EWOULDBLOCK && errno != libc::EAGAIN {
                println!("exiting due to trx: {:?} errno {:?}", trx, errno);
                return Ok((true,trx));
            }

            if trx < 0 && (errno == libc::EWOULDBLOCK || errno == libc::EAGAIN) {
                break;
            }

            log!("sent {} bytes to {:?}", trx, endpoint.dir);


            if trx == 0 {
                break;
            }

        }

        // If this is the Sender we wrote to, then we've just completed
        // msg exchange and are now only interested in READABLE events from
        // the sender
        if endpoint.dir == portal::Direction::Sender && endpoint.writable == false {
            registry.reregister(&mut endpoint.stream,token,Ready::readable(),PollOpt::level())?;
        }
        
    }

    
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

        unsafe {
            let errno = libc::__errno_location(); 
            *errno = 0;
            trx = libc::splice(src_fd, 0 as *mut libc::loff_t, dst_fd, 0 as *mut libc::loff_t, 65535, libc::SPLICE_F_NONBLOCK);  
        }

        // check if connection is closed
        let errno = std::io::Error::last_os_error().raw_os_error();
        if trx < 0 && errno != Some(libc::EWOULDBLOCK) && errno != Some(libc::EAGAIN) {
            return Ok((true,trx));
        }

        if trx == 0 {
            return Ok((true,trx));
        }

        //log!("wrote {} bytes to pipe", trx);
        log!("got {} bytes from {:?}", trx, endpoint.dir);

        // If this is the Reciever, the we've received the last message
        // to be read, we're now only interested in WRITABLE events,
        // we'll use edge triggering for this endpoint since we'll want to fully
        // drain the pipe when a writable event occurs
        if endpoint.dir == portal::Direction::Receiver {
            registry.reregister(&mut endpoint.stream,token,Ready::writable(),PollOpt::level())?;
        }

    }

    Ok((false,trx))
}
