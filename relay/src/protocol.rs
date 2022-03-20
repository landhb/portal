use mio::net::TcpStream;
use mio::Token;
use os_pipe::pipe;
use portal_lib::Portal;
use std::error::Error;
use std::io::Write;
use std::net::SocketAddr;
use std::os::unix::io::AsRawFd;
use std::time::SystemTime;

use crate::{networking, Endpoint, EndpointPair, MAX_SPLICE_SIZE, PENDING_ENDPOINTS};

const PLACEHOLDER: usize = 0;

/**
 * Attempt to parse a Portal request from the client and match it
 * with a peer. If matched, the pair will be added to an event loop
 */
pub fn register(
    addr: SocketAddr,
    mut connection: TcpStream,
    tx: mio_extras::channel::Sender<EndpointPair>,
) -> Result<(), Box<dyn Error>> {
    let mut received_data = Vec::with_capacity(1024);
    while received_data.is_empty() {
        match networking::recv_generic(&mut connection, &mut received_data) {
            Ok(v) if v < 0 => {
                break; // done recieving
            }
            Ok(_) => {}
            Err(_) => {
                break;
            }
        }
    }

    log::trace!("[?] Received {:?} bytes", received_data.len());

    // attempt to recieve a portal request
    let req: Portal = match Portal::parse(&received_data) {
        Ok(r) => r,
        Err(e) => {
            log::debug!("Failed to parse portal request: {:?}", e);
            return Err(e.into());
        }
    };

    // Lookup existing endpoint with this ID
    let id = req.get_id();
    let dir = req.get_direction();

    log::info!("[{:.6}] New Portal request: {:?}({:?})", id, dir, addr);

    // Clear old entries before accepting, will keep
    // connections < 15 min old
    let mut ref_endpoints = PENDING_ENDPOINTS.lock().unwrap();
    ref_endpoints.retain(|_, v| {
        v.has_peer
            || (v.time_added.elapsed().unwrap().as_secs()
                < std::time::Duration::from_secs(60 * 15).as_secs())
    });

    match dir {
        portal::Direction::Receiver => {
            //let mut ref_endpoints = endpoints.borrow_mut();
            let mut peer = match ref_endpoints.remove(&id.to_string()) {
                Some(p) => p,
                None => {
                    return Ok(());
                }
            };

            log::info!("[{:.6}] Receiver matched with Sender", id);

            // if the peer already has a connection, disregard this one
            if peer.has_peer {
                let _ = connection.shutdown(std::net::Shutdown::Both);
                log::info!("[{:.6}] Canceled receiving connection: Sender already has a different connection.", id);
                return Ok(());
            }

            // This pipe will be used to send data from Receiver->Sender
            // so the Sender will keep the read side, and the Receiver will
            // keep the write side
            let (reader2, mut writer2) = match pipe() {
                Ok((r, w)) => (r, w),
                Err(err) => {
                    log::error!(
                        "[{:.6}] Error creating pipe for peer communication. Reason: {}",
                        id,
                        err
                    );
                    return Err(Box::new(err));
                }
            };

            // write the acknowledgement response to both pipe endpoints
            let resp = req.serialize()?;
            writer2.write_all(&resp)?;

            log::debug!("[{:.6}] Acknowledgement sent to peer", id);

            // update the peer with the pipe information
            let old_reader = std::mem::replace(&mut peer.peer_reader, Some(reader2));
            peer.has_peer = true;

            // create this endpoint
            let endpoint = Endpoint {
                id: id.to_string(),
                dir,
                stream: connection,
                peer_reader: old_reader,
                peer_writer: Some(writer2), //None,
                has_peer: true,
                time_added: SystemTime::now(),
            };

            log::debug!("[{:.6}] Added Receiver", id);

            let pair = EndpointPair {
                sender: peer,
                sender_token: Token(PLACEHOLDER),
                receiver: endpoint,
                receiver_token: Token(PLACEHOLDER),
            };

            // Communicate the new pair over the MPSC channel
            // back to the main event loop
            tx.send(pair)?;
        }
        portal::Direction::Sender => {
            // Kill the connection if this ID is being used by another pending sender
            let search =
                ref_endpoints
                    .iter()
                    .find_map(|(key, val)| if *val.id == *id { Some(key) } else { None });

            if search.is_some() {
                return Ok(());
            }

            // This pipe will be used to send data from Sender->Receiver
            let (reader, mut writer) = pipe().unwrap();

            // resize the pipe that we will be using for the actual
            // file transfer
            unsafe {
                let res = libc::fcntl(reader.as_raw_fd(), libc::F_SETPIPE_SZ, MAX_SPLICE_SIZE);
                if res < 0 {
                    return Ok(());
                }
            }

            let resp = req.serialize()?;
            writer.write_all(&resp)?;

            let endpoint = Endpoint {
                id: id.to_string(),
                dir,
                stream: connection,
                peer_writer: Some(writer),
                peer_reader: Some(reader),
                has_peer: false,
                time_added: SystemTime::now(),
            };

            log::debug!("[{:.6}] Added Sender", id);

            ref_endpoints.entry(id.to_string()).or_insert(endpoint);
        }
    }
    Ok(())
}
