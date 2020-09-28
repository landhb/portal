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



    let addr: std::net::SocketAddr = "127.0.0.1:13265".parse()?;

    match matches.subcommand() {
        ("send", Some(args)) =>  { 

            // TODO: generate unique string here
            let pass = String::from("testpasswd");
            let file = args.value_of("filename").unwrap();

            let (req,msg) = Portal::init(
                Some(portal::Direction::Sender),
                pass,
                Some(file.to_string()),
            );

            transfer(req,msg, args.value_of("filename"),addr, false)?;
            
        },
        ("recv", Some(args)) =>  { 


            let pass = rpassword::read_password_from_tty(Some("Password: ")).unwrap();


            let (req,msg) = Portal::init(
                Some(portal::Direction::Receiver),
                pass,
                None, // receiver will get the filename from the sender
            );


            transfer(req,msg, args.value_of("filename"),addr, true)?;

        },
        _ => {println!("Please provide a valid subcommand. Run portal -h for more information.");},
    }

    Ok(())
}

fn transfer(mut portal: Portal, msg: Vec<u8>, file_path: Option<&str>, addr: std::net::SocketAddr, is_reciever: bool) -> Result<(), Box<dyn Error>>  {
    

    let mut client = TcpStream::connect(addr)?;
    println!("[+] Connected {:?}", client);


    /*
     * Step 1: Portal Request
     */
    let req = portal.serialize()?;
    client.write_all(&req)?;
    println!("[+] Sent {:?}", portal);

    /*
     * Step 2: Portal Response/Acknowledgement of peering
     */
    let resp = Portal::read_response_from(&mut client)?;
    println!("[+] Recieved {:?}", resp);

    /*
     * Step 3: PAKE2 msg exchange
     */
    client.write_all(&msg)?;
    let confirm_msg = Portal::read_confirmation_from(&mut client)?;


    /*
     * Step 4: Key derivation
     */
    portal.confirm_peer(&confirm_msg).unwrap();

        
    /*
     * Step 5: Begin file transfer
     */
    let mut total = 0;
    //let mut received_data = Vec::with_capacity(8192);
    match is_reciever {

        true => {

            println!("[+] Your transfer ID is: {:?}", resp.get_id());

            // create outfile
            let file = portal.create_file("/tmp/test")?;

            // Receive until connection is done
            let mut len = 1;
            while len != 0 {
                //received_data.clear();
                //len = networking::recv_generic(&mut client, &mut received_data)?;
                //file.write(&received_data)?;
                len = file.process_next_chunk(&client)?;
                total += len;
            }

        }
        false => {

            let id = resp.get_id();
            println!("[+] Sending file to: {:?}", id);

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
