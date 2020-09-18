extern crate portal_lib as portal;


use mio::Registry;
use crate::Endpoint;
use mio::event::Event;
use anyhow::Result;
use std::os::unix::io::AsRawFd;

pub fn handle_client_event (
    registry: &Registry,
    endpoint: &mut Endpoint,
    event: &Event) -> Result<bool> {


    if event.is_writable() {
        // We can (likely) write to the socket without blocking.

        

        
    }

    if event.is_readable() {
        // We can (likely) read from the socket without blocking.

        let writer = match &endpoint.peer_writer {
            Some(p) => p,
            None => {
                // end this connection if there is no peer pipe
                return Ok(true);
            }
        };

        let src_fd = endpoint.stream.as_raw_fd();
        let dst_fd = writer.as_raw_fd();

        let sent;
        unsafe {
            sent = libc::splice(src_fd, 0 as *mut libc::loff_t, dst_fd, 0 as *mut libc::loff_t, 2048, 0);    
        }

        println!("wrote {} bytes to pipe", sent);

        //libc::syscall(libc::SYS_copy_file_range, fd_in, off_in, fd_out, off_out, len, flags)

        // After we've written something we'll reregister the connection
        /* to only respond to readable events.
        registry.reregister(connection, event.token(), Interest::READABLE)? */
    }

    Ok(false)
}