use std::io::{Write, Read, Error};
use std::net::{TcpListener, TcpStream};

#[allow(dead_code)]
enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    UPDATE,
}

#[allow(dead_code)]
struct HttpRequest {
    host: String,
    port: u32,
    method: HttpMethod,
    scheme: String,
    path: String,
    http_version: String,
    headers: Vec<String>,
    content: Vec<u8>
}

#[allow(dead_code)]
struct HttpResponse {
    http_version: String,
    status_code: u16,
    reason: String,
    headers: Vec<String>,
    content: Vec<u8>
}

fn parse_method(method_str: &str) -> Result<HttpMethod, Error> {
    match method_str {
        "GET" => Ok(HttpMethod::GET),
        _ => Err(Error::new(std::io::ErrorKind::InvalidData, "Invalid HTTP method"))
    }
}

#[allow(dead_code)]
fn parse_request(stream: &mut TcpStream) -> Result<HttpRequest, Error> {
    let mut buffer = [0; 1024];
    match stream.read(&mut buffer) {
        Ok(bytes_read) => {
            println!("Bytes Read: {}", bytes_read);

            let data = std::str::from_utf8(&buffer).unwrap();
            println!("Received data: {}", data);

            let mut method: Option<HttpMethod> = Option::None;
            let mut path: &str = "";
            let mut http_version: &str = "";

            let mut encountered_newlines = 0;
            for (line_num, line) in data.lines().enumerate() {
                if line_num == 0 {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    let method_str = parts[0];
                    method = Some(parse_method(method_str)?);
                    path = parts[1];
                    http_version = parts[2];
                }

                if line.is_empty() {
                    encountered_newlines += 1;
                } else if encountered_newlines == 2 {
                    // TODO: fill body
                }
            }

            Ok(HttpRequest {
                host: "localhost".to_string(),
                port: 80,
                method: method.unwrap(),
                scheme: "http".to_string(),
                path: path.to_string(),
                http_version: http_version.to_string(),
                headers: Vec::new(),
                content: Vec::new()
            })
        },
        Err(e) => {
            let error_message = "Failed parsing request";
            println!("{}", error_message);
            Err(e)
        }
    }

}

fn handle_echo(request: HttpRequest, stream: &mut TcpStream) -> usize {
    let echo_text = request.path.replace("/echo/", "");
    return stream.write(
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length:{}\r\n\r\n{}",
            echo_text.len(),
            echo_text
        ).as_bytes()
    ).unwrap();
}

fn handle_connection(mut stream: TcpStream) -> Result<(), std::io::Error>{
    let ok_response = b"HTTP/1.1 200 OK\r\n\r\n";
    let not_found_response = b"HTTP/1.1 404 NOT FOUND\r\n\r\n";

    let request = parse_request(&mut stream)?;

    let bytes_written: usize;
    match request.path.as_str() {
        "/" => {
            bytes_written = stream.write(ok_response)?;
        },

        _ if request.path.starts_with("/echo/") => {
            bytes_written = handle_echo(request, &mut stream);
        }

        _ => {
            bytes_written = stream.write(not_found_response)?;
        }
    }
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
