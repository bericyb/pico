use rico::create_pico_service;

fn main() -> std::io::Result<()> {
    println!("Starting pico application...");

    let mut pico = match create_pico_service(Some("server.lua".to_string()), None) {
        Ok(service) => service,
        Err(e) => {
            panic!("{}", e);
        }
    };

    return pico.start_http_server();
}
