use crate::wordlist::gen_phrase;
use crate::{MULTI, PSTYLE};
use colored::*;
use indicatif::ProgressBar;
use portal::{errors::PortalError, Direction, Metadata, Portal, TransferInfo};
use std::{
    error::Error,
    net::TcpStream,
    path::{Path, PathBuf},
};

/// As the sender, a pass-phrase muse be created to deliver
/// out-of-band (in secret) to the receiver.
fn create_password() -> (String, String) {
    let id = gen_phrase(1);
    let pass = gen_phrase(3);
    log_success!(
        "Tell your peer their pass-phrase is: {:?}",
        format!("{}-{}", id, pass)
    );
    (id, pass)
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

    for file in files {
        info.add_file(file.as_path())?;
    }

    Ok(info)
}

/// Send a file
pub fn send_all(client: &mut TcpStream, files: Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
    // Parse the input files
    let info = validate_files(files)?;

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
