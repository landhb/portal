extern crate portal_lib as portal;

use portal::Portal;
use std::net::TcpStream;
use std::io::Write;
use std::error::Error;
use clap::{Arg, App, SubCommand,AppSettings};
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use colored::*;
use serde::{Serialize, Deserialize};
use confy;
use dns_lookup::lookup_host;

#[derive(Serialize, Deserialize, Debug)]
struct AppConfig {
    relay_host: String,
    relay_port: u16,
}

impl ::std::default::Default for AppConfig {
    fn default() -> Self { 
        Self {
            relay_host: String::from("portal-relay.landhb.dev"),
            relay_port: portal::DEFAULT_PORT,
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


fn transfer(mut portal: Portal, msg: Vec<u8>, file_path: Option<&str>, mut client: std::net::TcpStream, is_reciever: bool) -> Result<(), Box<dyn Error>>  {

    /*
     * Step 1: Portal Request
     */
    let req = portal.serialize()?;
    client.write_all(&req)?;

    /*
     * Step 2: Portal Response/Acknowledgement of peering
     */
    log_status!("Waiting for peer to connect...");
    let resp = Portal::read_response_from(&mut client)?;

    /*
     * Step 3: PAKE2 msg exchange
     */
    client.write_all(&msg)?;
    log_status!("Waiting for PAKE2 msg exchange...");
    let confirm_msg = Portal::read_confirmation_from(&mut client)?;


    /*
     * Step 4: Key derivation
     */
    portal.confirm_peer(&confirm_msg).unwrap();
    log_success!("Peer confirmed!");
        
    /*
     * Step 5: Begin file transfer
     */
    let mut total = 0;
    let pstyle = 
        ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .progress_chars("#>-");

    match is_reciever {

        true => {

            let fname = resp.get_file_name()?;
            let fsize = resp.get_file_size();
            log_success!("Your transfer ID is: {:?}", resp.get_id());
            log_status!("Downloading file: {:?}, size: {:?}", fname, fsize);

            let pb = ProgressBar::new(fsize);
            pb.set_style(pstyle);

            // create outfile
            let file = portal.create_file("/tmp/test")?;

            // Receive until connection is done
            while let Ok(len) = file.process_next_chunk(&client) {
                total += len;
                pb.set_position(total as u64);
            }

            pb.finish_with_message(format!("Downloaded {:?}", fname).as_str());
        }
        false => {

            let id = resp.get_id();
            log_success!("Your transfer ID is: {:?}", id);
            log_status!("Sending file: {:?}", file_path.unwrap());

            let pb = ProgressBar::new(portal.get_file_size());
            pb.set_style(pstyle);

            // open file read-only for sending
            let file = portal.load_file(file_path.unwrap())?;

            // This will be empty for files created with create_file()
            let csize = 16384;
            let chunks = portal.get_chunks(&file,csize);

            for data in chunks.into_iter() {
                client.write_all(&data)?;
                total += csize; 
                pb.set_position(total as u64);
            }

            pb.finish_with_message(format!("Sent {:?} bytes", total).as_str());
        }
    }

    Ok(())
}


fn main() -> Result<(), Box<dyn Error>> {

    let matches = App::new(env!("CARGO_PKG_NAME"))
                  .version(env!("CARGO_PKG_VERSION"))
                  .author(env!("CARGO_PKG_AUTHORS"))
                  .about("Quick & Safe File Transfers")
                  .setting(AppSettings::ArgRequiredElseHelp)
                  .subcommand(SubCommand::with_name("send")
                              .about("Send a file")
                              .arg(Arg::with_name("filename")
                                  .short("f")
                                  .takes_value(true)
                                  .required(true)
                                  .help("file to transfer"))
                              .arg(Arg::with_name("password")
                                  .short("i")
                                  .takes_value(true)
                                  .required(true)
                                  .help("id to send to"))
                  )
                  .subcommand(SubCommand::with_name("recv")
                              .about("Recieve a file")
                              .arg(Arg::with_name("timeout")
                                  .short("t")
                                  .takes_value(true)
                                  .required(false)
                                  .help("Timeout for the transfer"))
                  )
                  .get_matches();


    // Load/create config location
    let cfg: AppConfig = confy::load(env!("CARGO_PKG_NAME"))?;
    log_status!("Using portal.toml config, relay: {}!", cfg.relay_host.yellow());

    // Determin the IP address to connect to
    let addr: std::net::IpAddr = match cfg.relay_host.parse() {
        Ok(res) => res,
        Err(_) => {
            let ips: Vec<std::net::IpAddr> = lookup_host(&cfg.relay_host).unwrap();
            ips[0]
        }
    };

    log_success!("Resolved relay to {:?} port {}!", addr, cfg.relay_port);
    
    let addr: std::net::SocketAddr = format!("{}:{}",addr, cfg.relay_port).parse()?;


    let client = match TcpStream::connect_timeout(&addr, std::time::Duration::new(3, 0)) {
        Ok(res) => res,
        Err(e) => {
            log_error!("Failed to connect");
            return Err(e.into());
        }
    };
    log_success!("Connected to {:?}!", addr);

    match matches.subcommand() {
        ("send", Some(args)) =>  { 

            // TODO: generate unique string here
            let pass = String::from("testpasswd");
            let file = args.value_of("filename").unwrap();

            let (mut req,msg) = Portal::init(
                Some(portal::Direction::Sender),
                pass,
                Some(file.to_string()),
            );

            let metadata = std::fs::metadata(file)?;
            req.set_file_size(metadata.len());

            transfer(req,msg, args.value_of("filename"),client, false)?;
            
        },
        ("recv", Some(args)) =>  { 


            let pass = rpassword::read_password_from_tty(Some("Password: ")).unwrap();


            let (req,msg) = Portal::init(
                Some(portal::Direction::Receiver),
                pass,
                None, // receiver will get the filename from the sender
            );

            transfer(req,msg, args.value_of("filename"),client, true)?;

        },
        _ => {println!("Please provide a valid subcommand. Run portal -h for more information.");},
    }

    Ok(())
}
