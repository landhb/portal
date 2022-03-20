extern crate portal_lib as portal;

use crate::Endpoint;
use crate::{log, MAX_SPLICE_SIZE};
use anyhow::Result;
use std::os::unix::io::AsRawFd;

/**
 *  Handles TCP splicing without utilizing a userpace intermediary buffer
 *
 *  When the src_fd is readable, we will attempt to splice data into the dst_fd,
 *  using an intermediary pipe
 */
pub fn tcp_splice(endpoint: &Endpoint, peer: &Endpoint) -> Result<bool> {
    let mut rx;
    let mut tx;

    let src_fd = endpoint.stream.as_raw_fd();
    let p_in = endpoint.peer_writer.as_ref().unwrap().as_raw_fd();

    let p_out = peer.peer_reader.as_ref().unwrap().as_raw_fd();
    let dst_fd = peer.stream.as_raw_fd();

    let id = endpoint.id.clone();

    loop {
        unsafe {
            *libc::__errno_location() = 0;
            rx = libc::splice(
                src_fd,
                std::ptr::null_mut::<libc::loff_t>(),
                p_in,
                std::ptr::null_mut::<libc::loff_t>(),
                MAX_SPLICE_SIZE,
                libc::SPLICE_F_MOVE | libc::SPLICE_F_NONBLOCK,
            );
        }

        let errno = std::io::Error::last_os_error().raw_os_error().unwrap();

        // check if connection is closed
        if rx < 0 && errno != 0 && errno != libc::EWOULDBLOCK && errno != libc::EAGAIN {
            log::error!(
                "[{:.6}] Error receiving data from {:?}: errno: {}",
                id,
                endpoint.dir,
                errno
            );
            return Ok(true);
        }

        log::debug!("[{:.6}] Received {} bytes from {:?}", id, rx, endpoint.dir);

        /* We cannot break here on EWOULDBLOCK since the first splice may return EWOULDBLOCK
         * if the pipe is full, in that case we'd still want to complete the second splice
         * to clear the pipe */

        // Done reading
        if rx == 0 {
            return Ok(true);
        }

        unsafe {
            tx = libc::splice(
                p_out,
                std::ptr::null_mut::<libc::loff_t>(),
                dst_fd,
                std::ptr::null_mut::<libc::loff_t>(),
                MAX_SPLICE_SIZE,
                libc::SPLICE_F_MOVE | libc::SPLICE_F_NONBLOCK,
            );
        }

        let errno = std::io::Error::last_os_error().raw_os_error().unwrap();

        // check for errors
        if tx < 0 && errno != 0 && errno != libc::EWOULDBLOCK && errno != libc::EAGAIN {
            log::error!(
                "[{:.6}] Exiting due to error splicing pipes from {:?}. trx: {:?} errno {:?}",
                id,
                endpoint.dir,
                tx,
                errno
            );
            return Ok(true);
        }

        // break if blocking
        if tx < 0 && (errno == libc::EWOULDBLOCK || errno == libc::EAGAIN) {
            break;
        }

        if tx == 0 {
            return Ok(true);
        }

        log::debug!("[{:.6}] Sent {} bytes to {:?}", id, tx, peer.dir);
    }

    Ok(false)
}

/**
 * Drain the pipe of any additional data destined for an Endpoint
 */
pub fn drain_pipe(endpoint: &Endpoint) -> Result<bool> {
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

    let id = endpoint.id.clone();

    unsafe {
        let errno = libc::__errno_location();
        *errno = 0;
    }
    loop {
        unsafe {
            trx = libc::splice(
                src_fd,
                std::ptr::null_mut::<libc::loff_t>(),
                dst_fd,
                std::ptr::null_mut::<libc::loff_t>(),
                MAX_SPLICE_SIZE,
                libc::SPLICE_F_MOVE | libc::SPLICE_F_NONBLOCK,
            );
        }

        let errno = std::io::Error::last_os_error().raw_os_error().unwrap();

        // check for errors
        if trx < 0 && errno != 0 && errno != libc::EWOULDBLOCK && errno != libc::EAGAIN {
            log::error!(
                "[{:.6}] Exiting due to error draining pipe to {:?}. trx: {:?} errno {:?}",
                id,
                endpoint.dir,
                trx,
                errno
            );
            return Ok(true);
        }

        // break if blocking
        if trx < 0 && (errno == libc::EWOULDBLOCK || errno == libc::EAGAIN) {
            break;
        }

        if trx == 0 {
            return Ok(true);
        }

        log::debug!(
            "[{:.6}] Drained {} bytes to {:?}, errno: {:?}",
            id,
            trx,
            endpoint.dir,
            errno
        );
    }

    Ok(false)
}
