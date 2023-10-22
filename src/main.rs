use std::io;
use io::Error;
use nom::AsBytes;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[allow(dead_code)]
enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    UPDATE,
}

#[allow(dead_code)]
struct HttpRequest<'a> {
    host: &'a str,
    port: u32,
    method: HttpMethod,
    scheme: &'a str,
    path: &'a str,
    http_version: String,
    headers: Vec<&'a str>,
    content: Vec<u8>
}

#[allow(dead_code)]
struct HttpResponse<'a> {
    http_version: &'a str,
    status_code: u16,
    reason: &'a str,
    headers: Vec<&'a str>,
    content: Vec<u8>
}

fn parse_method(method_str: &str) -> Result<HttpMethod, Error> {
    match method_str {
        "GET" => Ok(HttpMethod::GET),
        _ => Err(Error::new(std::io::ErrorKind::InvalidData, "Invalid HTTP method"))
    }
}

#[allow(dead_code)]
fn parse_request(data: &[u8]) -> Result<HttpRequest, io::Error> {
    let string_data = std::str::from_utf8(data).unwrap();
    println!("Received data: {}", string_data);

    let mut method: Option<HttpMethod> = Option::None;
    let mut path: &str = "";
    let mut http_version: &str = "";
    let mut headers: Vec<&str> = Vec::new();

    let mut encountered_newlines = 0;
    for (line_num, line) in string_data.lines().enumerate() {
        if line_num == 0 {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let method_str = parts[0];
            method = Some(parse_method(method_str)?);
            path = parts[1];
            http_version = parts[2];
        } else if line.is_empty() {
            encountered_newlines += 1;
        } else if encountered_newlines == 2 {
            // TODO: fill body
        } else {
            headers.push(line);
        }
    }

    Ok(HttpRequest {
        host: "localhost",
        port: 80,
        method: method.unwrap(),
        scheme: "http",
        path,
        http_version: http_version.to_string(),
        headers,
        content: Vec::new()
    })
}

fn handle_ok() -> HttpResponse<'static> {
    HttpResponse {
        http_version: "HTTP/1.1",
        status_code: 200,
        reason: "OK",
        headers: Vec::new(),
        content: Vec::new()
    }
}

fn handle_not_found() -> HttpResponse<'static> {
    HttpResponse {
        http_version: "HTTP/1.1",
        status_code: 404,
        reason: "NOT FOUND",
        headers: Vec::new(),
        content: Vec::new()
    }
}

fn handle_echo(request: HttpRequest) -> HttpResponse {
    let echo_text = request.path.replace("/echo/", "");
    let mut headers = Vec::new();
    headers.push("Content-Type: text/plain");

    HttpResponse {
        http_version: "HTTP/1.1",
        status_code: 200,
        reason: "OK",
        headers,
        content: echo_text.as_bytes().to_vec(),
    }
}

fn handle_user_agent(request: HttpRequest) -> HttpResponse {
    println!("handle_user_agent");
    for header in &request.headers {
        println!("Header: {}", header);
    }

    let user_agent = &request.headers
        .iter()
        .find(|&&header| header.starts_with("User-Agent"))
        .map(|&header| header.replace("User-Agent: ", ""))
        .unwrap_or("Unkown".to_string());

    let headers = vec!["Content-Type: text/plain"];

    HttpResponse {
        http_version: "HTTP/1.1",
        status_code: 200,
        reason: "OK",
        headers,
        content: user_agent.as_bytes().to_vec(),
    }
}

fn http_response_to_string(response: &HttpResponse) -> Vec<u8> {
    let mut http_string: String = "".to_owned();

    // http status line
    http_string.push_str(format!("{} {} {}\r\n",
                                 response.http_version,
                                 response.status_code,
                                 response.reason
    ).as_str());

    // headers
    for header in &response.headers {
        http_string.push_str(format!("{}\r\n", &header).as_str());
    }

    // content-lenght and newline
    if response.content.is_empty() {
        http_string.push_str("Content-Length: 0\r\n\r\n");
    } else {
        http_string.push_str(
            format!("Content-Length: {}\r\n\r\n", response.content.len()).as_str()
        )
    }


    // body
    let mut http_message = http_string.to_string().as_bytes().to_vec();
    http_message.extend(response.content.clone());
    http_message
}

fn handle_request(request: HttpRequest) -> HttpResponse {

    match request.path {
        "/" => handle_ok(),
        "/user-agent" => handle_user_agent(request),
        _ if request.path.starts_with("/echo/") => {
            handle_echo(request)
        },
        _ => handle_not_found(),
    }

}

async fn handle_client(mut socket: TcpStream) {
    loop {
        let mut buffer = [0; 1024];

        loop {
            match socket.read(&mut buffer).await {
                Ok(0) => return,
                Ok(n) => {
                    println!("Read {} bytes", n);
                    match parse_request(&buffer[..n]) {
                        Ok(request) => {
                            let response = handle_request(request);
                            println!(
                                "Responding with {} {}",
                                response.status_code,
                                response.reason
                            );
                            let response_str = http_response_to_string(&response);
                            if socket.write_all(response_str.as_bytes()).await.is_err() {
                                eprintln!("Failed writing to socket!");
                                return;
                            }
                        }
                        Err(_) => return
                    }

                    return
                },
                Err(_) => {
                    return;
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").await.unwrap();

    loop {
        let (socket, _) = listener.accept().await.unwrap();

        tokio::spawn(async move {
            handle_client(socket).await;
        });
    }
}
