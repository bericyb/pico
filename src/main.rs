use rico::{init_pico, start_http_server};

fn main() -> std::io::Result<()> {
    println!("Starting pico application...");

    let pico = match init_pico(Some("server.lua".to_string()), None) {
        Ok(service) => service,
        Err(e) => {
            panic!("{}", e);
        }
    };

    return start_http_server(pico);
}
