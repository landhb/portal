use crate::wordlist::gen_phrase;
use crate::{MULTI, PSTYLE};
use colored::*;
use indicatif::ProgressBar;
use portal::{Direction, Metadata, Portal, TransferInfo};
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

/// Send a file
pub fn send_all(client: &mut TcpStream, files: Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
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

    // RMEOVE
    let fpath = files.first().unwrap();

    // Obtain file size for the progress bar
    let metadata = std::fs::metadata(fpath)?;

    // TODO make this a builder pattern
    // with .add_file() and add_directory()
    let info = TransferInfo {
        all: vec![Metadata {
            filesize: metadata.len(),
            filename: Path::new(fpath)
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
        }],
    };

    for file in portal.outgoing(client, info)? {
        // Start the progress bar
        let pb = MULTI.add(ProgressBar::new(file.filesize));
        pb.set_style(PSTYLE.clone());

        // Required to render
        pb.tick();

        // Set filename as the message
        pb.set_message(file.filename.clone());

        // User callback to display progress
        let progress = |transferred: usize| {
            pb.set_position(transferred as u64);
        };

        // Begin the transfer
        let _sent = portal
            .send_file(client, fpath.to_str().unwrap(), Some(progress))
            .ok();

        pb.finish();
    }

    Ok(())
}
