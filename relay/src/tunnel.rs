extern crate portal_lib as portal;
use crate::errors::RelayError;
use crate::ffi::splice;
use crate::Endpoint;
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
            // Done reading, close write end of pipe
            Ok(x) if x == 0 => {
                self.source_finished = true;
                unsafe {
                    libc::close(self.pipe_in);
                }
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
    pub fn drain_to_destination(&mut self) -> Result<ControlFlow<bool>, RelayError> {
        // Write from the pipe into the peer connection
        let tx = splice(self.pipe_out, self.destination);
        match tx {
            // Closed pipe
            Ok(x) if x == 0 => Ok(ControlFlow::Break(true)),
            // Would block
            Err(RelayError::WouldBlock) => Ok(ControlFlow::Break(false)),
            // Fatal errors
            Err(_e) => tx.map(|_| ControlFlow::Break(true)),
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
    pub fn transfer_until_blocked(&mut self) -> Result<bool, Box<dyn Error>> {
        loop {
            // Only read when data is available
            if !self.source_finished {
                self.buffer_source()?;
            }

            // Continue draining pipe
            match self.drain_to_destination() {
                Ok(ControlFlow::Break(x)) => return Ok(x),
                Ok(ControlFlow::Continue(())) => continue,
                Err(e) => return Err(e.into()),
            }
        }
    }
}
