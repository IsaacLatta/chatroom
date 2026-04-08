use std::io;
use std::io::{Read, Write};
use std::net;

pub struct Config {
    pub bind_addr: String,
    pub port: u16,
}

fn handle_client(mut stream: net::TcpStream) -> io::Result<()> {
    let mut buffer = [0u8; 4096];

    loop {
        let bytes_read = stream.read(&mut buffer)?;
        if bytes_read == 0 {
            println!("Client disconnected");
            break;
        }

        let message = String::from_utf8_lossy(&buffer[..bytes_read]);

        println!("Received: {}", message);
        stream.write_all(&buffer[..bytes_read])?;
    }

    Ok(())
}

pub fn run_server(config: &Config) -> io::Result<()> {
    let listener = net::TcpListener::bind(format!("{}:{}", config.bind_addr, config.port))?;

    for stream in listener.incoming() {
        handle_client(stream?)?;
    }

    Ok(())
}