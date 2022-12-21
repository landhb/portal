use mio::net::TcpStream;
use mio::Token;
use mio_extras::channel::Sender;
use os_pipe::{pipe, PipeReader, PipeWriter};
use portal_lib::errors::PortalError;
use portal_lib::protocol::{ConnectMessage, Direction, PortalMessage};
use std::collections::HashMap;
use std::error::Error;
use std::io::Write;
use std::net::SocketAddr;
use std::os::unix::io::AsRawFd;
use std::sync::MutexGuard;
use std::time::SystemTime;

use crate::{networking, Endpoint, EndpointPair, MAX_SPLICE_SIZE, PENDING_ENDPOINTS};

/// The recevier will have a placeholder token before
/// receiving
const PLACEHOLDER: usize = 0;

pub struct ConnectionBroker {
    /// Active remote address
    addr: SocketAddr,
    /// Active incoming stream
    connection: TcpStream,
    /// Channel for completed endpoints
    tx: Sender<EndpointPair>,
}

impl ConnectionBroker {
    /// Initiate a new broker to connect two endpoints.
    pub fn new(addr: SocketAddr, connection: TcpStream, tx: Sender<EndpointPair>) -> Self {
        Self {
            addr,
            connection,
            tx,
        }
    }

    /// Accept a new receiver by attempting to match it with an existing Sender.
    ///
    /// The Receiver should be 2nd to connect, after entering the appropriate password
    /// provided by the Sender. So if there is no matching peer this ends the connection.
    pub fn add_receiver(
        self,
        req: ConnectMessage,
        reader: PipeReader,
        writer: PipeWriter,
        mut endpoints: MutexGuard<'_, HashMap<String, Endpoint>>,
    ) -> Result<(), Box<dyn Error>> {
        // Remove any endpoint with a matching ID
        let mut peer = endpoints
            .remove(&req.id)
            .ok_or_else(|| Into::<Box<dyn Error>>::into("No peer."))?;
        log::info!("[{:.6}] Receiver matched with Sender", req.id);

        // If the peer already has a connection, disregard this one
        if peer.has_peer {
            let _ = self.connection.shutdown(std::net::Shutdown::Both);
            log::info!(
                "[{:.6}] Canceled receiving connection: Sender already has a different connection.",
                req.id
            );
            return Ok(());
        }

        log::debug!("[{:.6}] Acknowledgement sent to peer", req.id);

        // Update the peer with the pipe information
        let old_reader = std::mem::replace(&mut peer.peer_reader, Some(reader));
        peer.has_peer = true;

        // create this endpoint
        let endpoint = Endpoint {
            id: req.id.clone(),
            dir: Direction::Receiver,
            stream: self.connection,
            peer_reader: old_reader,
            peer_writer: Some(writer), //None,
            has_peer: true,
            time_added: SystemTime::now(),
        };

        log::debug!("[{:.6}] Added Receiver", req.id);

        let pair = EndpointPair {
            sender: peer,
            sender_token: Token(PLACEHOLDER),
            receiver: endpoint,
            receiver_token: Token(PLACEHOLDER),
        };

        // Communicate the new pair over the MPSC channel
        // back to the main event loop
        self.tx.send(pair)?;
        Ok(())
    }

    /// Accept a new sender by adding it to a pending queue of non-paired endpoints
    ///
    /// The Sender should be 1st to connect, and will remain pending until the timeout
    /// or until a valid Receiver connects to the relay.
    pub fn add_sender(
        self,
        req: ConnectMessage,
        reader: PipeReader,
        writer: PipeWriter,
        mut endpoints: MutexGuard<'_, HashMap<String, Endpoint>>,
    ) -> Result<(), Box<dyn Error>> {
        // Verify that no other connection is using this ID
        let search = endpoints
            .iter()
            .find_map(|(key, val)| if *val.id == *req.id { Some(key) } else { None });

        // Kill the connection if this ID is being used by another pending sender
        if search.is_some() {
            return Ok(());
        }

        // resize the pipe that we will be using for the actual
        // file transfer
        unsafe {
            let res = libc::fcntl(reader.as_raw_fd(), libc::F_SETPIPE_SZ, MAX_SPLICE_SIZE);
            if res < 0 {
                return Ok(());
            }
        }

        let endpoint = Endpoint {
            id: req.id.clone(),
            dir: Direction::Sender,
            stream: self.connection,
            peer_writer: Some(writer),
            peer_reader: Some(reader),
            has_peer: false,
            time_added: SystemTime::now(),
        };

        log::debug!("[{:.6}] Added Sender", req.id);

        endpoints.entry(req.id).or_insert(endpoint);
        Ok(())
    }

    /// Establishement requires receiving a ConnectMessage from the new endpoint.
    ///
    /// Then either entering a pending state if the endpoint is a new sender, or
    /// attempting to match endpoints if the new endpoint is a receiver.
    pub fn establish(mut self) -> Result<(), Box<dyn Error>> {
        let mut received_data = Vec::with_capacity(1024);
        while received_data.is_empty() {
            match networking::recv_generic(&mut self.connection, &mut received_data) {
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
        let req: ConnectMessage = match PortalMessage::parse(&received_data)? {
            PortalMessage::Connect(r) => r,
            x => {
                log::debug!("Got incorrect PortalMessage: {:?}", x);
                return Err(PortalError::BadMsg.into());
            }
        };

        log::info!(
            "[{:.6}] New Portal request: {:?}({:?})",
            req.id,
            req.direction,
            self.addr
        );

        // This pipe will be used to send data from Receiver->Sender or Sender->Receiver
        // depending on direction. Each endpoint will create a pipe for uni-directional communication.
        let (reader, mut writer) = pipe()?;

        // Write the sender data or receiver acknowledgement to the write side
        writer.write_all(&received_data)?;

        // Clear old entries before accepting, will keep
        // connections < 15 min old
        let mut ref_endpoints = PENDING_ENDPOINTS.lock()?;
        ref_endpoints.retain(|_, v| {
            v.has_peer
                || (v.time_added.elapsed().unwrap().as_secs()
                    < std::time::Duration::from_secs(60 * 15).as_secs())
        });

        // Side specific behavior
        match req.direction {
            Direction::Receiver => self.add_receiver(req, reader, writer, ref_endpoints)?,
            Direction::Sender => self.add_sender(req, reader, writer, ref_endpoints)?,
        }
        Ok(())
    }
}
