use crate::wordlist::gen_phrase;
use crate::{MULTI, PSTYLE};
use colored::*;
use indicatif::ProgressBar;
use portal::{errors::PortalError, Direction, Portal, TransferInfo};
use std::{error::Error, net::TcpStream, path::PathBuf};

/// As the sender, a pass-phrase muse be created to deliver
/// out-of-band (in secret) to the receiver.
fn create_password() -> (String, String) {
    let (id, pass) = (gen_phrase(1), gen_phrase(3));
    log_success!(
        "Tell your peer their pass-phrase is: {:?}",
        format!("{}-{}", id, pass)
    );
    (id, pass)
}

// Helper method to enumerate directories depth 1
fn add_all(info: &mut TransferInfo, dir: PathBuf) -> Result<(), Box<dyn Error>> {
    // Collect all entries
    let entries = std::fs::read_dir(dir)?
        .filter_map(|res| {
            res.as_ref().map_or(None, |e| {
                if e.metadata().map_or(false, |f| f.is_file()) {
                    return Some(e.path());
                }
                None
            })
        })
        .collect::<Vec<PathBuf>>();

    // Add them individually
    for entry in entries {
        info.add_file(&entry)?;
    }

    Ok(())
}

/// Converts a list of input files into TransferInfo
pub fn validate_files(files: Vec<PathBuf>) -> Result<TransferInfo, Box<dyn Error>> {
    // Validate that there is at least one file to send
    if files.is_empty() {
        log_error!("Provide at least one file to send");
        return Err(PortalError::BadFileName.into());
    }

    // Begin adding files to this transfer
    let mut info = TransferInfo::empty();
    for item in files {
        match item.is_dir() {
            true => {
                add_all(&mut info, item)?;
            }
            false => {
                info.add_file(item.as_path())?;
            }
        }
    }

    Ok(info)
}

/// Send a file
pub fn send_all(client: &mut TcpStream, files: Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
    // Parse the input files
    let info = validate_files(files)?;

    log_status!("Outgoing files:");
    crate::display_info(&info);

    // Sender must generate the password
    let (id, pass) = create_password();

    // Initialize portal
    let mut portal = Portal::init(Direction::Sender, id, pass).map_err(|e| {
        log_error!("Failed to initialize portal");
        e
    })?;

    // Complete handshake
    portal.handshake(client).map_err(|e| {
        log_error!(
            "Failed to complete portal handshake.
            Verify client version & passphrase."
        );
        e
    })?;

    // TODO: Establish P2P QUIC connection here?

    log_status!("Starting transfer...");

    for (fullpath, metadata) in portal.outgoing(client, &info)? {
        // Start the progress bar
        let pb = MULTI.add(ProgressBar::new(metadata.filesize));
        pb.set_style(PSTYLE.clone());

        // Required to render
        pb.tick();

        // Set filename as the message
        pb.set_message(metadata.filename.clone());

        // User callback to display progress
        let progress = |transferred: usize| {
            pb.set_position(transferred as u64);
        };

        // Begin the transfer
        let _sent = portal.send_file(client, &fullpath, Some(progress)).ok();

        pb.finish();
    }

    Ok(())
}
