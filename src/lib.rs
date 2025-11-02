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
    path::Path,
};

use chrono::Utc;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use log::{debug, error, info, warn};
use mlua::{Lua, LuaSerdeExt, Table};
use percent_encoding::percent_decode_str;
use serde_json::Value;

use crate::{
    cron::cron::Crons,
    html::html::View,
    http::http::{Body, PicoResponse, ResponseCode, handle_stream},
    route::route::{Method, Route, RouteHandler},
    sql::sql::{SQL, SQL_FUNCTION_TEMPLATE, initialize_sql_service},
};

/// Extracts JWT claims from pico_jwt cookie in request headers
fn extract_jwt_claims(headers: &HashMap<String, Vec<String>>, secret_key: &str) -> Option<Value> {
    debug!("=== JWT EXTRACTION DEBUG ===");
    debug!("All headers: {:#?}", headers);

    let cookie_headers = headers.get("cookie");
    debug!("Cookie headers found: {:?}", cookie_headers.is_some());

    let cookie_headers = match cookie_headers {
        Some(headers) => {
            debug!("Cookie headers content: {:#?}", headers);
            headers
        }
        None => {
            debug!("No 'cookie' header found in request");
            return None;
        }
    };

    for (i, cookie_header) in cookie_headers.iter().enumerate() {
        debug!("Processing cookie header {}: '{}'", i, cookie_header);

        for (j, cookie) in cookie_header.split(';').enumerate() {
            let cookie = cookie.trim();
            debug!("  Cookie {}: '{}'", j, cookie);

            if cookie.starts_with("pico_jwt=") {
                let jwt_token = &cookie[9..]; // Remove "pico_jwt=" prefix
                debug!("Found pico_jwt cookie with token: '{}'", jwt_token);

                // Create validation - use default HS256 algorithm, but don't require exp claim
                let mut validation = Validation::new(Algorithm::HS256);
                validation.required_spec_claims.remove("exp");

                match decode::<Value>(
                    jwt_token,
                    &DecodingKey::from_secret(secret_key.as_ref()),
                    &validation,
                ) {
                    Ok(token_data) => {
                        debug!(
                            "JWT successfully decoded with claims: {:#?}",
                            token_data.claims
                        );
                        return Some(token_data.claims);
                    }
                    Err(e) => {
                        debug!("JWT decode failed: {}", e);
                        continue; // Invalid JWT, try next cookie
                    }
                }
            }
        }
    }

    debug!("No valid pico_jwt cookie found");
    None
}

/// Helper function to call a Lua function with flexible arity (1 or 2 parameters)
fn call_lua_function_with_optional_jwt(
    function: &mlua::Function,
    data: mlua::Value,
    jwt: mlua::Value,
) -> mlua::Result<mlua::Value> {
    // First try calling with 2 parameters (data, jwt)
    match function.call((data.clone(), jwt)) {
        Ok(result) => Ok(result),
        Err(e) => {
            // If it fails, check if it's an arity error and try with 1 parameter
            let error_msg = e.to_string();
            if error_msg.contains("wrong number of arguments")
                || error_msg.contains("attempt to call")
            {
                // Try calling with just the data parameter for backward compatibility
                function.call(data)
            } else {
                // Other error, propagate it
                Err(e)
            }
        }
    }
}

/// Extracts clean error message from Lua errors, especially user-triggered error() calls
fn extract_lua_error_message(error: &mlua::Error) -> String {
    match error {
        mlua::Error::RuntimeError(msg) => {
            // Extract just the user error message, not full stack trace
            if let Some(user_msg) = msg.split("stack traceback:").next() {
                // Remove "error: " prefix if present and clean up
                let cleaned = user_msg.trim();
                if cleaned.starts_with("error: ") {
                    cleaned[7..].trim().to_string()
                } else {
                    cleaned.to_string()
                }
            } else {
                msg.clone()
            }
        }
        _ => error.to_string(),
    }
}

/// Determines if a Lua error should be treated as a user error (vs system error)
fn is_user_lua_error(error: &mlua::Error) -> bool {
    match error {
        mlua::Error::RuntimeError(msg) => {
            // User called error() function
            msg.contains("error: ") || msg.starts_with("error: ")
        }
        _ => false,
    }
}

/// Returns the MIME type for a file based on its extension
fn get_mime_type(file_path: &str) -> &'static str {
    match Path::new(file_path).extension().and_then(|s| s.to_str()) {
        Some("html") => "text/html",
        Some("css") => "text/css",
        Some("js") => "application/javascript",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("json") => "application/json",
        Some("txt") => "text/plain",
        Some("ico") => "image/x-icon",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        Some("otf") => "font/otf",
        _ => "application/octet-stream",
    }
}

/// Attempts to serve a static file from the public directory
fn try_serve_static_file(request_path: &str) -> Result<Vec<u8>, ResponseCode> {
    // URL decode the request path to handle special characters like spaces
    let decoded_path = match percent_decode_str(request_path).decode_utf8() {
        Ok(decoded) => decoded.to_string(),
        Err(_) => {
            debug!("Failed to decode URL path: {}", request_path);
            return Err(ResponseCode::BadRequest);
        }
    };

    // Security: prevent path traversal attacks (check both original and decoded paths)
    if request_path.contains("..") || decoded_path.contains("..") {
        debug!(
            "Rejecting request with path traversal attempt: {} (decoded: {})",
            request_path, decoded_path
        );
        return Err(ResponseCode::NotFound);
    }

    // Construct the file path using the decoded path
    let mut file_path = String::from("public");
    if !decoded_path.starts_with('/') {
        file_path.push('/');
    }
    file_path.push_str(&decoded_path);

    // Default to index.html for directory requests
    if decoded_path.ends_with('/') {
        file_path.push_str("index.html");
    }

    debug!("Attempting to serve static file: {}", file_path);

    // Try to read the file
    let file_contents = match std::fs::read(&file_path) {
        Ok(contents) => contents,
        Err(e) => {
            debug!("Failed to read static file {}: {}", file_path, e);
            return Err(ResponseCode::NotFound);
        }
    };

    // Determine MIME type
    let mime_type = get_mime_type(&file_path);

    // Build HTTP response
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), vec![mime_type.to_string()]);
    headers.insert(
        "Content-Length".to_string(),
        vec![file_contents.len().to_string()],
    );

    let mut response = String::from("HTTP/1.1 200 OK\r\n");
    for (key, values) in headers {
        response.push_str(&key);
        response.push_str(": ");
        for (i, value) in values.iter().enumerate() {
            if i > 0 {
                response.push_str("; ");
            }
            response.push_str(value);
        }
        response.push_str("\r\n");
    }
    response.push_str("\r\n");

    // Combine response headers with file contents
    let mut response_bytes = response.into_bytes();
    response_bytes.extend(file_contents);

    debug!(
        "Successfully served static file: {} ({} bytes)",
        file_path,
        response_bytes.len()
    );
    Ok(response_bytes)
}

pub struct PicoService {
    port: String,
    secret_key: String,
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
        self.to_string_with_indent(0)
    }
    
    fn to_string_with_indent(&self, indent: usize) -> String {
        let mut res = String::new();
        let indent_str = "  ".repeat(indent);
        
        for (key, node) in &self.nodes {
            res.push_str(&format!("{}{}:\n", indent_str, key));
            res.push_str(&node.to_string_with_indent(indent + 1));
        }
        res
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
    let pico_config_path = config_path.unwrap_or("config.lua".to_string());
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

    let (port, db, routes, route_tree, crons) = match validate_pico_config(pico_config_table) {
        Ok(r) => r,
        Err(es) => return Err(format!("error validating pico config: {}", es)),
    };

    let sql = match initialize_sql_service(&db) {
        Ok(sql) => sql,
        Err(e) => return Err(format!("error initializing sql database: {}", e)),
    };

    // Check if all specified functions in config.lua are initialized in the SQL service
    let mut missing_functions = vec![];
    for r in routes.iter() {
        for h in r.1.definitions.iter() {
            if h.1.sql_function_name.is_some() {
                let sql_name = h.1.sql_function_name.clone().unwrap();
                let func_name = sql_name.strip_suffix(".sql").unwrap_or(&sql_name);
                if sql.functions.get(func_name).is_none() {
                    missing_functions.push(h.1.sql_function_name.clone().unwrap())
                }
            }
        }
    }
    if missing_functions.len() > 0 {
        return Err(format!(
            "SQL handler(s) with name(s): {:#?} specified but does not exist.",
            missing_functions
        ));
    }
    let secret_key = std::env::var("PICO_SECRET_KEY").unwrap_or("default_secret".to_string());

    return Ok(PicoService {
        port,
        secret_key,
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
        error!("Migration name required");
        return;
    }

    let now = Utc::now().timestamp();

    let file_name = format!("migrations/{}:{}.sql", now, input);

    let _file = match File::create(&file_name) {
        Ok(f) => f,
        Err(e) => {
            error!("Migration creation failed: {}", e);
            return;
        }
    };

    info!("Migration file {} created.", &file_name);
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
        error!("Function name required");
        return;
    }

    let file_path = format!("functions/{}.sql", input);

    let mut file = match File::create_new(file_path) {
        Ok(f) => f,
        Err(e) => {
            error!("Function creation failed: {}", e);
            return;
        }
    };

    match file.write(SQL_FUNCTION_TEMPLATE.replace("{name}", input).as_bytes()) {
        Ok(_) => {}
        Err(e) => {
            error!("Function creation failed: {}", e);
            return;
        }
    }

    info!("Function file {} created.", &input);
    return;
}

impl PicoService {
    pub fn start_http_server(&mut self) -> std::io::Result<()> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", self.port))?;

        println!("Pico server listening on {}", listener.local_addr()?);

        // Debug: Display all routes in the route tree
        debug!("Route tree structure:\n{}", self.route_tree.to_string());

        // Debug: Display all configured routes with their methods
        debug!("Configured routes:");
        for (route_path, route) in &self.routes {
            let methods: Vec<String> = route.definitions.keys().map(|m| m.to_string()).collect();
            debug!("  {} -> [{}]", route_path, methods.join(", "));
        }

        for stream in listener.incoming() {
            let mut s = match stream {
                Err(e) => {
                    error!("Error accepting incoming TcpStream: {}", e);
                    continue;
                }
                Ok(s) => s,
            };
            match handle_stream(&mut s) {
                Ok(pr) => {
                    let response = self.handle_http_pico_request(pr);
                    let _nbw = s.write(&response.to_http_bytes()).unwrap();
                }
                Err(rc) => {
                    // TODO: implement failed write retry logic and abstract to write response
                    // code
                    let _nbw = s.write(&rc.to_bytes()).unwrap();
                }
            }
        }
        return Ok(());
    }

    pub fn handle_http_pico_request(&mut self, request: PicoRequest) -> PicoResponse {
        debug!(
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
            debug!("Working on segment: {}", seg);
            match tree.nodes.get(&seg.to_string()) {
                Some(subtree) => {
                    debug!("Found exact match for segment");
                    if !pico_route_path.is_empty() {
                        pico_route_path.push('/');
                    }
                    pico_route_path = pico_route_path + &subtree.parameter_name;
                    tree = &subtree;
                }
                None => {
                    // Try static file first before wildcard routes
                    debug!("No exact match found, checking for static file before wildcard routes");
                    if let Ok(static_response) = try_serve_static_file(&request.path) {
                        debug!("Static file found and served");
                        return PicoResponse::success(static_response);
                    }

                    // If no static file found, try wildcard route match
                    match tree.nodes.get(&"*".to_string()) {
                        Some(subtree) => {
                            debug!("Wildcard match found for segment");
                            route_parameters
                                .insert(subtree.parameter_name.clone(), seg.to_string());
                            if !pico_route_path.is_empty() {
                                pico_route_path.push('/');
                            }
                            pico_route_path = pico_route_path + &subtree.parameter_name;
                            tree = &subtree;
                        }
                        None => {
                            debug!("No route match found for segment");
                            return PicoResponse::error(ResponseCode::NotFound, "Route not found");
                        }
                    }
                }
            }
        }

        debug!("Resolved pico_route_path: {}", pico_route_path);

        let pico_route: &Route = match self.routes.get(&pico_route_path) {
            Some(r) => r,
            None => {
                debug!("No route handlers found for {}", pico_route_path);
                return PicoResponse::error(ResponseCode::NotFound, "Route not found");
            }
        };

        let route_handler = match pico_route.definitions.get(&request.method) {
            Some(rh) => rh,
            None => {
                debug!(
                    "No route handler for {} found with method {}",
                    pico_route_path,
                    request.method.to_string()
                );
                return PicoResponse::error(
                    ResponseCode::NotFound,
                    "Method not allowed for this route",
                );
            }
        };

        debug!("Route handler: {:#?}", route_handler);

        // Extract JWT claims once at the beginning for use throughout the pipeline
        let mut jwt_claims = extract_jwt_claims(&request.headers, &self.secret_key);
        debug!("Extracted JWT claims: {:#?}", jwt_claims);

        let mut json_body = match &route_handler.sql_function_name {
            Some(file_name) => {
                debug!(
                    "Executing sql function {} for route {}",
                    file_name, pico_route_path
                );
                let function_name = file_name.strip_suffix(".sql").unwrap_or(file_name);
                let function = match self.sql.functions.get(function_name) {
                    Some(s) => s,
                    None => {
                        error!(
                            "Internal error getting sql function {} for route {}",
                            function_name, pico_route_path,
                        );
                        return PicoResponse::error(
                            ResponseCode::InternalError,
                            "SQL function not found",
                        );
                    }
                };
                let mut function_input: HashMap<String, Value> = HashMap::new();

                // STEP 1: Build initial function_input from request body and route parameters
                debug!("=== INITIAL PARAMETER BUILDING ===");
                debug!("JSON body provided: {:#?}", request.body);
                debug!("Route parameters provided: {:#?}", route_parameters);
                debug!("Query parameters provided: {:#?}", request.query);

                match request.body {
                    Body::Json(j_body) => {
                        // Add all JSON body parameters
                        if let Some(obj) = j_body.as_object() {
                            for (key, value) in obj {
                                function_input.insert(key.clone(), value.clone());
                            }
                        }
                    }
                    Body::Form(hash_map) => {
                        // Add all form parameters
                        for (key, value) in hash_map {
                            function_input.insert(key.clone(), Value::String(value.clone()));
                        }
                    }
                    Body::Raw(_items) => {
                        warn!("Raw input into SQL not yet implemented");
                        todo!();
                    }
                }

                // Add route parameters (these can override body parameters)
                for (key, value) in route_parameters {
                    function_input.insert(key.clone(), Value::String(value.clone()));
                }

                debug!(
                    "Initial function_input before PREPROCESS: {:#?}",
                    function_input
                );

                // PREPROCESS
                // Apply preprocessing if defined
                if let Some(pre_process_fn) = &route_handler.pre_process {
                    debug!(
                        "Preprocessing request using lua function with JWT: {:#?}",
                        jwt_claims
                    );

                    // Create function input as JSON
                    let function_input_json =
                        serde_json::to_value(&function_input).unwrap_or(Value::Null);
                    let lua_input: mlua::Value = self.lua.to_value(&function_input_json).unwrap();

                    let lua_jwt: mlua::Value = match &jwt_claims {
                        Some(claims) => self.lua.to_value(claims).unwrap(),
                        None => mlua::Value::Nil,
                    };

                    let preprocessed: mlua::Value = match call_lua_function_with_optional_jwt(
                        pre_process_fn,
                        lua_input.clone(),
                        lua_jwt,
                    ) {
                        Ok(p) => p,
                        Err(e) => {
                            // Check if this is a user error (from Lua error() call)
                            if is_user_lua_error(&e) {
                                return PicoResponse::error(
                                    ResponseCode::BadRequest,
                                    &extract_lua_error_message(&e),
                                );
                            }
                            // System error - continue with fallback behavior
                            warn!("Error preprocessing request: {}", e);
                            lua_input.clone()
                        }
                    };

                    // Convert back to function input
                    let preprocessed_json: Value = match self.lua.from_value(preprocessed) {
                        Ok(pj) => pj,
                        Err(e) => {
                            warn!("Error converting preprocessed result back to json: {}", e);
                            function_input_json
                        }
                    };

                    // Update function_input with preprocessed values
                    if let Value::Object(obj) = preprocessed_json {
                        for (key, value) in obj {
                            function_input.insert(key, value);
                        }
                    }
                }

                // STEP 3: Validate that all required SQL function parameters are present
                debug!("=== PARAMETER VALIDATION AFTER PREPROCESS ===");
                debug!("Function expects parameters: {:#?}", function.parameters);
                debug!(
                    "Final function_input after PREPROCESS: {:#?}",
                    function_input
                );

                for param in &function.parameters {
                    if !function_input.contains_key(param) {
                        debug!("=== MISSING PARAMETER ERROR ===");
                        debug!(
                            "Required parameter '{}' not found in final function_input",
                            param
                        );
                        debug!(
                            "Available parameters: {:#?}",
                            function_input.keys().collect::<Vec<_>>()
                        );
                        debug!("This could be due to:");
                        debug!("  1. Parameter missing from request body/route params");
                        debug!(
                            "  2. PREPROCESS function not adding/transforming the required parameter"
                        );
                        return PicoResponse::error(
                            ResponseCode::BadRequest,
                            &format!("Missing required parameter: {}", param),
                        );
                    }
                }
                debug!("All required parameters validated successfully");

                match function.execute(&mut self.sql.connection, function_input) {
                    Ok(value) => value,
                    Err(rc) => {
                        error!(
                            "Error executing sql function {} for route {}: {:?}",
                            function_name,
                            pico_route_path,
                            rc.to_str()
                        );
                        return PicoResponse::error(
                            rc.clone(),
                            &format!("SQL execution failed: {}", rc.to_str()),
                        );
                    }
                }
            }
            None => {
                debug!("No sql function found for {}", pico_route_path);
                Value::Null
            }
        };

        // SETJWT
        let mut headers: HashMap<String, Vec<String>> = HashMap::new();
        if let Some(set_jwt_fn) = &route_handler.set_jwt {
            debug!(
                "Setting JWT using lua function with SQL body: {:#?} and current JWT: {:#?}",
                json_body, jwt_claims
            );

            let lua_body: mlua::Value = match json_body.is_null() {
                true => mlua::Value::Table(self.lua.create_table().unwrap()),
                false => self.lua.to_value(&json_body).unwrap(),
            };
            match set_jwt_fn.call(lua_body.clone()) {
                Ok(claims) => {
                    debug!("Setting JWT: {:#?}", claims);
                    // Convert Lua value to JSON for JWT encoding
                    let new_jwt_claims: Value = match self.lua.from_value(claims) {
                        Ok(jc) => jc,
                        Err(e) => {
                            error!("Error converting lua claims to json: {}", e);
                            return PicoResponse::error(
                                ResponseCode::InternalError,
                                "JWT claims conversion failed",
                            );
                        }
                    };
                    let jwt = match encode(
                        &Header::default(),
                        &new_jwt_claims,
                        &EncodingKey::from_secret(self.secret_key.as_ref()),
                    ) {
                        Ok(jwt) => jwt,
                        Err(e) => {
                            error!("Error encoding JWT: {}", e);
                            "".to_string()
                        }
                    };
                    if jwt != "" {
                        headers.insert(
                            "Set-Cookie".to_string(),
                            vec![format!("pico_jwt={}; HttpOnly; Path=/;", jwt)],
                        );
                        // Update jwt_claims for use in POSTPROCESS
                        jwt_claims = Some(new_jwt_claims);
                    }
                }
                Err(e) => {
                    // Check if this is a user error (from Lua error() call)
                    if is_user_lua_error(&e) {
                        return PicoResponse::error(
                            ResponseCode::Unauthorized,
                            &extract_lua_error_message(&e),
                        );
                    }
                    // System error - log and continue
                    error!("Error setting JWT: {}", e);
                }
            };
        }

        // POSTPROCESS
        // Overwrite json_body with transformed value
        debug!("Initial response body: {}", json_body);
        if let Some(post_process_fn) = &route_handler.post_process {
            debug!(
                "Transforming response {} using lua function with JWT: {:#?}",
                json_body, jwt_claims
            );

            let lua_body: mlua::Value = match json_body.is_null() {
                true => mlua::Value::Table(self.lua.create_table().unwrap()),
                false => self.lua.to_value(&json_body).unwrap(),
            };

            let lua_jwt: mlua::Value = match &jwt_claims {
                Some(claims) => self.lua.to_value(claims).unwrap(),
                None => mlua::Value::Nil,
            };

            let transformed: mlua::Value = match call_lua_function_with_optional_jwt(
                post_process_fn,
                lua_body.clone(),
                lua_jwt,
            ) {
                Ok(t) => t,
                Err(e) => {
                    // Check if this is a user error (from Lua error() call)
                    if is_user_lua_error(&e) {
                        return PicoResponse::error(
                            ResponseCode::BadRequest,
                            &extract_lua_error_message(&e),
                        );
                    }
                    // System error - continue with fallback behavior
                    warn!("Error transforming response body: {}", e);
                    lua_body.clone()
                }
            };

            json_body = match self.lua.from_value(transformed) {
                Ok(jb) => jb,
                Err(e) => {
                    warn!("Error transforming response body back to json: {}", e);
                    json_body
                }
            };
        }

        let mut binding = json_body.to_string();
        let mut body_bytes = binding.as_bytes();

        // VIEW
        if let Some(accept_headers) = request.headers.get("accept") {
            debug!("Accept headers: {:#?}", accept_headers.get(0));
            // If accept headers is text/html and we have a View method on the route
            // render html and return it as the body
            if accept_headers.get(0).unwrap_or(&"".to_string()) == (&"text/html".to_string())
                || request.headers.get("hx-request").unwrap_or(&vec![]).get(0)
                    == Some(&"true".to_string())
            {
                debug!("Accept header is text/html or hx-request is true");
                if let Some(view) = &route_handler.view {
                    debug!("Rendering html view for route");
                    binding = view.to_html(json_body);
                    body_bytes = binding.as_bytes();
                    headers.insert("Content-Type".to_string(), vec!["text/html".to_string()]);
                    // headers.insert("HX-Refresh".to_string(), vec!["true".to_string()]);
                } else {
                    headers.insert(
                        "Content-Type".to_string(),
                        vec!["application/json".to_string()],
                    );
                }
            } else {
                headers.insert(
                    "Content-Type".to_string(),
                    vec!["application/json".to_string()],
                );
            }
        }

        headers.insert(
            "Content-Length".to_string(),
            vec![format!("{}", body_bytes.len())],
        );

        // Create PicoResponse with proper headers and body
        let mut response = PicoResponse::success(body_bytes.to_vec());
        response.headers = headers;

        response
    }
}

// Validate and serialize fields from pico configurations
pub fn validate_pico_config(
    config: mlua::Table,
) -> Result<
    (
        String,
        String,
        HashMap<String, Route>,
        RouteTree,
        Option<Crons>,
    ),
    String,
> {
    let port: String = match config.get("PORT") {
        Ok(p) => p,
        Err(_) => {
            info!("PORT not specified, using port 8080");
            "8080".to_string()
        }
    };
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

    debug!("Routes table: {:#?}", routes_table);

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

            let pre_process: Option<mlua::Function> = match handler.get("PREPROCESS") {
                Ok(v) => v,
                Err(e) => {
                    return Err(format!(
                        "invalid pico config: Route {}: {} has PREPROCESS but is not a function {}",
                        path, method, e
                    ));
                }
            };
            let post_process: Option<mlua::Function> = match handler.get("POSTPROCESS") {
                Ok(v) => v,
                Err(e) => {
                    return Err(format!(
                        "invalid pico config: Route {}: {} has POSTPROCESS but is not a function {}",
                        path, method, e
                    ));
                }
            };

            definitions.insert(
                method,
                RouteHandler {
                    view,
                    sql_function_name: sql,
                    set_jwt,
                    pre_process,
                    post_process,
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
        debug!("Creating route {}", route);
        let mut current = &mut route_tree;
        for seg in route.split("/") {
            if seg.is_empty() {
                continue;
            }
            // Add a wildcard if parameterized
            if seg.starts_with(':') {
                current = current.nodes.entry("*".to_string()).or_insert(RouteTree {
                    nodes: HashMap::new(),
                    parameter_name: seg.to_string(),
                });
            } else {
                current = current.nodes.entry(seg.to_string()).or_insert(RouteTree {
                    nodes: HashMap::new(),
                    parameter_name: seg.to_string(),
                });
            }
        }
    }

    // TODO: implement crons
    // let crons: Option<Crons> = match config.get("CRONS") {
    //     Ok(c) => c,
    //     Err(e) => return Err(format!("invalid pico config: CRONS field not found. {}", e)),
    // };
    //

    return Ok((port, db, routes, route_tree, None));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_mime_type() {
        assert_eq!(get_mime_type("test.html"), "text/html");
        assert_eq!(get_mime_type("styles.css"), "text/css");
        assert_eq!(get_mime_type("script.js"), "application/javascript");
        assert_eq!(get_mime_type("image.png"), "image/png");
        assert_eq!(get_mime_type("image.jpg"), "image/jpeg");
        assert_eq!(get_mime_type("unknown.xyz"), "application/octet-stream");
    }
}
