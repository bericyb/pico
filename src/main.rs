use rico::{create_pico_function, create_pico_migration, create_pico_service};

fn main() -> std::io::Result<()> {
    match std::env::args().nth(1) {
        Some(arg) => match arg.as_str() {
            "--version" | "-v" => {
                println!("Pico version {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "migrate" | "m" => {
                create_pico_migration();
                return Ok(());
            }
            "function" | "f" => {
                create_pico_function();
                return Ok(());
            }
            _ => {}
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
