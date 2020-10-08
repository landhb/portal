extern crate portal_lib as portal;

use crate::Endpoint;
use anyhow::Result;
use std::os::unix::io::AsRawFd;
use crate::{log,MAX_SPLICE_SIZE};

/**
 *  Handles TCP splicing without utilizing a userpace intermediary buffer
 *
 *  When the src_fd is readable, we will attempt to splice data into the dst_fd,
 *  using an intermediary pipe
 */
pub fn tcp_splice (
    endpoint: &Endpoint,
    peer: &Endpoint) -> Result<bool> {

    let mut rx;
    let mut tx;


    let src_fd = endpoint.stream.as_raw_fd();
    let p_in = endpoint.peer_writer.as_ref().unwrap().as_raw_fd();

    let p_out = peer.peer_reader.as_ref().unwrap().as_raw_fd();
    let dst_fd = peer.stream.as_raw_fd();

    
    loop {

        unsafe {
            *libc::__errno_location() = 0;
            rx = libc::splice(src_fd, std::ptr::null_mut::<libc::loff_t>(), p_in, std::ptr::null_mut::<libc::loff_t>(), MAX_SPLICE_SIZE, libc::SPLICE_F_MOVE | libc::SPLICE_F_NONBLOCK);  
        }

        let errno = std::io::Error::last_os_error().raw_os_error().unwrap();
        log!("got {} bytes from {:?}, errno: {:?}", rx, endpoint.dir,errno);

        // check if connection is closed
        if rx < 0 && errno != 0 && errno != libc::EWOULDBLOCK && errno != libc::EAGAIN {
            return Ok(true);
        }

        /* We cannot break here on EWOULDBLOCK since the first splice may return EWOULDBLOCK
         * if the pipe is full, in that case we'd still want to complete the second splice 
         * to clear the pipe */

        // Done reading
        if rx == 0 {
            return Ok(true);
        } 

        unsafe {
            tx = libc::splice(p_out, std::ptr::null_mut::<libc::loff_t>(), dst_fd, std::ptr::null_mut::<libc::loff_t>(), MAX_SPLICE_SIZE,  libc::SPLICE_F_MOVE | libc::SPLICE_F_NONBLOCK);    
        }

        let errno = std::io::Error::last_os_error().raw_os_error().unwrap();
        log!("sent {} bytes to {:?}, errno: {:?}", tx, peer.dir , errno);

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
    endpoint: &Endpoint) -> Result<bool> {

    let reader = match &endpoint.peer_reader {
        Some(p) => p,
        None => {
            // end this connection if there is no peer pipe
            return Ok(true);
        }
    };


    let dst_fd = endpoint.stream.as_raw_fd();
    let src_fd = reader.as_raw_fd();

    let mut trx;

    unsafe { let errno = libc::__errno_location(); *errno = 0;}
    loop  { 
        unsafe {
            trx = libc::splice(src_fd, std::ptr::null_mut::<libc::loff_t>(), dst_fd, std::ptr::null_mut::<libc::loff_t>(), MAX_SPLICE_SIZE,libc::SPLICE_F_MOVE | libc::SPLICE_F_NONBLOCK);     
        }

        let errno = std::io::Error::last_os_error().raw_os_error().unwrap();

        // check for errors
        if trx < 0  && errno != 0 && errno != libc::EWOULDBLOCK && errno != libc::EAGAIN {
            println!("exiting due to trx: {:?} errno {:?}", trx, errno);
            return Ok(true);
        }

        log!("drained {} bytes to {:?}, errno: {:?}", trx, endpoint.dir, errno);

        // break if blocking
        if trx < 0 && (errno == libc::EWOULDBLOCK || errno == libc::EAGAIN) {
            break;
        }

        if trx == 0 {
            return Ok(true);
        } 

    }
        
    Ok(false)
}
