extern crate portal_lib as portal;

use crate::{Endpoint,EndpointPair};
use anyhow::Result;
use std::os::unix::io::AsRawFd;
use crate::{log,MAX_SPLICE_SIZE};
use portal::Direction;

/**
 *  Handles TCP splicing without utilizing a userpace intermediary buffer
 *
 *  READABLE: Transfer data from Sender socket -> pipe
 *  WRITEABLE: Transfer data from pipe -> Reciever socket
 *
 *  The data will be transfered from:
 *   
 *  Sender socket -> Pipe -> Reciever Socket
 */
pub fn tcp_splice (
    direction: Direction,
    pair: &mut EndpointPair) -> Result<bool> {

    let mut rx;
    let mut tx;
    
    // Depending on which peer is readable, 
    // use the appropriate pipe and source/dst FDs
    let (src_fd, p_in, p_out, dst_fd,_peer) = match direction {
        Direction::Sender => {
            let src_fd = pair.sender.stream.as_raw_fd();
            let pipe_writer = pair.sender.peer_writer.as_ref().unwrap().as_raw_fd();
            let pipe_reader = pair.receiver.peer_reader.as_ref().unwrap().as_raw_fd();
            let dst_fd = pair.receiver.stream.as_raw_fd();
            (src_fd,pipe_writer,pipe_reader,dst_fd,Direction::Receiver)
        }
        Direction::Receiver => {
            let src_fd = pair.receiver.stream.as_raw_fd();
            let pipe_writer = pair.receiver.peer_writer.as_ref().unwrap().as_raw_fd();
            let pipe_reader = pair.sender.peer_reader.as_ref().unwrap().as_raw_fd();
            let dst_fd = pair.sender.stream.as_raw_fd();
            (src_fd,pipe_writer,pipe_reader,dst_fd,Direction::Sender)
        }
    };

    
    loop {

        unsafe {
            *libc::__errno_location() = 0;
            rx = libc::splice(src_fd, 0 as *mut libc::loff_t, p_in, 0 as *mut libc::loff_t, MAX_SPLICE_SIZE, libc::SPLICE_F_MOVE | libc::SPLICE_F_NONBLOCK);  
        }

        let errno = std::io::Error::last_os_error().raw_os_error().unwrap();
        log!("got {} bytes from {:?}, errno: {:?}", rx, direction,errno);

        // check if connection is closed
        if rx < 0 && errno != 0 && errno != libc::EWOULDBLOCK && errno != libc::EAGAIN {
            return Ok(true);
        }

        // We cannot break here on EWOULDBLOCK since the first splice may return EWOULDBLOCK
        // if the pipe is full, in that case we'd want to complete the second splice to clear
        // the pipe

        // Done reading
        if rx == 0 {
            return Ok(true);
        } 

        unsafe {
            tx = libc::splice(p_out, 0 as *mut libc::loff_t, dst_fd, 0 as *mut libc::loff_t, MAX_SPLICE_SIZE,  libc::SPLICE_F_MOVE | libc::SPLICE_F_NONBLOCK);    
        }

        let errno = std::io::Error::last_os_error().raw_os_error().unwrap();
        log!("sent {} bytes to {:?}, errno: {:?}", tx, _peer, errno);

        // check for errors
        if tx < 0  && errno != 0 && errno != libc::EWOULDBLOCK && errno != libc::EAGAIN {
            println!("exiting due to trx: {:?} errno {:?}", tx, errno);
            return Ok(true);
        }

        // break if blocking
        if tx < 0 && (errno == libc::EWOULDBLOCK || errno == libc::EAGAIN) {
            break;
        }

        if tx == 0 {
            return Ok(true);
        } 

    }

    Ok(false)
}

/**
 * Drain the pipe of any additional data destined for an Endpoint
 */
pub fn drain_pipe(
    endpoint: &Endpoint) -> Result<(bool,isize)> {

    let reader = match &endpoint.peer_reader {
        Some(p) => p,
        None => {
            // end this connection if there is no peer pipe
            return Ok((true,0));
        }
    };


    let dst_fd = endpoint.stream.as_raw_fd();
    let src_fd = reader.as_raw_fd();

    let mut trx;

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
            return Ok((true,0));
        } 

    }
        
    Ok((false,trx))
}
