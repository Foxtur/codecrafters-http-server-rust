use std::io::{Write, Result, Read};
use std::net::{TcpListener, TcpStream};

fn handle_connection(mut stream: TcpStream) -> Result<()>{
    let ok_response = b"HTTP/1.1 200 OK\r\n\r\n";
    let not_found_response = b"HTTP/1.1 404 NOT FOUND\r\n\r\n";

    let mut buffer = [0; 1024];
    println!("Reading data from stream");
    let _bytes_read = stream.read(&mut buffer)?;
    let received_data = std::str::from_utf8(&buffer).unwrap();
    println!("Received data: {}", received_data);

    let start_line = received_data.lines().nth(0).unwrap();
    let path = start_line.split(" ").nth(1).unwrap();
    println!("Extracted path: {}", path);

    if path == "/" {
        let bytes_written = stream.write(ok_response)?;
        println!("Wrote {} bytes", bytes_written);
        stream.flush()?;
    } else {
        let bytes_written = stream.write(not_found_response)?;
        println!("Wrote {} bytes", bytes_written);
        stream.flush()?;
    }

    Ok(())
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("accepted new connection");
                let _ = handle_connection(stream);
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
