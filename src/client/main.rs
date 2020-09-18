extern crate portal_lib as portal;

use mio::net::TcpStream;
use mio::{Events, Interest, Poll, Token};
use mio::event::Event;
use std::error::Error;
use clap::{Arg, App, SubCommand,AppSettings};
use anyhow::Result;
const CLIENT: Token = Token(0);


fn handle_read(mut conn: &mut TcpStream, _event: &Event) -> Result<usize> { //, _event: &Event
    let mut received_data = Vec::with_capacity(4096);
    portal::portal_recv_data(&mut conn, &mut received_data)
}

fn handle_write(mut conn: &mut TcpStream, _event: &Event) -> Result<()> { // , _event: &Event
    let data = *b"Hello, World!";
    portal::portal_send_data(&mut conn,data.to_vec())
}


fn main() -> Result<(), Box<dyn Error>> {

    let matches = App::new("portal")
                  .version("1.0")
                  .author("Bradley Landherr")
                  .about("Quick File Transfers")
                  .setting(AppSettings::ArgRequiredElseHelp)
                  .subcommand(SubCommand::with_name("send")
                              .about("Send a file")
                              .arg(Arg::with_name("filename")
                                  .short("f")
                                  .takes_value(true)
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

            let req = portal::Request {
                direction: portal::Direction::Sender,
                id: Some(args.value_of("id").unwrap().to_string()),
                pubkey: None,
            };

            transfer(req,addr, false)?;
            
        },
        ("recv", Some(_args)) =>  { 

            let req = portal::Request {
                direction: portal::Direction::Reciever,
                id: None,
                pubkey: Some(String::from("Test")),
            };

            transfer(req,addr, true)?;

        },
        _ => {println!("Please provide a valid subcommand. Run portal -h for more information.");},
    }

    Ok(())
}

fn transfer(req: portal::Request, addr: std::net::SocketAddr, is_reciever: bool) -> Result<(), Box<dyn Error>>  {

    // Create a poll instance.
    let mut poll = Poll::new()?;

    // Create storage for events.
    let mut events = Events::with_capacity(128);

    
    // Setup the client socket.
    let mut client = TcpStream::connect(addr)?;


    portal::portal_send_request(&mut client, req)?;


    // Wait until we get our transfer ID or the peer's public key
    let pubkey;
    let mut interest_opts = Interest::READABLE;
    while let Ok(resp) = portal::portal_get_response(&mut client) {

        if resp == None {
            continue;
        }

        if is_reciever {
            println!("[+] Your transfer ID is: {:?}", resp.unwrap().id);
            interest_opts = Interest::READABLE;
            break;
        }

        pubkey = resp.unwrap().pubkey;
        interest_opts = Interest::WRITABLE;
        println!("[+] Received client public key: {:?}", pubkey);
        break;
    }


    poll.registry().register(&mut client, CLIENT,interest_opts)?;

    
    // main transfer loop
    loop {

        // Poll Mio for events, blocking until we get an event.
        poll.poll(&mut events, None)?;

        println!("poll returned");

        // Process each event.
        for event in events.iter() {

            println!("got event {:?}", event);
           
            if event.is_writable() {
                handle_write(&mut client, &event)?;               
            }

            if event.is_readable() {
                let read = handle_read(&mut client, &event)?;
                println!("finished read {:?}", read); 
            }

        }



    } 

}


