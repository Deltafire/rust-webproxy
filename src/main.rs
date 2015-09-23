// Simple web proxy
use std::net::{TcpListener, TcpStream, Shutdown};
use std::thread;
use std::io::{copy, BufReader, Write, BufRead};
//use std::io::prelude::*;

fn handle_client(source: TcpStream) {
    // Read & parse GET request
    println!("Client connected: {}", source.peer_addr().unwrap());
    let mut source_clone = source.try_clone().unwrap();
    let mut reader = BufReader::new(source);
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    println!("{}", line.trim_right());
    let args = line.split_whitespace().collect::<Vec<_>>();
    let command = args[0].to_uppercase();
    let host = match command.as_ref() {
        "CONNECT" => args[1],
        _ => args[1].split('/').collect::<Vec<_>>()[2],
    }.to_string();
    let host_port = match host.find(':') {
        Some(_) => host,
        None => host + ":80",
    };
    println!("Host: {}", host_port);
    // Connect to remote server
    let mut dest = TcpStream::connect(&host_port as &str).unwrap();
    if command.eq("CONNECT") {
        // Consume empty line
        let mut line = String::new();
        while line.ne("\0x0d\0x0a") {
            line.clear();
            if reader.read_line(&mut line).unwrap() == 0 { break }
        }
        let _ = source_clone.write(b"200 Connected\r\n");
    } else {
        let _ = dest.write(line.as_ref());
    }
    let mut dest_clone = dest.try_clone().unwrap();

    thread::spawn(move|| {
        let _ = copy(&mut dest_clone, &mut source_clone);
        let _ = source_clone.shutdown(Shutdown::Both);
        });
    let _ = copy(&mut reader, &mut dest);

    println!("Client disconnected: {}", dest.peer_addr().unwrap());
    let _ = dest.shutdown(Shutdown::Both);
}

fn main() {
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
