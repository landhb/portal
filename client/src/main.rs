extern crate portal_lib as portal;

use anyhow::Result;
use clap::{App, AppSettings, Arg, SubCommand};
use colored::*;
use directories::UserDirs;
use dns_lookup::lookup_host;
use indicatif::{ProgressBar, ProgressStyle};
use portal::{Direction, Portal};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::net::TcpStream;
use std::path::Path;

#[macro_use]
extern crate lazy_static;

mod wordlist;
use wordlist::gen_phrase;

#[derive(Serialize, Deserialize, Debug)]
struct AppConfig {
    relay_host: String,
    relay_port: u16,
    download_location: String,
}

impl ::std::default::Default for AppConfig {
    fn default() -> Self {
        // select ~/Downloads or /tmp for downloads
        let hdir = UserDirs::new();
        let ddir = match &hdir {
            Some(home) => home.download_dir().map_or("/tmp", |v| v.to_str().unwrap()),
            None => "/tmp",
        };

        Self {
            relay_host: String::from("portal-relay.landhb.dev"),
            relay_port: portal::DEFAULT_PORT,
            download_location: String::from(ddir),
        }
    }
}

macro_rules! log_status {
    ($($arg:tt)*) => (println!("{} {}", "[*]".blue().bold(), format_args!($($arg)*)));
}

macro_rules! log_error {
    ($($arg:tt)*) => (println!("{} {}", "[!]".red().bold(), format_args!($($arg)*)));
}

macro_rules! log_success {
    ($($arg:tt)*) => (println!("{} {}", "[+]".green().bold(), format_args!($($arg)*)));
}
/*
macro_rules! log_wait {
    ($($arg:tt)*) => (print!("{} {}", "[...]".yellow().bold(), format_args!($($arg)*)); std::io::stdout().flush().unwrap(););
}*/

fn transfer(
    mut portal: Portal,
    fpath: &str,
    mut client: std::net::TcpStream,
) -> Result<(), Box<dyn Error>> {
    let pstyle = ProgressStyle::default_bar()
        .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .progress_chars("#>-");

    // Start the progress bar hidden
    let pb = ProgressBar::new(0); //new(fsize);
    pb.set_style(pstyle);

    // User callback to display progress
    let progress = |transferred: usize| {
        pb.set_position(transferred as u64);
    };

    match portal.get_direction() {
        Direction::Receiver => {
            // User callback to confirm/deny a transfer
            let confirm_download = |path: &str, size: u64| -> bool {
                log_status!("Downloading file: {:?}, size: {:?}", path, size);
                pb.set_length(size);
                true
            };

            log_status!("Waiting for peer to begin transfer...");
            let metadata = match portal.recv_file(
                &mut client,
                Path::new(fpath),
                Some(confirm_download),
                Some(progress),
            ) {
                Ok(result) => result,
                Err(e) => {
                    log_error!("Failed to recv file.");
                    return Err(e);
                }
            };

            let fname = std::str::from_utf8(&metadata.filename).unwrap();
            pb.finish_with_message(format!("Downloaded {:?}", fname).as_str());
        }
        Direction::Sender => {
            log_status!("Starting transfer...");

            // Obtain file size for progress bar
            let metadata = std::fs::metadata(fpath)?;
            pb.set_length(metadata.len());
            let sent = match portal.send_file(&mut client, fpath, Some(progress)) {
                Ok(total) => total,
                Err(e) => {
                    log_error!("Failed to send file.");
                    return Err(e);
                }
            };

            pb.finish_with_message(format!("Sent {:?} bytes", sent).as_str());
        }
    }

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
    let mut cfg: AppConfig = confy::load("portal")?; // CARGO_BIN_NAME is nightly only
    log_status!(
        "Using portal.toml config, relay: {}!",
        cfg.relay_host.yellow()
    );

    // Determin the IP address to connect to
    let addr: std::net::IpAddr = match cfg.relay_host.parse() {
        Ok(res) => res,
        Err(_) => {
            let ips: Vec<std::net::IpAddr> = lookup_host(&cfg.relay_host).unwrap();
            ips[0]
        }
    };

    let addr: std::net::SocketAddr = format!("{}:{}", addr, cfg.relay_port).parse()?;

    let mut client = match TcpStream::connect_timeout(&addr, std::time::Duration::new(3, 0)) {
        Ok(res) => res,
        Err(e) => {
            log_error!("Failed to connect");
            return Err(e.into());
        }
    };
    log_success!("Connected to {:?}!", addr);

    let (direction, id, pass, path) = match matches.subcommand() {
        ("send", Some(args)) => {
            let id = gen_phrase(1);
            let pass = gen_phrase(3);

            log_success!(
                "Tell your peer their pass-phrase is: {:?}",
                format!("{}-{}", id, pass)
            );

            let file = args.value_of("filename").unwrap();

            (Direction::Sender, id, pass, file.to_string())
        }
        ("recv", Some(args)) => {
            let pass = rpassword::read_password_from_tty(Some("Enter pass-phrase: ")).unwrap();

            // check if we need to override the download location
            if args.is_present("download_folder") {
                cfg.download_location = args.value_of("download_folder").unwrap().to_string();
            }

            // Parse ID from password
            let mut pass = pass.split('-');
            let id = pass.next().unwrap().to_string();
            let opass = pass.collect::<Vec<&str>>().join("-");

            (Direction::Receiver, id, opass, cfg.download_location)
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

    Ok(())
}
