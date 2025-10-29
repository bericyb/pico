use mlua::LuaSerdeExt;
use rico::create_pico_service;

const ADMIN_SCRIPT: &str = include_str!("../admin.lua");

fn main() -> std::io::Result<()> {
    match std::env::args().nth(1) {
        Some(_) => {
            let args: Vec<String> = std::env::args().skip(1).collect();
            let lua = mlua::Lua::new();
            let lua_args = match lua.to_value(&args) {
                Ok(la) => la,
                Err(e) => panic!("failed to read args, {}", e),
            };

            match lua.globals().set("arg", lua_args) {
                Ok(_) => {}
                Err(e) => panic!("failed to load args, {}", e),
            }

            match lua.load(ADMIN_SCRIPT).exec() {
                Ok(_) => return Ok(()),
                Err(e) => panic!("{}", e),
            }
        }
        None => {}
    }
    println!("Starting pico application...");

    let mut pico = match create_pico_service(Some("config.lua".to_string()), None) {
        Ok(service) => service,
        Err(e) => {
            panic!("{}", e);
        }
    };

    return pico.start_http_server();
}
