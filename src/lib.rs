pub mod cron;
pub mod html;
pub mod http;
pub mod route;
pub mod sql;
use std::{collections::HashMap, fs::File, io::Read, net::TcpListener};

use mlua::{Lua, LuaSerdeExt, Table};

use crate::{
    cron::cron::Crons,
    html::html::View,
    http::http::{Body, handle_stream},
    route::route::{Method, Route, RouteHandler},
    sql::sql::SQL,
};

#[derive(Debug)]
pub struct PicoService {
    lua: Lua,
    sql: SQL,
    db: String,
    routes: HashMap<String, Route>,
    crons: Crons,
}

struct PicoRequest {
    pub method: String,
    pub path: String,
    pub version: u8,
    pub headers: HashMap<String, Vec<u8>>,
    pub body: Body,
}

/// Initializes pico using the config and environment variables
/// found at the provided file.
///
/// If no path is provided then the current working dir is searched
/// for pico.lua and *.env
pub fn init_pico(
    config_path: Option<String>,
    _env_file_path: Option<String>,
) -> Result<PicoService, String> {
    let pico_config_path = config_path.unwrap_or("pico.lua".to_string());
    let mut pico_config_file = match File::open(pico_config_path.clone()) {
        Ok(file) => file,
        Err(e) => {
            return Err(format!(
                "failed to open pico config {} error: {}",
                pico_config_path.clone(),
                e
            ));
        }
    };
    let mut pico_config = String::new();

    match pico_config_file.read_to_string(&mut pico_config) {
        Ok(_) => {}
        Err(e) => {
            return Err(format!(
                "failed to read pico config {} error: {}",
                pico_config_path.clone(),
                e
            ));
        }
    }

    let lua = Lua::new();
    let pico_config_table = match lua.load(pico_config).eval() {
        Ok(table) => table,
        Err(e) => {
            return Err(format!(
                "error reading pico config {} error: {}",
                pico_config_path.clone(),
                e
            ));
        }
    };

    let (db, routes, crons) = validate_pico_config(pico_config_table)?;

    return Ok(PicoService {
        lua,
        sql: todo!(),
        db,
        routes,
        crons,
    });
}

pub fn start_http_server(pico: PicoService) -> std::io::Result<()> {
    // For now let's just bind on 8080.
    // TODO: Get port from pico config
    let listener = TcpListener::bind("127.0.0.1:8080")?;

    for stream in listener.incoming() {
        handle_stream(stream?)
    }

    return Ok(());
}

// Validate and serialize fields from pico configurations
pub fn validate_pico_config(
    config: mlua::Table,
) -> Result<(String, HashMap<String, Route>, Crons), String> {
    let db: String;
    match config.get("DB") {
        Ok(l_db) => {
            db = l_db;
        }
        Err(e) => {
            return Err(format!(
                "invalid pico config: DB field is not a string. {}",
                e
            ));
        }
    };

    let routes: HashMap<String, Route>;
    let routes_table: Table;
    match config.get("ROUTES") {
        Ok(l_routes) => {
            routes_table = l_routes;
        }
        Err(e) => {
            return Err(format!(
                "invalid pico config: ROUTES field is not a table. {}",
                e
            ));
        }
    };

    for route in routes_table.pairs::<String, Table>() {
        let (path, handlers) = match route {
            Ok(route) => route,
            Err(e) => {
                return Err(format!(
                    "invalid pico config: ROUTES is not a table with String, Table key value pairs. {}",
                    e
                ));
            }
        };

        let definitions: HashMap<Method, RouteHandler>;
        for handler_def in handlers.pairs::<String, Table>() {
            let (method, handler) = match handler_def {
                Ok(handler_def) => handler_def,
                Err(e) => {
                    return Err(format!(
                        "invalid pico config: Route {} is not defined with a method String, Table key value pair {}",
                        path, e
                    ));
                }
            };

            let pico_method = match method.as_str() {
                "GET" => Method::GET,
                "PUT" => Method::PUT,
                "POST" => Method::POST,
                "DELETE" => Method::DELETE,
                "WS" => Method::WS,
                "SSE" => Method::SSE,
                m => {
                    return Err(format!(
                        "invalid pico config: Route {} is defined with an unknown method {}",
                        path, m
                    ));
                }
            };

            let view: Option<View> = match handler.get("VIEW") {
                Ok(v) => v,
                Err(e) => {
                    return Err(format!(
                        "invalid pico config: Route {}: {} has VIEW but is not properly shaped {}",
                        path, pico_method, e
                    ));
                }
            };

            let sql: Option<String> = match handler.get("SQL") {
                Ok(v) => v,
                Err(e) => {
                    return Err(format!(
                        "invalid pico config: Route {}: {} has SQL  but is not a string {}",
                        path, pico_method, e
                    ));
                }
            };

            let set_jwt: Option<mlua::Function> = match handler.get("SETJWT") {
                Ok(v) => v,
                Err(e) => {
                    return Err(format!(
                        "invalid pico config: Route {}: {} has SETJWT but is not a fucntion {}",
                        path, pico_method, e
                    ));
                }
            };

            let transform: Option<mlua::Function> = match handler.get("TRANSFORM") {
                Ok(v) => v,
                Err(e) => {
                    return Err(format!(
                        "invalid pico config: Route {}: {} has TRANSFORM but is not a function {}",
                        path, pico_method, e
                    ));
                }
            };

            RouteHandler {
                view,
                sql,
                set_jwt,
                transform,
            };
        }
    }

    let crons: Crons = match config.get("CRONS") {
        Ok(c) => c,
        Err(e) => return Err(format!("invalid pico config: CRONS field not found. {}", e)),
    };

    return Ok((db, routes, crons));
}
