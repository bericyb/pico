use log::{error, info};
use mlua::LuaSerdeExt;
use picos::create_pico_service;

const ADMIN_SCRIPT: &str = include_str!("../admin.lua");

fn main() -> std::io::Result<()> {
    // Initialize the logger
    env_logger::init();

    match std::env::args().nth(1) {
        Some(_) => {
            let args: Vec<String> = std::env::args().skip(1).collect();
            let lua = mlua::Lua::new();
            let lua_args = match lua.to_value(&args) {
                Ok(la) => la,
                Err(e) => {
                    error!("Failed to read args: {}", e);
                    panic!("failed to read args, {}", e);
                }
            };

            match lua.globals().set("arg", lua_args) {
                Ok(_) => {}
                Err(e) => {
                    error!("Failed to load args: {}", e);
                    panic!("failed to load args, {}", e);
                }
            }

            match lua.load(ADMIN_SCRIPT).exec() {
                Ok(_) => return Ok(()),
                Err(e) => {
                    error!("Admin script execution failed: {}", e);
                    panic!("{}", e);
                }
            }
        }
        None => {}
    }
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
