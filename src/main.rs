// Simple web proxy
#[macro_use]
extern crate log;
extern crate env_logger;
use std::net::{Shutdown, TcpListener, TcpStream};
use std::thread;
use std::io::{self, BufRead, BufReader, copy, Write};
//use std::io::prelude::*;

fn handle_client(source: TcpStream) -> Result<(), io::Error> {
    let peer_addr = try!(source.peer_addr());
    info!("Client connected: {}", peer_addr);
    let mut source_clone = try!(source.try_clone());

    // Read & parse GET request
    let mut reader = BufReader::new(source);
    let mut line = String::new();
    try!(reader.read_line(&mut line));
    let args = line.split_whitespace().collect::<Vec<_>>();
    if args.len() < 2 { return Ok(())}
    let command = args[0].to_uppercase();
    let host = match command.as_ref() {
        "CONNECT" => args[1],
        // Most likely a get or post request, remove http://
        _ => { let sub_args = args[1].split('/').collect::<Vec<_>>();
               if sub_args.len() >= 3 { sub_args[2]} else { args[1] }}
    }.to_string();
    let host_port = match host.find(':') {
        Some(_) => host,
        None => host + ":80",
    };
    info!("Destination: {}", host_port);

    // Connect to remote server
    let mut dest = try!(TcpStream::connect(&host_port as &str));
    if command.eq("CONNECT") {
        // Consume headers
        let mut line = String::new();
        while line.ne("\0x0d\0x0a") {
            line.clear();
            if try!(reader.read_line(&mut line)) == 0 { break }
        }
        let _ = source_clone.write(b"200 Connected\r\n");
    } else {
        let _ = dest.write(line.as_ref());
    }

    // Copy streams
    let mut dest_clone = try!(dest.try_clone());
    let child = thread::spawn(move|| {
        let ret = copy(&mut dest_clone, &mut source_clone);
        let _ = source_clone.shutdown(Shutdown::Both);
        ret.unwrap_or(0)
        });
    let bytes_up = copy(&mut reader, &mut dest).unwrap_or(0);
    let _ = dest.shutdown(Shutdown::Both);
    let bytes_down = child.join().unwrap_or(0);

    info!("Client disconnected: {} ({} UP, {} DOWN)",
          peer_addr, bytes_up, bytes_down);
    Ok(())
}

fn main() {
    env_logger::init().unwrap();

    info!("Starting up");
    // Config
    const BIND_ADDR: &'static str = "127.0.0.1:1234";

    let listener = TcpListener::bind(&BIND_ADDR).unwrap();

    // Accept & process incoming connections
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => { thread::spawn(move||handle_client(stream)); }
            Err(e) => { println!("Connection failed: {:?}", e) }
        }
    }
}
