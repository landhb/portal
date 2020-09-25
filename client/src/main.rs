extern crate portal_lib as portal;

use portal::Portal;
use std::net::TcpStream;
use std::io::Write;
use std::error::Error;
use clap::{Arg, App, SubCommand,AppSettings};
use anyhow::Result;

mod networking;

fn main() -> Result<(), Box<dyn Error>> {

    let matches = App::new(env!("CARGO_PKG_NAME"))
                  .version(env!("CARGO_PKG_VERSION"))
                  .author(env!("CARGO_PKG_AUTHORS"))
                  .about("Quick File Transfers")
                  .setting(AppSettings::ArgRequiredElseHelp)
                  .subcommand(SubCommand::with_name("send")
                              .about("Send a file")
                              .arg(Arg::with_name("filename")
                                  .short("f")
                                  .takes_value(true)
                                  .required(true)
                                  .help("file to transfer"))
                              .arg(Arg::with_name("id")
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



    let addr: std::net::SocketAddr = "127.0.0.1:13265".parse()?;

    match matches.subcommand() {
        ("send", Some(args)) =>  { 

            let req = Portal::init(
                Some(portal::Direction::Sender),
                Some(args.value_of("id").unwrap().to_string()),
                None,
            );

            transfer(req,args.value_of("filename"),addr, false)?;
            
        },
        ("recv", Some(args)) =>  { 

            let req = Portal::init(
                Some(portal::Direction::Reciever),
                None,
                Some(String::from("Test")),
            );

            transfer(req,args.value_of("filename"),addr, true)?;

        },
        _ => {println!("Please provide a valid subcommand. Run portal -h for more information.");},
    }

    Ok(())
}

fn transfer(portal: Portal, file_path: Option<&str>, addr: std::net::SocketAddr, is_reciever: bool) -> Result<(), Box<dyn Error>>  {
    
    // Setup the client socket.
    let mut client = TcpStream::connect(addr)?;
    println!("[+] Connected {:?}", client);

    let req = portal.serialize()?;
    client.write_all(&req)?;
    println!("[+] Sent {:?}", req);

    let mut received_data = Vec::with_capacity(8192);
    networking::recv_generic(&mut client, &mut received_data)?;    

    // attempt to deserialize the portal response
    let resp = Portal::parse(&received_data.to_vec())?;
    println!("[+] Recieved {:?}", resp);
        
    let mut total = 0;

    match is_reciever {

        true => {

            println!("[+] Your transfer ID is: {:?}", resp.get_id().unwrap());

            // create outfile
            let file = portal.create_file("/tmp/test")?;

            // Receive until connection is done
            let mut len = 1;
            while len != 0 {
                received_data.clear();
                len = networking::recv_generic(&mut client, &mut received_data)?;
                file.write(&received_data)?;
                total += len;
            }

        }
        false => {

            let pubkey = resp.get_pubkey().unwrap();
            println!("[+] Received client public key: {:?}", pubkey);

            // open file read-only for sending
            let file = portal.load_file(file_path.unwrap())?;

            // This will be empty for files created with create_file()
            let chunks = portal.get_chunks(&file,8192);

            for data in chunks.into_iter() {
                client.write_all(&data)?;
                total += data.len();
            }
        }
    }
    

    println!("[+] transferred {:?}", total);

    Ok(())
}