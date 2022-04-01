use crate::{MULTI, PSTYLE};
use colored::*;
use indicatif::ProgressBar;
use portal::{Direction, Portal, TransferInfo};
use std::{
    error::Error,
    net::TcpStream,
    path::{Path, PathBuf},
};

/// The receiver must prompt the user for the pass-phrase
/// Splits the input and returns a tuple (id, password)
fn prompt_password() -> Result<(String, String), Box<dyn Error>> {
    let input = rpassword::prompt_password("Enter pass-phrase: ")?;
    let mut input = input.split('-');
    let id = input.next().unwrap().to_string();
    let opass = input.collect::<Vec<&str>>().join("-");
    Ok((id, opass))
}

/// Recv a file
pub fn recv_all(client: &mut TcpStream, download_directory: PathBuf) -> Result<(), Box<dyn Error>> {
    // Receiver must enter the password
    let (id, pass) = prompt_password()?;

    // Initialize portal
    let mut portal = Portal::init(Direction::Receiver, id, pass).map_err(|e| {
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

    log_success!("Completed portal handshake with peer.");

    // User callback to confirm/deny a transfer
    let confirm_download = |info: &TransferInfo| -> bool {
        for file in info.all.iter() {
            log_status!(
                "Incoming file: {:?}, size: {:?}",
                file.filename,
                file.filesize
            );
        }
        let ans = rpassword::prompt_password("Download the file(s)? [y/N]: ").unwrap();
        true
    };

    log_status!("Waiting for peer to begin transfer...");

    // For each file create a new progress bar
    for file in portal.incoming(client, Some(confirm_download))? {
        // Create a new bar
        let pb = MULTI.add(ProgressBar::new(file.filesize));
        pb.set_style(PSTYLE.clone());

        // Set filename as the message
        pb.set_message(file.filename.clone());

        // User callback to display progress
        let progress = |transferred: usize| {
            pb.set_position(transferred as u64);
        };

        // Required to render
        pb.tick();

        let _metadata = portal
            .recv_file(
                client,
                Path::new(&download_directory),
                &file,
                Some(progress),
            )
            .ok();

        pb.finish();
    }

    Ok(())
}
