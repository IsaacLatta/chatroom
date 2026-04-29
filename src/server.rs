use std::io;
use std::io::{Read, Write};
use std::net;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;

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

    rx_length: usize,
    rx_buffer: [u8; 4096],

    tx_length: usize,
    tx_buffer: [u8; 4096],
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

fn do_read(client: &mut Client) -> io::Result<bool> {
    loop {
        let bytes_read = match client.sock.read(&mut client.rx_buffer[client.rx_length..]) {
            Ok(0) => return Ok(false),
            Ok(n_bytes) => {
                client.rx_length = client.rx_length + n_bytes;
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                break;
            },
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {
                continue;
            },
            Err(e) => return Err(e)
        };
    }

    Ok(true)
}

fn handle_client(client: &mut Client) -> io::Result<bool> {
    if !do_read(client)? {
        return Ok(false)
    }

    Ok(true)
}

fn new_client(sock: net::TcpStream, addr: net::SocketAddr) -> Client {
    return Client { sock, addr, rx_length: 0, rx_buffer: [0u8; 4096], tx_length: 0, tx_buffer: [0u8; 4096] };
}

fn poll_loop(receiver: Receiver<(net::TcpStream, net::SocketAddr)>) -> io::Result<()> {
    let mut clients: Vec<Client> = Vec::new();

    loop {
        while let Ok((sock, addr)) = receiver.try_recv() {
            clients.push(new_client(sock, addr));
        }

        for client in clients.iter_mut() {
            if !do_read(client)? {
                clients.remove(client);
                continue;
            }


        }

        std::thread::yield_now();
    }
}

fn is_fatal(error: io::ErrorKind) -> bool {
    return true;
}