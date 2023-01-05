extern crate portal_lib as portal;
use crate::errors::RelayError;
use crate::ffi::splice;
use crate::Endpoint;
use crate::MAX_SPLICE_SIZE;
use std::error::Error;
use std::ops::ControlFlow;
use std::os::unix::io::AsRawFd;
use std::os::unix::io::RawFd;

/// Abstraction around a TCP splice using an intermediary kernel pipe.
///
/// Data will be buffered on the pipe from the source and written to the
/// destination when able.
///
/// The tunnel holds references to the underlying endpoints to ensure the
/// tunnel may only live as long as the provided references on initialization.
#[allow(dead_code)]
pub struct Tunnel<'a> {
    /// Source file descriptor
    source: RawFd,
    /// Input end of the intermediary pipe
    pipe_in: RawFd,
    /// Output end of the intermediary pipe
    pipe_out: RawFd,
    /// Destination file descriptor
    destination: RawFd,
    /// Boolean to track when the source is finished.
    source_finished: bool,
    /// Reference to Source Endpoint
    sref: &'a Endpoint,
    /// Reference to Destination Endpoint
    dref: &'a Endpoint,
}

impl<'a> Tunnel<'a> {
    /// Initialize a tunnel from two endpoints.
    pub fn new(source: &'a Endpoint, destination: &'a Endpoint) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            source: source.stream.as_raw_fd(),
            pipe_in: source
                .peer_writer
                .as_ref()
                .ok_or(RelayError::MissingFileDescriptor)?
                .as_raw_fd(),
            pipe_out: destination
                .peer_reader
                .as_ref()
                .ok_or(RelayError::MissingFileDescriptor)?
                .as_raw_fd(),
            destination: destination.stream.as_raw_fd(),
            source_finished: false,
            sref: source,
            dref: destination,
        })
    }

    /// Buffer data from the source into the pipe.
    ///
    /// Only errors on fatal errors. Wouldblock results in an Ok() condition
    /// since it is possible for the pipe to be full while we are still able
    /// to drain data from the pipe into the destination.
    fn buffer_source(&mut self) -> Result<usize, RelayError> {
        // Read into the pipe
        let rx = splice(self.source, self.pipe_in);
        match rx {
            // Done reading
            Ok(x) if x == 0 => {
                self.source_finished = true;
                Ok(0)
            }
            // Would block
            Err(RelayError::WouldBlock) => Ok(0),
            // All other errors or Ok() results propogate
            _ => rx,
        }
    }

    /// Drain data from the pipe into the destination.
    ///
    /// Publicly available so that it may be called standalone when the endpoint
    /// is writable but not readable.
    pub fn drain_to_destination(&mut self) -> Result<ControlFlow<()>, RelayError> {
        // Write from the pipe into the peer connection
        let tx = splice(self.pipe_out, self.destination);
        match tx {
            // Closed pipe
            Ok(x) if x == 0 => Ok(ControlFlow::Break(())),
            // Would block
            Err(RelayError::WouldBlock) => Ok(ControlFlow::Break(())),
            // Fatal errors
            Err(_e) => tx.map(|_| ControlFlow::Break(())),
            // Continue writing
            _ => Ok(ControlFlow::Continue(())),
        }
    }

    /// Helper method to transfer as much data as possible, until either the source
    /// or destination is in a blocking state.
    ///
    /// When the source is blocking, but there is data still available in the pipe. This
    /// method will continue draining the internal buffer until there is no more data
    /// available to send.
    pub fn transfer_until_blocked(&mut self) -> Result<ControlFlow<()>, Box<dyn Error>> {
        loop {
            // Only read when data is available
            if !self.source_finished {
                self.buffer_source()?;
            }

            // Continue draining pipe
            match self.drain_to_destination() {
                Ok(ControlFlow::Break(())) => break,
                Ok(ControlFlow::Continue(())) => continue,
                Err(e) => return Err(e.into()),
            }
        }
        // Continue polling until next event
        Ok(ControlFlow::Continue(()))
    }
}

/**
 *  Handles TCP splicing without utilizing a userpace intermediary buffer
 *
 *  When the src_fd is readable, we will attempt to splice data into the dst_fd,
 *  using an intermediary pipe
 */
pub fn tcp_splice(endpoint: &Endpoint, peer: &Endpoint) -> Result<bool, Box<dyn Error>> {
    let mut rx;
    let mut tx;

    // Pipe from tcp -> p_in
    let src_fd = endpoint.stream.as_raw_fd();
    let p_in = endpoint.peer_writer.as_ref().unwrap().as_raw_fd();

    // Pipe from p_out -> tcp
    let p_out = peer.peer_reader.as_ref().unwrap().as_raw_fd();
    let dst_fd = peer.stream.as_raw_fd();

    // Connection ID
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
pub fn drain_pipe(endpoint: &Endpoint) -> Result<bool, Box<dyn Error>> {
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
