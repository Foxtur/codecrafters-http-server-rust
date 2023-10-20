use std::io::{Write, Result};
use std::net::{TcpListener, TcpStream};

fn handle_connection(mut stream: TcpStream) -> Result<()>{
    let ok_response = b"HTTP/1.1 200 OK\r\n\r\n";
    let bytes_written = stream.write(ok_response)?;
    println!("Wrote {} bytes", bytes_written);
    stream.flush()?;
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
