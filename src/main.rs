use std::io;
use std::io::Write;
use std::process::exit;
mod server;

fn main() -> io::Result<()> {
    let mut bind_addr = String::new();
    let mut port_as_str = String::new();

    print!("Enter server IP>");
    io::stdout().flush().unwrap();
    let ip: String = match io::stdin().read_line(&mut bind_addr) {
        Ok(bytes) => {
            let ip = bind_addr.trim().to_string();
            println!("Got IP \"{}\" ({}) bytes", ip, bytes);
            ip
        }
        Err(e) => {
            println!("Failed to read input: {}", e);
            exit(1);
        }
    };

    print!("Enter server port>");
    io::stdout().flush().unwrap();
    match io::stdin().read_line(&mut port_as_str) {
        Ok(_) => {}
        Err(e) => {
            println!("Failed to read port: {}", e);
            exit(1);
        }
    }

    let port: u16 = match port_as_str.trim().parse() {
        Ok(p) => p,
        Err(e) => {
            println!("Failed to convert port: {}", e);
            exit(1);
        }
    };

    println!("Running server on {}:{} ...", ip, port);

    let config = server::Config { bind_addr: ip, port: port };
    server::run_server(&config)
}
