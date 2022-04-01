extern crate portal_lib as portal;

use anyhow::Result;
use colored::*;
use dns_lookup::lookup_host;
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use portal::errors::PortalError;
use std::error::Error;
use std::net::TcpStream;
use std::path::PathBuf;
use structopt::StructOpt;

#[macro_use]
extern crate lazy_static;

#[macro_use]
mod macros;
mod config;
use config::AppConfig;

/// EFF's dice generated wordlist
mod wordlist;

/// Receiver path
mod receive;
use receive::recv_all;

/// Sender path
mod send;
use send::send_all;

lazy_static! {
    /// Global multi-bar that contains other progress bars
    pub static ref MULTI: MultiProgress = MultiProgress::with_draw_target(ProgressDrawTarget::stdout());

    /// All bars have the same style
    pub static ref PSTYLE: ProgressStyle = ProgressStyle::default_bar()
        .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta}) {msg}")
        .progress_chars("#>-");
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "portal",
    author = "landhb",
    about = "Quick & Safe File Transfers"
)]
enum Command {
    /// Send file(s) to a peer
    Send {
        /// List of files to send
        #[structopt(parse(from_os_str))]
        files: Vec<PathBuf>,
    },

    /// Recv file(s) from a peer
    Recv {
        /// Optional: override the download directory in the config file.
        #[structopt(short, long)]
        download_dir: Option<PathBuf>,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    // Parse CLI args
    let cmd = Command::from_args();

    // Fix terminal output on windows
    #[cfg(target_os = "windows")]
    control::set_virtual_terminal(true).unwrap();

    // Load/create config location
    let mut cfg: AppConfig = confy::load("portal")?;
    log_status!(
        "Using portal.toml config, relay: {}!",
        cfg.relay_host.yellow()
    );

    // Check if we need to override the download location
    if let Command::Recv { download_dir } = &cmd {
        cfg.download_location = download_dir
            .as_ref()
            .map_or(cfg.download_location, |val| val.clone());
    }

    // Determin the IP address to connect to
    let addr: std::net::IpAddr = match cfg.relay_host.parse() {
        Ok(res) => res,
        Err(_) => *lookup_host(&cfg.relay_host)?
            .first()
            .ok_or(PortalError::NoPeer)?,
    };

    // Use the port config value to create an IP/port pair
    let addr: std::net::SocketAddr = format!("{}:{}", addr, cfg.relay_port).parse()?;

    // Connect to the relay
    let mut client =
        TcpStream::connect_timeout(&addr, std::time::Duration::new(6, 0)).map_err(|e| {
            log_error!("Failed to connect");
            e
        })?;
    log_success!("Connected to {:?}!", addr);

    // Create a hidden bar so the progress bar doesn't
    // go out of scope.
    let hidden = MULTI.add(ProgressBar::hidden());

    // Start rendering the bars
    let thread = std::thread::spawn(|| {
        MULTI.join().unwrap();
    });

    // Begin the transfer
    let result = match cmd {
        Command::Send { files } => send_all(&mut client, files),
        Command::Recv { .. } => recv_all(&mut client, cfg.download_location),
    };

    // Allow the hidden bar to go out of scope
    // which allows the global one to as well
    hidden.finish_and_clear();
    thread.join().unwrap();

    match result {
        Ok(_) => log_success!("Complete!"),
        Err(e) => log_error!("{:?}", e),
    }

    Ok(())
}
