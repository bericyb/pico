use log::{error, info};
use mlua::LuaSerdeExt;
use picos::create_pico_service;

const ADMIN_SCRIPT: &str = include_str!("../admin.lua");
const CONFIG_TEMPLATE: &str = include_str!("../templates/config.lua");
const STYLES_TEMPLATE: &str = include_str!("../templates/styles.css");
const AGENTS_TEMPLATE: &str = include_str!("../templates/AGENTS.md");

// SQL Migration Templates
const MIGRATION_PGCRYPTO_TEMPLATE: &str = include_str!("../templates/migration_pgcrypto.sql");
const MIGRATION_USERS_TABLE_TEMPLATE: &str = include_str!("../templates/migration_users_table.sql");
const MIGRATION_PING_COUNTER_TEMPLATE: &str = include_str!("../templates/migration_ping_counter.sql");

// SQL Function Templates
const FUNCTION_AUTHENTICATE_USER_TEMPLATE: &str = include_str!("../templates/function_authenticate_user.sql");
const FUNCTION_REGISTER_USER_TEMPLATE: &str = include_str!("../templates/function_register_user.sql");
const FUNCTION_PONG_TEMPLATE: &str = include_str!("../templates/function_pong.sql");
const FUNCTION_TEMPLATE: &str = include_str!("../templates/function_template.sql");

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

            // Make templates available to the Lua script
            match lua.globals().set("CONFIG_TEMPLATE", CONFIG_TEMPLATE) {
                Ok(_) => {}
                Err(e) => {
                    error!("Failed to load config template: {}", e);
                    panic!("failed to load config template, {}", e);
                }
            }

            match lua.globals().set("STYLES_TEMPLATE", STYLES_TEMPLATE) {
                Ok(_) => {}
                Err(e) => {
                    error!("Failed to load styles template: {}", e);
                    panic!("failed to load styles template, {}", e);
                }
            }

            match lua.globals().set("AGENTS_TEMPLATE", AGENTS_TEMPLATE) {
                Ok(_) => {}
                Err(e) => {
                    error!("Failed to load agents template: {}", e);
                    panic!("failed to load agents template, {}", e);
                }
            }

            // Make SQL migration templates available to the Lua script
            match lua.globals().set("MIGRATION_PGCRYPTO_TEMPLATE", MIGRATION_PGCRYPTO_TEMPLATE) {
                Ok(_) => {}
                Err(e) => {
                    error!("Failed to load pgcrypto migration template: {}", e);
                    panic!("failed to load pgcrypto migration template, {}", e);
                }
            }

            match lua.globals().set("MIGRATION_USERS_TABLE_TEMPLATE", MIGRATION_USERS_TABLE_TEMPLATE) {
                Ok(_) => {}
                Err(e) => {
                    error!("Failed to load users table migration template: {}", e);
                    panic!("failed to load users table migration template, {}", e);
                }
            }

            match lua.globals().set("MIGRATION_PING_COUNTER_TEMPLATE", MIGRATION_PING_COUNTER_TEMPLATE) {
                Ok(_) => {}
                Err(e) => {
                    error!("Failed to load ping counter migration template: {}", e);
                    panic!("failed to load ping counter migration template, {}", e);
                }
            }

            // Make SQL function templates available to the Lua script
            match lua.globals().set("FUNCTION_AUTHENTICATE_USER_TEMPLATE", FUNCTION_AUTHENTICATE_USER_TEMPLATE) {
                Ok(_) => {}
                Err(e) => {
                    error!("Failed to load authenticate user function template: {}", e);
                    panic!("failed to load authenticate user function template, {}", e);
                }
            }

            match lua.globals().set("FUNCTION_REGISTER_USER_TEMPLATE", FUNCTION_REGISTER_USER_TEMPLATE) {
                Ok(_) => {}
                Err(e) => {
                    error!("Failed to load register user function template: {}", e);
                    panic!("failed to load register user function template, {}", e);
                }
            }

            match lua.globals().set("FUNCTION_PONG_TEMPLATE", FUNCTION_PONG_TEMPLATE) {
                Ok(_) => {}
                Err(e) => {
                    error!("Failed to load pong function template: {}", e);
                    panic!("failed to load pong function template, {}", e);
                }
            }

            match lua.globals().set("FUNCTION_TEMPLATE", FUNCTION_TEMPLATE) {
                Ok(_) => {}
                Err(e) => {
                    error!("Failed to load function template: {}", e);
                    panic!("failed to load function template, {}", e);
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
