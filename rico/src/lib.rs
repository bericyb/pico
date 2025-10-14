pub mod cron;
pub mod html;
pub mod http;
pub mod route;
pub mod sql;
use std::{
    collections::HashMap,
    fs::File,
    io::{self, Read, Write},
    net::TcpListener,
};

use chrono::Utc;
use mlua::{Lua, Table};
use serde_json::Value;

use crate::{
    cron::cron::Crons,
    html::html::View,
    http::http::{Body, ResponseCode, handle_stream},
    route::route::{Method, Route, RouteHandler},
    sql::sql::{SQL, SQL_FUNCTION_TEMPLATE, initialize_sql_service},
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

impl RouteTree {
    pub fn to_string(&self) -> String {
        let mut res = String::new();

        for node in self.nodes.clone() {
            res = res + &node.0 + ":\n\t" + &(node.1.clone()).to_string();
        }
        return res;
    }
}

pub struct PicoRequest {
    pub method: Method,
    pub path: String,
    pub query: HashMap<String, String>,
    pub version: String,
    pub headers: HashMap<String, Vec<String>>,
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

    let (db, routes, route_tree, crons) = match validate_pico_config(pico_config_table) {
        Ok(r) => r,
        Err(es) => return Err(format!("error validating pico config: {}", es)),
    };

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

pub fn create_pico_migration() {
    print!("Migration name:");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    input = input.replace(" ", "_");
    let input = input.trim();

    if input == "" {
        println!("Migration name required");
        return;
    }

    let now = Utc::now().timestamp();

    let file_name = format!("db/migrations/{}:{}.sql", now, input);

    let _file = match File::create(&file_name) {
        Ok(f) => f,
        Err(e) => {
            println!("migration creation failed {}", e);
            return;
        }
    };

    println!("Migration file {} created.", &file_name);
    return;
}

pub fn create_pico_function() {
    print!("SQL function name:");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    input = input.replace(" ", "_");
    let input = input.trim();

    if input == "" {
        println!("Function name required");
        return;
    }

    let file_path = format!("db/functions/{}.sql", input);

    let mut file = match File::create_new(file_path) {
        Ok(f) => f,
        Err(e) => {
            println!("function creation failed: {}", e);
            return;
        }
    };

    match file.write(SQL_FUNCTION_TEMPLATE.replace("{name}", input).as_bytes()) {
        Ok(_) => {}
        Err(e) => {
            println!("function creation failed: {}", e);
            return;
        }
    }

    println!("Function file {} created.", &input);
    return;
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

        let mut tree = &self.route_tree;

        let mut pico_route_path = String::new();
        let mut route_parameters: HashMap<String, String> = HashMap::new();
        for seg in request.path.split("/") {
            if seg == "" {
                continue;
            }
            println!("working on seg: {}", seg);
            match tree.nodes.get(&seg.to_string()) {
                Some(subtree) => {
                    println!("found match!");
                    pico_route_path = pico_route_path + &subtree.parameter_name;
                    tree = &subtree;
                }
                None => match tree.nodes.get(&"*".to_string()) {
                    Some(subtree) => {
                        println!("Wildcard match found");
                        route_parameters.insert(subtree.parameter_name.clone(), seg.to_string());
                        pico_route_path = pico_route_path + &subtree.parameter_name;
                        tree = &subtree;
                    }
                    None => {
                        println!("no route match found, even with wildcard");
                        return Err(ResponseCode::NotFound);
                    }
                },
            }
        }

        println!("pico_route_path: {}", pico_route_path);

        let pico_route: &Route = match self.routes.get(&pico_route_path) {
            Some(r) => r,
            None => {
                println!("no route handlers for {} found", pico_route_path);
                return Err(ResponseCode::NotFound);
            }
        };

        let route_handler = match pico_route.definitions.get(&request.method) {
            Some(rh) => rh,
            None => {
                println!(
                    "no route handler for {} found with method {}",
                    pico_route_path,
                    request.method.to_string()
                );
                return Err(ResponseCode::NotFound);
            }
        };

        let json_body = match &route_handler.function_name {
            Some(file_name) => {
                let function_name = file_name.strip_suffix(".sql").unwrap_or(file_name);
                let function = match self.sql.functions.get(function_name) {
                    Some(s) => s,
                    None => {
                        println!(
                            "internal error getting sql function {} for route {}",
                            function_name, pico_route_path,
                        );
                        return Err(ResponseCode::InternalError);
                    }
                };
                let mut function_input: HashMap<String, Value> = HashMap::new();
                match request.body {
                    Body::Json(j_body) => {
                        for param in function.parameters.clone() {
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
                            function_input.insert(param, val.clone());
                        }
                    }
                    Body::QueryParams(hash_map) => {
                        for param in function.parameters.clone() {
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
                            function_input.insert(param, Value::String(val.clone()));
                        }
                    }
                    Body::Raw(_items) => {
                        println!("Gotta figure out raw input into sql...");
                        todo!();
                    }
                }

                match function.execute(&mut self.sql.connection, function_input) {
                    Ok(value) => value,
                    Err(rc) => return Err(rc),
                }
            }
            None => {
                println!("no sql function found for {}", pico_route_path);
                Value::Null
            }
        };

        // TODO: TRANSFORM
        // TODO: SETJWT
        // TODO: POLICY
        // TODO: VIEW

        if request.headers.get("accept").is_some() {
            let accept_headers = request.headers.get("accept").unwrap_or(&vec![]);
            if accept_headers.get(0).unwrap_or(&"".to_string()) == (&"text/html".to_string()) {
                if let Some(view) = &route_handler.view {
                    let html = view.to_html(&json_body);
                    return self.create_html_response(html);
                }
            }
        }

        let mut headers: HashMap<String, Vec<String>> = HashMap::new();
        let binding = json_body.to_string();
        let body_bytes = binding.as_bytes();
        headers.insert(
            "Content-Length".to_string(),
            vec![format!("{}", body_bytes.len())],
        );
        headers.insert(
            "Content-Type".to_string(),
            vec!["application/json".to_string()],
        );

        let mut resp: String = "HTTP/1.1 200 OK\r\n".to_string();
        for (k, vs) in headers {
            resp = resp + &k + ": ";
            for (i, v) in vs.clone().into_iter().enumerate() {
                if i > 1 && i < vs.len() - 1 {
                    resp = resp + "; ";
                }
                resp = resp + &v;
            }
            resp = resp + "\r\n";
        }

        resp = resp + "\r\n";
        Ok((resp + &binding).as_bytes().to_vec())
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

    let mut routes: HashMap<String, Route> = HashMap::new();
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

    println!("routes table: {:#?}", routes_table);

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
                    function_name: sql,
                    set_jwt,
                    transform,
                },
            );
        }
        routes.insert(path, Route { definitions });
    }

    let mut route_tree = RouteTree {
        nodes: HashMap::new(),
        parameter_name: "".to_string(),
    };

    // Create route tree
    for (route, _) in &routes {
        println!("Creating route {}", route);
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
