// Simple web proxy
use std::net::{TcpListener, TcpStream, Shutdown};
use std::thread;
use std::io::{BufReader, Read, Write, BufRead};
//use std::io::prelude::*;

fn stream_copy<A: BufRead, B: Write> (mut source: A, mut destination: B) {
    let mut buffer = [0; 1024];
    loop {
        println!("Entering loop..");
        let bytes_read = source.read(&mut buffer[..]).unwrap();
        destination.write(&buffer[0..bytes_read]).unwrap();
    }
//    for line in source.lines() {
//        destination.write(line.unwrap().as_bytes()).unwrap();
//    }
}

fn handle_client(stream: TcpStream) {
    // Read & parse GET request
    println!("Client connected: {}", stream.peer_addr().unwrap());
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut connect_string = String::new();
    let bytes_read = reader.read_line(&mut connect_string).unwrap();
    println!("Read {} bytes:\n{}\n", bytes_read, &connect_string);
    let host = parse_connect(&connect_string).unwrap();

    // Connect to remote server
    let mut dest_stream = TcpStream::connect(&host).unwrap();
    let _ = dest_stream.write(&connect_string.as_bytes()).unwrap();

    let dest_reader = BufReader::new(dest_stream.try_clone().unwrap());
    thread::spawn(move||stream_copy(dest_reader, &stream));
    stream_copy(reader, &dest_stream);

    println!("Client disconnecting: {}", dest_stream.peer_addr().unwrap());

    let _ = dest_stream.shutdown(Shutdown::Both);

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
