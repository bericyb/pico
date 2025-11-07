use log::error;
use mlua::LuaSerdeExt;
use std::{fs::File, io::Read};
use picos::validate_pico_config;

// Admin script and templates
const ADMIN_SCRIPT: &str = include_str!("../../admin.lua");
const CONFIG_TEMPLATE: &str = include_str!("../../templates/config.lua");
const STYLES_TEMPLATE: &str = include_str!("../../templates/styles.css");
const AGENTS_TEMPLATE: &str = include_str!("../../templates/AGENTS.md");
const STYLUA_TEMPLATE: &str = include_str!("../../templates/.stylua.toml");

// SQL Migration Templates
const MIGRATION_PGCRYPTO_TEMPLATE: &str = include_str!("../../templates/migration_pgcrypto.sql");
const MIGRATION_USERS_TABLE_TEMPLATE: &str = include_str!("../../templates/migration_users_table.sql");
const MIGRATION_PING_COUNTER_TEMPLATE: &str = include_str!("../../templates/migration_ping_counter.sql");

// SQL Function Templates
const FUNCTION_AUTHENTICATE_USER_TEMPLATE: &str = include_str!("../../templates/function_authenticate_user.sql");
const FUNCTION_REGISTER_USER_TEMPLATE: &str = include_str!("../../templates/function_register_user.sql");
const FUNCTION_PONG_TEMPLATE: &str = include_str!("../../templates/function_pong.sql");
const FUNCTION_TEMPLATE: &str = include_str!("../../templates/function_template.sql");

/// Helper function to set a global variable in Lua with error handling
fn set_lua_global(lua: &mlua::Lua, name: &str, value: &str) -> Result<(), mlua::Error> {
    lua.globals().set(name, value)
}

/// Validate a Pico configuration file
pub fn validate_config(config_path: Option<String>) -> std::io::Result<()> {
    let pico_config_path = config_path.unwrap_or("config.lua".to_string());
    
    // Read the config file
    let mut pico_config_file = match File::open(&pico_config_path) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Error: Failed to open config file '{}': {}", pico_config_path, e);
            return Err(e);
        }
    };
    
    let mut pico_config = String::new();
    match pico_config_file.read_to_string(&mut pico_config) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error: Failed to read config file '{}': {}", pico_config_path, e);
            return Err(e);
        }
    }

    // Parse the Lua config
    let lua = mlua::Lua::new();
    let pico_config_table = match lua.load(pico_config).eval() {
        Ok(table) => table,
        Err(e) => {
            eprintln!("Error: Failed to parse Lua config file '{}': {}", pico_config_path, e);
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData, 
                format!("Lua parsing error: {}", e)
            ));
        }
    };

    // Validate the config using the lib.rs function
    match validate_pico_config(pico_config_table) {
        Ok((port, db, routes, _route_tree, _crons)) => {
            println!("✅ Configuration validation successful!");
            println!("   Port: {}", port);
            println!("   Database: {}", db);
            println!("   Routes found: {}", routes.len());
            
            // List the routes
            for (route_path, route) in routes.iter() {
                println!("   - {} (methods: {})", 
                    route_path, 
                    route.definitions.keys()
                        .map(|m| m.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
            Ok(())
        }
        Err(validation_error) => {
            eprintln!("❌ Configuration validation failed:");
            eprintln!("   {}", validation_error);
            Err(std::io::Error::new(std::io::ErrorKind::InvalidData, validation_error))
        }
    }
}

/// Run the admin script with the provided command line arguments
pub fn run_admin(args: Vec<String>) -> std::io::Result<()> {
    // Check for validate command first (handle in Rust)
    if args.len() > 0 && (args[0] == "validate" || args[0] == "-v") {
        let config_path = if args.len() > 1 {
            Some(args[1].clone())
        } else {
            None
        };
        return validate_config(config_path);
    }

    let lua = mlua::Lua::new();
    
    // Convert arguments to Lua values
    let lua_args = match lua.to_value(&args) {
        Ok(la) => la,
        Err(e) => {
            error!("Failed to read args: {}", e);
            panic!("failed to read args, {}", e);
        }
    };

    // Set arguments global
    if let Err(e) = lua.globals().set("arg", lua_args) {
        error!("Failed to load args: {}", e);
        panic!("failed to load args, {}", e);
    }

    // Set all template globals
    let templates = [
        ("CONFIG_TEMPLATE", CONFIG_TEMPLATE),
        ("STYLES_TEMPLATE", STYLES_TEMPLATE),
        ("AGENTS_TEMPLATE", AGENTS_TEMPLATE),
        ("STYLUA_TEMPLATE", STYLUA_TEMPLATE),
        ("MIGRATION_PGCRYPTO_TEMPLATE", MIGRATION_PGCRYPTO_TEMPLATE),
        ("MIGRATION_USERS_TABLE_TEMPLATE", MIGRATION_USERS_TABLE_TEMPLATE),
        ("MIGRATION_PING_COUNTER_TEMPLATE", MIGRATION_PING_COUNTER_TEMPLATE),
        ("FUNCTION_AUTHENTICATE_USER_TEMPLATE", FUNCTION_AUTHENTICATE_USER_TEMPLATE),
        ("FUNCTION_REGISTER_USER_TEMPLATE", FUNCTION_REGISTER_USER_TEMPLATE),
        ("FUNCTION_PONG_TEMPLATE", FUNCTION_PONG_TEMPLATE),
        ("FUNCTION_TEMPLATE", FUNCTION_TEMPLATE),
    ];

    for (name, template) in templates.iter() {
        if let Err(e) = set_lua_global(&lua, name, template) {
            error!("Failed to load {} template: {}", name, e);
            panic!("failed to load {} template, {}", name, e);
        }
    }

    // Execute the admin script
    match lua.load(ADMIN_SCRIPT).exec() {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("Admin script execution failed: {}", e);
            panic!("{}", e);
        }
    }
}