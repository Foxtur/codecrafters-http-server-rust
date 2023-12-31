use io::Error;
use nom::AsBytes;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, Read};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[derive(PartialEq)]
enum HttpMethod {
    GET,
    POST,
}

impl HttpMethod {
    fn parse_method(method_str: &str) -> std::io::Result<HttpMethod> {
        match method_str {
            "GET" => Ok(HttpMethod::GET),
            "POST" => Ok(HttpMethod::POST),
            _ => Err(Error::new(
                std::io::ErrorKind::InvalidData,
                "Unsupported HTTP method",
            )),
        }
    }
}

struct HttpRequest<'a> {
    method: HttpMethod,
    path: &'a str,
    headers: Vec<&'a str>,
    content: Vec<u8>,
}

impl<'a> HttpRequest<'a> {
    fn parse_request(data: &[u8]) -> std::io::Result<HttpRequest> {
        let string_data = std::str::from_utf8(data).unwrap();
        println!("Parsing Request - Received data:\n{}", string_data);

        let mut method: Option<HttpMethod> = None;
        let mut path: &str = "";
        let mut headers: Vec<&str> = Vec::new();
        let mut content = Vec::new();

        let mut encountered_newlines = 0;
        for (line_num, line) in string_data.lines().enumerate() {
            if line_num == 0 {
                let parts: Vec<&str> = line.split_whitespace().collect();
                let method_str = parts[0];
                method = Some(HttpMethod::parse_method(method_str)?);
                path = parts[1];
            } else if line.is_empty() {
                encountered_newlines += 1;
            } else if encountered_newlines == 1 {
                content.extend(line.as_bytes())
            } else {
                headers.push(line);
            }
        }

        Ok(HttpRequest {
            method: method.unwrap(),
            path,
            headers,
            content,
        })
    }
}

struct HttpResponse<'a> {
    http_version: &'a str,
    status_code: u16,
    reason: &'a str,
    headers: Option<Vec<&'a str>>,
    content: Option<Vec<u8>>,
}

impl<'a> HttpResponse<'a> {
    fn ok(headers: Option<Vec<&'a str>>, content: Option<Vec<u8>>) -> HttpResponse<'a> {
        HttpResponse {
            http_version: "HTTP/1.1",
            status_code: 200,
            reason: "OK",
            headers,
            content,
        }
    }

    fn created() -> HttpResponse<'a> {
        HttpResponse {
            http_version: "HTTP/1.1",
            status_code: 201,
            reason: "Created",
            headers: None,
            content: None,
        }
    }

    fn not_found() -> HttpResponse<'a> {
        HttpResponse {
            http_version: "HTTP/1.1",
            status_code: 404,
            reason: "NOT FOUND",
            headers: None,
            content: None,
        }
    }

    fn internal_server_error() -> HttpResponse<'a> {
        HttpResponse {
            http_version: "HTTP/1.1",
            status_code: 500,
            reason: "Internal Server Error",
            headers: None,
            content: None,
        }
    }

    fn to_byte_vector(&self) -> Vec<u8> {
        let mut http_string: String = "".to_owned();

        // http status line
        http_string.push_str(
            format!(
                "{} {} {}\r\n",
                self.http_version, self.status_code, self.reason
            )
            .as_str(),
        );

        // headers
        if let Some(headers) = &self.headers {
            for header in headers {
                http_string.push_str(format!("{}\r\n", &header).as_str());
            }
        }

        // content-lenght and newline
        if let Some(content) = &self.content {
            if content.is_empty() {
                http_string.push_str("Content-Length: 0\r\n\r\n");
            } else {
                http_string.push_str(format!("Content-Length: {}\r\n\r\n", content.len()).as_str())
            }
        } else {
            http_string.push_str("\r\n");
        }

        // body
        let mut http_message = http_string.to_string().as_bytes().to_vec();
        if let Some(content) = &self.content {
            http_message.extend(content.clone());
        }
        http_message
    }
}

fn handle_echo(request: HttpRequest) -> HttpResponse {
    let echo_text = request.path.replace("/echo/", "");
    let headers = vec!["Content-Type: text/plain"];
    HttpResponse::ok(Some(headers), Some(echo_text.as_bytes().to_vec()))
}

fn handle_user_agent(request: HttpRequest) -> HttpResponse {
    println!("handle_user_agent");
    for header in &request.headers {
        println!("Header: {}", header);
    }

    let user_agent = &request
        .headers
        .iter()
        .find(|&&header| header.starts_with("User-Agent"))
        .map(|&header| header.replace("User-Agent: ", ""))
        .unwrap_or("Unkown".to_string());

    let headers = vec!["Content-Type: text/plain"];

    HttpResponse::ok(Some(headers), Some(user_agent.as_bytes().to_vec()))
}

fn read_file(file_path: String) -> io::Result<Vec<u8>> {
    let mut f = File::open(file_path)?;
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer)?;
    Ok(buffer)
}

fn handle_file(request: HttpRequest, data_dir: String) -> HttpResponse {
    let path = request.path.replace("/files/", "");
    match read_file(format!("{}/{}", data_dir, path)) {
        Ok(file_content) => {
            println!("{:?}", std::str::from_utf8(&file_content).unwrap());
            let headers = vec!["Content-Type: application/octet-stream"];
            HttpResponse::ok(Some(headers), Some(file_content))
        }
        Err(_) => HttpResponse::not_found(),
    }
}

fn handle_file_upload(request: HttpRequest, data_dir: String) -> HttpResponse {
    let path = request.path.replace("/files/", "");
    println!("handle_file_upload - path: {}", path);

    let file_path = format!("{}{}", data_dir, path);

    let mut file = match File::create(file_path.clone()) {
        Ok(file) => file,
        Err(err) => panic!("Error creating file {:?}", err),
    };

    match file.write_all(&request.content) {
        Ok(_) => println!("File created and written to successfully: {}", file_path),
        Err(e) => println!("Error writing to file: {} - {:?}", file_path, e),
    }

    HttpResponse::created()
}

fn handle_request(request: HttpRequest, data_dir: Option<String>) -> HttpResponse {
    match request.path {
        "/" => HttpResponse::ok(None, None),
        "/user-agent" => handle_user_agent(request),
        _ if request.path.starts_with("/echo/") => handle_echo(request),
        _ if request.path.starts_with("/files/") && request.method == HttpMethod::GET => {
            match data_dir {
                Some(dir) => handle_file(request, dir),
                _ => HttpResponse::internal_server_error(),
            }
        }
        _ if request.path.starts_with("/files/") && request.method == HttpMethod::POST => {
            match data_dir {
                Some(dir) => handle_file_upload(request, dir),
                _ => HttpResponse::internal_server_error(),
            }
        }
        _ => HttpResponse::not_found(),
    }
}

async fn handle_client(mut socket: TcpStream, data_dir: Option<String>) {
    loop {
        let mut buffer = [0; 1024];

        loop {
            match socket.read(&mut buffer).await {
                Ok(0) => return,
                Ok(n) => {
                    match HttpRequest::parse_request(&buffer[..n]) {
                        Ok(request) => {
                            let response = handle_request(request, data_dir);
                            println!(
                                "Responding with {} {}",
                                response.status_code, response.reason
                            );
                            if socket
                                .write_all(&response.to_byte_vector().as_bytes())
                                .await
                                .is_err()
                            {
                                eprintln!("ERROR: Failed writing to socket!");
                                return;
                            }
                        }
                        Err(e) => {
                            eprintln!("ERROR: handle_client: {:?}", e);
                            return;
                        }
                    }
                    // close connection
                    return;
                }
                Err(e) => {
                    eprintln!("ERROR: handle_client: {:?}", e);
                    return;
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let mut data_dir: Option<String> = None;

    if args.len() > 1 {
        println!("Received arguments: {:?}", args);
        match &args[1][..] {
            "--directory" => {
                if args.len() > 2 {
                    let path = &args[2];
                    println!("Setting data dir to: {}", path);
                    data_dir = Some(path.to_string())
                }
            }
            _ => println!("Unknown argument: {}", &args[1]),
        }
    }

    let listener = TcpListener::bind("127.0.0.1:4221").await.unwrap();

    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let data_dir = data_dir.clone();

        tokio::spawn(async move {
            handle_client(socket, data_dir).await;
        });
    }
}
