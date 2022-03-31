extern crate portal_lib as portal;

use anyhow::Result;
use clap::{App, AppSettings, Arg, SubCommand};
use colored::*;
use dns_lookup::lookup_host;
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use portal::{errors::PortalError, protocol::Metadata, Direction, Portal, TransferInfo};
use std::error::Error;
use std::net::TcpStream;
use std::path::Path;
use std::sync::Arc;

#[macro_use]
extern crate lazy_static;

#[macro_use]
mod macros;
mod config;
use config::AppConfig;

mod wordlist;
use wordlist::gen_phrase;

lazy_static! {
    /// Global multi-bar that contains other progress bars
    pub static ref PROGRESS_BAR: Arc<MultiProgress> = Arc::new(
        MultiProgress::with_draw_target(ProgressDrawTarget::stdout())
    );

    /// All bars have the same style
    pub static ref PSTYLE: ProgressStyle = ProgressStyle::default_bar()
        .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta}) {msg}")
        .progress_chars("#>-");
}

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

/// The receiver must prompt the user for the pass-phrase
/// Splits the input and returns a tuple (id, password)
fn prompt_password() -> Result<(String, String), Box<dyn Error>> {
    let input = rpassword::prompt_password("Enter pass-phrase: ")?;
    let mut input = input.split('-');
    let id = input.next().unwrap().to_string();
    let opass = input.collect::<Vec<&str>>().join("-");
    Ok((id, opass))
}

/// Send a file
fn send_file(
    portal: &mut Portal,
    client: &mut TcpStream,
    fpath: &str,
) -> Result<(), Box<dyn Error>> {
    log_status!("Starting transfer...");

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
        let pb = PROGRESS_BAR.add(ProgressBar::new(file.filesize));
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
        let sent = match portal.send_file(client, fpath, Some(progress)) {
            Ok(total) => total,
            Err(e) => {
                log_error!("Failed to send file.");
                return Err(e);
            }
        };

        pb.finish();
    }

    Ok(())
}

/// Recv a file
fn recv_all(
    portal: &mut Portal,
    client: &mut TcpStream,
    download_directory: &str,
) -> Result<(), Box<dyn Error>> {
    log_status!("Waiting for peer to begin transfer...");

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

    // For each file create a new progress bar
    for file in portal.incoming(client, Some(confirm_download))? {
        // Create a new bar
        let pb = PROGRESS_BAR.add(ProgressBar::new(file.filesize));
        pb.set_style(PSTYLE.clone());

        // Required to render
        pb.tick();

        // Set filename as the message
        pb.set_message(file.filename.clone());

        // User callback to display progress
        let progress = |transferred: usize| {
            pb.set_position(transferred as u64);
        };

        let metadata =
            match portal.recv_file(client, Path::new(download_directory), &file, Some(progress)) {
                Ok(result) => result,
                Err(e) => {
                    log_error!("Failed to recv file.");
                    return Err(e);
                }
            };

        pb.finish();
    }

    Ok(())
}

/// Transfer
fn transfer(mut portal: Portal, fpath: &str, mut client: TcpStream) -> Result<(), Box<dyn Error>> {
    let msg = match portal.get_direction() {
        Direction::Receiver => {
            recv_all(&mut portal, &mut client, fpath)?;
        }
        Direction::Sender => {
            send_file(&mut portal, &mut client, fpath)?;
        }
    };

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let matches = App::new("portal")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("Quick & Safe File Transfers")
        .setting(AppSettings::ArgRequiredElseHelp)
        .subcommand(
            SubCommand::with_name("send").about("Send a file").arg(
                Arg::with_name("filename")
                    .short("f")
                    .takes_value(true)
                    .required(true)
                    .index(1)
                    .help("file to transfer"),
            ),
        )
        .subcommand(
            SubCommand::with_name("recv").about("Recieve a file").arg(
                Arg::with_name("download_folder")
                    .short("d")
                    .takes_value(true)
                    .required(false)
                    .help("override download folder"),
            ),
        )
        .get_matches();

    // Fix terminal output on windows
    #[cfg(target_os = "windows")]
    control::set_virtual_terminal(true).unwrap();

    // Load/create config location
    let mut cfg: AppConfig = confy::load("portal")?;
    log_status!(
        "Using portal.toml config, relay: {}!",
        cfg.relay_host.yellow()
    );

    // Determin the IP address to connect to
    let addr: std::net::IpAddr = match cfg.relay_host.parse() {
        Ok(res) => res,
        Err(_) => *lookup_host(&cfg.relay_host)?
            .first()
            .ok_or(PortalError::NoPeer)?,
    };

    let addr: std::net::SocketAddr = format!("{}:{}", addr, cfg.relay_port).parse()?;
    let mut client = match TcpStream::connect_timeout(&addr, std::time::Duration::new(6, 0)) {
        Ok(res) => res,
        Err(e) => {
            log_error!("Failed to connect");
            return Err(e.into());
        }
    };
    log_success!("Connected to {:?}!", addr);

    let (direction, id, pass, path, args) = match matches.subcommand() {
        ("send", Some(args)) => {
            let (id, pass) = create_password();
            let file = args.value_of("filename").unwrap();
            (Direction::Sender, id, pass, file.to_string(), args)
        }
        ("recv", Some(args)) => {
            let (id, pass) = prompt_password()?;

            // check if we need to override the download location
            if args.is_present("download_folder") {
                cfg.download_location = args.value_of("download_folder").unwrap().to_string();
            }

            (Direction::Receiver, id, pass, cfg.download_location, args)
        }
        _ => {
            log_error!("Please provide a valid subcommand. Run portal -h for more information.");
            std::process::exit(0);
        }
    };

    // Initialize portal
    let mut portal = match Portal::init(direction, id, pass) {
        Ok(res) => res,
        Err(e) => {
            log_error!("Failed to initialize portal");
            return Err(e.into());
        }
    };

    // Complete handshake
    match portal.handshake(&mut client) {
        Ok(_) => log_success!("Completed portal handshake with peer."),
        x => {
            log_error!("Failed to complete portal handshake. Verify client version & passphrase.");
            return x;
        }
    }

    // Begin transfer
    match transfer(portal, &path, client) {
        Ok(_) => log_success!("Complete!"),
        Err(e) => log_error!("{:?}", e),
    }

    // Must be called to wait for rendering
    PROGRESS_BAR.join().unwrap();

    Ok(())
}
