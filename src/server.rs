use std::io;
use std::io::{Read, Write};
use std::net;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;

const BUFFER_SIZE: usize = 6555536;

pub struct Config {
    pub bind_addr: String,
    pub port: u16,
}

pub fn try_parse_cli_args(mut args: impl Iterator<Item = String>) -> Result<Config, String> {
    let usage_str = String::from("Usage: chatroom <bind-address> <port>");

    let _ = args.next();
    let bind_addr = match args.next() {
        Some(val) => val,
        None => return Err(usage_str.to_string())
    };

    let port_str = match args.next() {
        Some(val) => val,
        None => return Err(usage_str.to_string())
    };

    let port = match port_str.parse::<u16>() {
        Ok(val) => val,
        Err(_) => return Err(usage_str.to_string())
    };

    Ok(Config { bind_addr, port })
}

pub fn run_server(config: &Config) -> io::Result<()> {
    let host_addr = format!("{}:{}", config.bind_addr, config.port);
    let listener = net::TcpListener::bind(host_addr)?;

    let (transmitter, receiver) = mpsc::channel::<(net::TcpStream, net::SocketAddr)>();

    let accept_thread = std::thread::spawn(move || {
        return accept_loop(listener, move |stream, addr| {
             match transmitter.send((stream, addr)) {
                 Ok(_) => {},
                 Err(e) => {
                    eprintln!("Failed to send accepted client: {}", e);
                 }
             }
        });
    });

    let poller_thread = std::thread::spawn(move || {
         return poll_loop(receiver);
    });

    accept_thread.join().unwrap()?;
    poller_thread.join().unwrap()?;
    Ok(())
}

struct Client {
    sock: net::TcpStream,
    addr: net::SocketAddr,

    rx_buffer: Vec<u8>,
    tx_buffer: Vec<u8>,

    is_connected: bool,
}

fn accept_loop<AcceptCallback>(listener: net::TcpListener, mut on_accept: AcceptCallback) -> io::Result<()>
where
    AcceptCallback: FnMut(net::TcpStream, net::SocketAddr) -> ()
{
    loop {
        let (sock, addr) = match listener.accept() {
            Ok((sock, addr)) => (sock, addr),
            Err(e) if is_fatal(e.kind()) => return Err(e),
            Err(e) => {
                eprintln!("accept error: {}", e);
                continue;
            }
        };
        on_accept(sock, addr);
    }
}

fn read_into(sock: &mut net::TcpStream, buffer: &mut Vec<u8>, scratch_buffer: &mut [u8; BUFFER_SIZE]) -> io::Result<()> {
    loop {
        match sock.read(scratch_buffer) {
            Ok(0) => return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "connection closed")),
            Ok(n_bytes_read) => {
                buffer.extend_from_slice(&scratch_buffer[..n_bytes_read]);
            },
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                return Ok(());
            },
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {
                continue;
            },
            Err(e) => return Err(e)        }
    }
}


fn write_to(sock: &mut net::TcpStream, src_buffer: &mut Vec<u8>) -> io::Result<()> {
    while !src_buffer.is_empty() {
        match sock.write(src_buffer) {
            Ok(0) => return Ok(()),
            Ok(n_bytes_written) => {
                src_buffer.drain(..n_bytes_written);
            },
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(()),
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e)
        }
    }
    Ok(())
}

fn message_ready(buffer: & Vec<u8>) -> bool {
    return true;
}

fn service_client(client: &mut Client, buffer: &mut [u8; BUFFER_SIZE]) -> io::Result<()> {
    read_into(&mut client.sock, &mut client.rx_buffer, buffer)?;
    write_to(&mut client.sock, &mut client.tx_buffer)?;
    Ok(())
}

fn new_client(sock: net::TcpStream, addr: net::SocketAddr) -> Client {
    Client { sock, addr, rx_buffer: Vec::new(), tx_buffer: Vec::new(), is_connected: true }
}

fn poll_loop<MessageCallback>(receiver: Receiver<(net::TcpStream, net::SocketAddr)>, mut on_message_ready: MessageCallback) -> io::Result<()>
where
    MessageCallback: FnMut(&mut Client) -> io::Result<()>
{
    let mut clients: Vec<Client> = Vec::new();
    let mut buffer = [0u8; BUFFER_SIZE];

    loop {
        while let Ok((sock, addr)) = receiver.try_recv() {
            clients.push(new_client(sock, addr));
        }

        for client in clients.iter_mut() {
            match service_client(client, &mut buffer) {
                Ok(()) => {},
                Err(e) => {
                    client.is_connected = false;
                }
            }
        }

        clients.retain(|client| client.is_connected);

        for client in clients.iter_mut() {
            if message_ready(&client.rx_buffer) {
                match on_message_ready(client) {
                    Ok(()) => {},
                    Err(e) => {
                        client.is_connected = false;
                    }
                }
            }
        }

        clients.retain(|client| client.is_connected);

        std::thread::yield_now();
    }
}

fn is_fatal(error: io::ErrorKind) -> bool {
    return true;
}
