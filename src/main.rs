mod server;

fn main() -> std::io::Result<()> {
    let config = match server::try_parse_cli_args(std::env::args()) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    println!("Running server on {}:{} ...", config.bind_addr, config.port);
    server::run_server(&config)
}
