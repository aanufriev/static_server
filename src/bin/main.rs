use std::fs::File;
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use static_server::{ThreadPool, read};
use std::str;
use chrono::prelude::*;
use urlencoding::decode;

const GET: &str = "GET";
const HEAD: &str = "HEAD";
const OK_STATUS: &str = "HTTP/1.1 200 OK";
const FORBIDDEN_STATUS: &str = "HTTP/1.1 403 Forbidden  ";
const NOT_FOUND_STATUS: &str = "HTTP/1.1 404 Not Found";
const NOT_ALLOWED_STATUS: &str = "HTTP/1.1 405 Method Not Allowed";

fn main() {
    let mut config = read().expect("could't read config");
    println!("thread limit: {}\ndocument root: {}", config.thread_limit, config.document_root);
    if config.document_root.chars().last().unwrap() != '/' {
        config.document_root.push_str("/");
    }

    let listener = TcpListener::bind("0.0.0.0:80").expect("couldnt't bind");
    println!("started at port 80");
    let pool = ThreadPool::new(config.thread_limit);
    println!("started thread pool");

    for stream in listener.incoming() {
        let stream = stream.expect("couldnt't unwrap TcpStream");

        let doc_root = config.document_root.clone();
        pool.execute(|| {
            handle_connection(stream, doc_root);
        });
    }
}

fn handle_connection(mut stream: TcpStream, mut document_root: String) {
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).expect("couldnt't read from buffer");

    let request = str::from_utf8(&buffer).expect("couldn't convert to string");
    let request = decode(request).expect("couldn't decode request");

    let doc_root = document_root.clone();

    let (mut status, mut filename) = if request.starts_with(GET) || request.starts_with(HEAD) {
        let slash_idx = request.find("/").unwrap();
        let http_idx = request.find("HTTP").unwrap();
        let mut path = &request[slash_idx + 1..http_idx - 1];
        match path.find("?") {
            Some(idx) => path = &path[..idx],
            None => {},
        }

        document_root.push_str(path);
        (OK_STATUS, document_root)
    } else {
        (NOT_ALLOWED_STATUS, format!("{}", "405.html"))
    };

    if status == NOT_ALLOWED_STATUS {
        let response = format!(
            "{}\r\nServer: Rust Server\r\nDate: {}\r\nConnection: close\r\n\r\n",
            status,
            Utc::now().to_rfc2822(),
        );

        stream.write(response.as_bytes()).expect("couldn't write to client");
        stream.flush().expect("couldn't flush stream");
        return
    }

    let path = std::path::Path::new(&filename);
    if !path.exists() || !path.canonicalize().unwrap().to_str().unwrap().contains(&doc_root) {
        println!("filename {} not found", filename);
        status = NOT_FOUND_STATUS;

        let response = format!(
            "{}\r\nServer: Rust Server\r\nDate: {}\r\nConnection: close\r\n\r\n",
            status,
            Utc::now().to_rfc2822(),
        );

        stream.write(response.as_bytes()).expect("couldn't write to client");
        stream.flush().expect("couldn't flush stream");
        return
    }

    let file = File::open(&filename).expect("couldn't open file");

    if file.metadata().expect("couldn't get metadata").file_type().is_dir() {
        if filename.chars().last().unwrap() == '/' {
            filename.push_str("index.html")
        } else {
            filename.push_str("/index.html")
        }
    }

    let exists = std::path::Path::new(&filename).exists();
    if !exists {
        status = FORBIDDEN_STATUS;
        filename = format!("{}", "403.html")
    }

    let filename_clone = filename.clone();
    let mimetype = get_mimetype(&filename_clone);

    let mut file = File::open(&filename).expect("couldn't open file");

    let mut contents = Vec::new();
    file.read_to_end(&mut contents).expect("couldn't read from file");

    let response = format!(
        "{}\r\nServer: Rust Server\r\nContent-Type: {}\r\nDate: {}\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",
        status,
        mimetype,
        Utc::now().to_rfc2822(),
        contents.len(),
    );

    println!("{}", response);

    stream.write(response.as_bytes()).expect("couldn't write to client");
    if request.starts_with(GET) {
        stream.write(&contents[..]).expect("couldn't write to client");
    }
    stream.flush().expect("couldn't flush stream");
}

fn get_mimetype(filename: &String) -> &str {
    let slice: Vec<&str> = filename.split(".").collect();

    match slice.last() {
        Some(mime) => {
            match *mime {
                "html" => "text/html",
                "css"  => "text/css",
                "js"   => "application/javascript",
                "jpg"  => "image/jpeg",
                "jpeg" => "image/jpeg",
                "png"  => "image/png",
                "gif"  => "image/gif",
                "swf"  => "application/x-shockwave-flash",
                _      => "text/plain",
            }
        }
        None => "text/plain",
    }
}
