// Simple web proxy
#[macro_use]
extern crate log;
extern crate env_logger;
use std::net::{Shutdown, TcpListener, TcpStream};
use std::thread;
use std::io::{self, BufRead, BufReader, ErrorKind, Read, Write};

struct CopyError {
    bytes_read: u64,
    error: io::Error,
}

struct CopyError (());

// Modified version of io::copy() that returns bytes read on error
fn copy<R: ?Sized, W: ?Sized>(reader: &mut R, writer: &mut W) -> io::Result<u64>
    where R: Read, W: Write
{
    let mut buf = [0; std::io::DEFAULT_BUF_SIZE];
    let mut written = 0;
    loop {
        let len = match reader.read(&mut buf) {
            Ok(0) => return Ok(written),
            Ok(len) => len,
            Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return Err(CopyError { bytes_read: written, error: e}),
        };
        try!(writer.write_all(&buf[..len]));
        written += len as u64;
    }
}

fn handle_client(source: TcpStream) -> Result<(), io::Error> {
    let peer_addr = try!(source.peer_addr());
    info!("Connect: {}", peer_addr);
    let mut source_clone = try!(source.try_clone());

    // Read & parse GET request
    let mut reader = BufReader::new(source);
    let mut line = String::new();
    try!(reader.read_line(&mut line));
    let args = line.split_whitespace().collect::<Vec<_>>();
    if args.len() < 2 {
        warn!("Parse fail: {} \"{}\"", peer_addr, line);
        return Ok(())}
    let command = args[0].to_uppercase();
    let host = match command.as_ref() {
        "CONNECT" => args[1],
        // Most likely a get or post request, strip protocol prefix
        _ => { let sub_args = args[1].split('/').collect::<Vec<_>>();
               if sub_args.len() >= 3 { sub_args[2]} else { args[1] }}
    }.to_string();
    let host_port = match host.find(':') {
        Some(_) => host,
        None => host + ":80",
    };
    info!("Connect: {} -> {}", peer_addr, host_port);

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

    // We lose the byte count if the connection is terminated abnormally
    if bytes_up + bytes_down > 0 {
        info!("Disconnect: {} - {} ({} UP, {} DOWN)",
              peer_addr, host_port, bytes_up, bytes_down);
    } else {
        info!("Reset: {} - {}", peer_addr, host_port);
    }

    Ok(())
}


fn err_exit(err_msg: &str) -> ! {
    writeln!(io::stderr(), "{}", err_msg).unwrap();
    writeln!(io::stderr(), "Usage: webproxy <bind_ip:port>").unwrap();
    std::process::exit(1);
}

fn main() {
    env_logger::init().unwrap();

    info!("Starting up");
    let args = std::env::args().collect::<Vec<String>>();
    if args.len() < 2 { err_exit("Insufficient parameters")}
    let bind_addr: &str = args[1].as_ref();

    let listener = match TcpListener::bind(bind_addr) {
        Ok(a) => a,
        Err(_) => err_exit("Unable to bind to requested host:port"),
    };
    info!("Listener bound to {}", bind_addr);

    // Accept & process incoming connections
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => { thread::spawn(move||handle_client(stream)); }
            Err(e) => { warn!("Connection failed: {:?}", e) }
        }
    }
}
