extern crate hello;

use std::fs::File;
use std::io::{Read, Write};
use std::net::*;
use std::time::Duration;

use hello::thread_pool;

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 512];
    stream.read_exact(&mut buffer).unwrap();

    let get = b"GET / HTTP/1.1\r\n";
    let sleep = b"GET /sleep HTTP/1.1\r\n";

    let (status_line, filename) = if buffer.starts_with(get) {
        ("HTTP/1.1 200 OK", "hello.html")
    } else if buffer.starts_with(sleep) {
        std::thread::sleep(Duration::from_secs(5));
        ("HTTP/1.1 200 OK", "hello.html")
    } else {
        ("HTTP/1.1 404 NOT FOUND", "404.html")
    };

    let mut buf = String::new();
    let mut f = File::open(filename).unwrap();
    f.read_to_string(&mut buf).unwrap();

    stream
        .write_all(
            format!(
                "{}\r\nContent-Length: {}\r\n\r\n{}",
                status_line,
                buf.len(),
                buf
            )
            .as_bytes(),
        )
        .unwrap();
    stream.flush().unwrap();
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    let pool = thread_pool::pool::ThreadPool::new(4).unwrap();

    for stream in listener.incoming().take(2) {
        let stream = stream.unwrap();
        pool.execute(|| {
            handle_connection(stream);
        });
    }

    println!("Shutting down...");
}
