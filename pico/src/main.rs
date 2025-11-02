use log::{error, info};
use picos::create_pico_service;

mod admin;

fn main() -> std::io::Result<()> {
    // Initialize the logger
    env_logger::init();

    // Check if we have command line arguments (admin mode)
    if let Some(_) = std::env::args().nth(1) {
        let args: Vec<String> = std::env::args().skip(1).collect();
        return admin::run_admin(args);
    }

    // No arguments, start the HTTP server
    info!("Starting pico application...");

    let mut pico = match create_pico_service(Some("config.lua".to_string()), None) {
        Ok(service) => service,
        Err(e) => {
            error!("Failed to create pico service: {}", e);
            panic!("{}", e);
        }
    };

    return pico.start_http_server();
}
