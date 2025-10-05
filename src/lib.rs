pub mod cron;
pub mod html;
pub mod http;
pub mod route;
pub mod sql;
use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    net::TcpListener,
};

use mlua::{Lua, Table};
use serde_json::{Value, to_string};

use crate::{
    cron::cron::Crons,
    html::html::View,
    http::http::{Body, ResponseCode, handle_stream},
    route::route::{Method, Route, RouteHandler},
    sql::sql::{SQL, initialize_sql_service},
};

pub struct PicoService {
    lua: Lua,
    sql: SQL,
    db: String,
    routes: HashMap<String, Route>,
    route_tree: RouteTree,
    crons: Option<Crons>,
}

#[derive(Clone)]
pub struct RouteTree {
    nodes: HashMap<String, RouteTree>,
    parameter_name: String,
}

pub struct PicoRequest {
    pub method: Method,
    pub path: String,
    pub query: HashMap<String, String>,
    pub version: u8,
    pub headers: HashMap<String, Vec<u8>>,
    pub body: Body,
}

/// Initializes pico using the config and environment variables
/// found at the provided file.
//
/// If no path is provided then the current working dir is searched
/// for pico.lua and *.env
pub fn create_pico_service(
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

    let (db, routes, route_tree, crons) = validate_pico_config(pico_config_table)?;

    let sql = match initialize_sql_service(&db) {
        Ok(sql) => sql,
        Err(e) => return Err(format!("error initializing sql database: {}", e)),
    };

    return Ok(PicoService {
        lua,
        sql,
        db,
        routes,
        route_tree,
        crons,
    });
}

impl PicoService {
    pub fn start_http_server(&mut self) -> std::io::Result<()> {
        // For now let's just bind on 8080.
        // TODO: Get port from pico config
        let listener = TcpListener::bind("127.0.0.1:8080")?;

        println!("Pico server listening on {}", listener.local_addr()?);

        for stream in listener.incoming() {
            let mut s = match stream {
                Err(e) => {
                    println!("error accepting incoming TcpStream: {}", e);
                    continue;
                }
                Ok(s) => s,
            };
            match handle_stream(&mut s) {
                Ok(pr) => match self.handle_http_pico_request(pr) {
                    Ok(response_bytes) => {
                        // TODO: implement failed write retry logic
                        let _nbw = s.write(&response_bytes).unwrap();
                    }
                    Err(rc) => {
                        // TODO: implement failed write retry logic and abstract to write response
                        // code
                        let _nbw = s.write(&rc.to_bytes()).unwrap();
                    }
                },
                Err(rc) => {
                    // TODO: implement failed write retry logic and abstract to write response
                    // code
                    let _nbw = s.write(&rc.to_bytes()).unwrap();
                }
            }
        }
        return Ok(());
    }

    pub fn handle_http_pico_request(
        &mut self,
        request: PicoRequest,
    ) -> Result<Vec<u8>, ResponseCode> {
        println!(
            "Received request: {} {}",
            request.method,
            request.path.as_str()
        );

        let mut tree = self.route_tree.clone();

        let mut pico_route_path = String::new();
        let mut route_parameters: HashMap<String, String> = HashMap::new();
        for seg in request.path.split("/") {
            match tree.nodes.get(&seg.to_string()) {
                Some(subtree) => {
                    pico_route_path = pico_route_path + &subtree.parameter_name;
                    tree = subtree.clone();
                }
                None => match tree.nodes.get(&"*".to_string()) {
                    Some(subtree) => {
                        route_parameters.insert(subtree.parameter_name.clone(), seg.to_string());
                        pico_route_path = pico_route_path + &subtree.parameter_name;
                        tree = subtree.clone();
                    }
                    None => return Err(ResponseCode::NotFound),
                },
            }
        }

        let pico_route: &Route = match self.routes.get(&pico_route_path) {
            Some(r) => r,
            None => return Err(ResponseCode::NotFound),
        };

        let route_handler = match pico_route.definitions.get(&request.method) {
            Some(rh) => rh,
            None => return Err(ResponseCode::NotFound),
        };

        match &route_handler.sproc_name {
            Some(sproc_name) => {
                let sproc = match self.sql.sprocs.get(&sproc_name.clone()) {
                    Some(s) => s,
                    None => return Err(ResponseCode::InternalError),
                };
                let mut sproc_input: HashMap<String, Value> = HashMap::new();
                match request.body {
                    Body::Json(j_body) => {
                        for param in sproc.parameters.clone() {
                            let val = match j_body.get(&param.clone()) {
                                Some(b_val) => b_val,
                                None => {
                                    match route_parameters.get(&param.clone()) {
                                        Some(rp_val) => &Value::String(rp_val.to_string()),
                                        // TODO: add required parameter missing code
                                        None => return Err(ResponseCode::BadRequest),
                                    }
                                }
                            };
                            sproc_input.insert(param, val.clone());
                        }
                    }
                    Body::QueryParams(hash_map) => {
                        for param in sproc.parameters.clone() {
                            let val = match hash_map.get(&param.clone()) {
                                Some(qp) => qp,
                                None => {
                                    match route_parameters.get(&param.clone()) {
                                        Some(rp_val) => &rp_val.to_string(),
                                        // TODO: add required parameter missing code
                                        None => return Err(ResponseCode::BadRequest),
                                    }
                                }
                            };
                            sproc_input.insert(param, Value::String(val.clone()));
                        }
                    }
                    Body::Raw(_items) => {
                        println!("Gotta figure out raw sql input");
                        todo!();
                    }
                }

                match sproc.execute(&mut self.sql.connection, sproc_input) {
                    Ok(value) => match to_string(&value) {
                        Ok(js) => return Ok(js.into_bytes()),
                        Err(e) => {
                            println!("error converting json value to string: {}", e);
                            return Err(ResponseCode::InternalError);
                        }
                    },
                    Err(rc) => return Err(rc),
                }
            }
            None => (),
        }

        Ok(vec![])
    }
}

// Validate and serialize fields from pico configurations
pub fn validate_pico_config(
    config: mlua::Table,
) -> Result<(String, HashMap<String, Route>, RouteTree, Option<Crons>), String> {
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

    let routes: HashMap<String, Route> = HashMap::new();
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

        let mut definitions: HashMap<Method, RouteHandler> = HashMap::new();
        for handler_def in handlers.pairs::<Method, Table>() {
            let (method, handler) = match handler_def {
                Ok(handler_def) => handler_def,
                Err(e) => {
                    return Err(format!(
                        "invalid pico config: Route {} is not defined with a method String, Table key value pair {}",
                        path, e
                    ));
                }
            };

            let view: Option<View> = match handler.get("VIEW") {
                Ok(v) => v,
                Err(e) => {
                    return Err(format!(
                        "invalid pico config: Route {}: {} has VIEW but is not properly shaped {}",
                        path, method, e
                    ));
                }
            };

            let sql: Option<String> = match handler.get("SQL") {
                Ok(v) => v,
                Err(e) => {
                    return Err(format!(
                        "invalid pico config: Route {}: {} has SQL  but is not a string {}",
                        path, method, e
                    ));
                }
            };

            let set_jwt: Option<mlua::Function> = match handler.get("SETJWT") {
                Ok(v) => v,
                Err(e) => {
                    return Err(format!(
                        "invalid pico config: Route {}: {} has SETJWT but is not a function {}",
                        path, method, e
                    ));
                }
            };

            let transform: Option<mlua::Function> = match handler.get("TRANSFORM") {
                Ok(v) => v,
                Err(e) => {
                    return Err(format!(
                        "invalid pico config: Route {}: {} has TRANSFORM but is not a function {}",
                        path, method, e
                    ));
                }
            };

            definitions.insert(
                method,
                RouteHandler {
                    view,
                    sproc_name: sql,
                    set_jwt,
                    transform,
                },
            );
        }
    }

    let mut route_tree = RouteTree {
        nodes: HashMap::new(),
        parameter_name: "".to_string(),
    };

    // Create route tree
    for (route, _) in &routes {
        for seg in route.split("/") {
            // TODO: is this correct and needs an underscore for lint?
            let mut _current = &mut route_tree;
            if seg.is_empty() {
                continue;
            }

            // Add a wildcard if parameterized
            if seg.chars().nth(0).unwrap().to_string() == ":" {
                _current = _current.nodes.entry("*".to_string()).or_insert(RouteTree {
                    nodes: HashMap::new(),
                    parameter_name: seg.to_string(),
                });
            } else {
                _current = _current.nodes.entry(seg.to_string()).or_insert(RouteTree {
                    nodes: HashMap::new(),
                    parameter_name: seg.to_string(),
                });
            }
        }
    }
    // let crons: Option<Crons> = match config.get("CRONS") {
    //     Ok(c) => c,
    //     Err(e) => return Err(format!("invalid pico config: CRONS field not found. {}", e)),
    // };
    //

    return Ok((db, routes, route_tree, None));
}
