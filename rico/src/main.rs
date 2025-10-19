use rico::{create_pico_function, create_pico_migration, create_pico_service};

fn main() -> std::io::Result<()> {
    match std::env::args().nth(1) {
        Some(arg) => match arg.as_str() {
            "version" | "-v" | "v" => {
                println!("Pico version {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "init" | "initialize" | "i" => {}
            "migrate" | "m" => {}
            "function" | "f" => {}
            "help" | "--help" | "h" | "-h" | _ => {
                println!(
                    "Pico\n\tversion,  v: Current Pico Version\n\tinit,     i: Create a new Pico application\n\tmigrate,  m: Create a database migration\n\tfunction, f: Create a SQL function"
                );
                return Ok(());
            }
        },
        None => {}
    }
    println!("Starting pico application...");

    let mut pico = match create_pico_service(Some("server.lua".to_string()), None) {
        Ok(service) => service,
        Err(e) => {
            panic!("{}", e);
        }
    };

    return pico.start_http_server();
}
