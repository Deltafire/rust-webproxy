// Simple web proxy
use std::net::{TcpListener, TcpStream, Shutdown};
use std::thread;
use std::io::{copy, BufReader, Write, BufRead};
//use std::io::prelude::*;

fn handle_client(mut source: TcpStream) {
    // Read & parse GET request
    println!("Client connected: {}", source.peer_addr().unwrap());
    let mut source_clone = source.try_clone().unwrap();
    let mut connect_string = String::new();
    {
        let source = source.try_clone().unwrap();
        let mut reader = BufReader::new(source);
        let bytes_read = reader.read_line(&mut connect_string).unwrap();
        println!("Read {} bytes:\n{}\n", bytes_read, &connect_string);
    }
    let host = parse_connect(&connect_string).unwrap();

    // Connect to remote server
    let mut dest = TcpStream::connect(&host).unwrap();
    let mut dest_clone = dest.try_clone().unwrap();

    let _ = dest.write(&connect_string.as_bytes()).unwrap();

    thread::spawn(move|| {
        let _ = copy(&mut dest_clone, &mut source_clone);
        let _ = source_clone.shutdown(Shutdown::Both);
        });
    let _ = copy(&mut source, &mut dest);

    println!("Client disconnecting: {}", dest.peer_addr().unwrap());
    let _ = dest.shutdown(Shutdown::Both);
}

fn parse_connect(connect_string: &String) -> Result<&str, String> {
    // Format is "CONNECT host:port HTTP/1.1"
    println!("Parsing {}", &connect_string);
    let words: Vec<&str> = connect_string.split_whitespace().collect();
    if words[0] != "CONNECT" && words[2] != "HTTP/1.1" {
        return Err("Parse error: ".to_string() + &connect_string);
    }
    return Ok(words[1]);
}

#[test]
fn test_parse_connect() {
    assert_eq!(
        "test.com:888",
        parse_connect(&"CONNECT test.com:888 HTTP:/1.1".to_string()).unwrap())
}

#[test]
#[should_panic]
fn test_panic_parse_connect() {
    let _ = parse_connect(&"blah blah".to_string());
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
