pub mod route {
    use std::{collections::HashMap, fmt, str::FromStr};

    use mlua::{FromLua, Function, Lua, Value};
    use serde::Deserialize;

    use crate::html::html::View;

    #[derive(Debug, PartialEq)]
    pub struct Route {
        pub definitions: HashMap<Method, RouteHandler>,
    }

    #[derive(Debug, PartialEq)]
    pub struct RouteHandler {
        pub view: Option<View>,
        pub sproc_name: Option<String>, // Name of sproc to execute on request
        pub set_jwt: Option<Function>,  // A lua function that sets the JWT for a user
        pub transform: Option<Function>, // A lua function that transforms the data for a request
    }

    #[derive(Eq, Deserialize, Debug, Hash, PartialEq)]
    pub enum Method {
        GET,
        POST,
        PUT,
        DELETE,
        WS,
        SSE,
    }

    impl fmt::Display for Method {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let s = match self {
                Method::GET => "GET",
                Method::POST => "POST",
                Method::PUT => "PUT",
                Method::DELETE => "DELETE",
                Method::WS => "WS",
                Method::SSE => "SSE",
            };
            write!(f, "{}", s)
        }
    }

    impl FromStr for Method {
        type Err = ();
        fn from_str(string: &str) -> Result<Self, Self::Err> {
            let m = match string.to_lowercase().as_str() {
                "get" => Method::GET,
                "post" => Method::POST,
                "put" => Method::PUT,
                "delete" => Method::DELETE,
                // TODO: not sure what ws or sse method is
                "ws_upgrade?" => Method::WS,
                "sse" => Method::SSE,
                _ => {
                    println!("unknown method type {}", string);
                    return Err(());
                }
            };
            Ok(m)
        }
    }

    impl FromLua for Method {
        fn from_lua(value: Value, _lua: &Lua) -> mlua::Result<Self> {
            match value {
                Value::String(method) => {
                    let method_string = match method.to_str() {
                        Ok(s) => s.to_string(),
                        Err(_) => {
                            return Err(mlua::Error::FromLuaConversionError {
                        from: "String",
                        to: "pico::route::Method".to_string(),
                        message: Some("invalid route method type, expected strings GET | POST | PUT | DELETE | WS | SSE".to_string()),
                            });
                        }
                    };
                    match method_string.as_str() {
                        "GET" => return Ok(Method::GET),
                        "POST" => return Ok(Method::POST),
                        "PUT" => return Ok(Method::PUT),
                        "DELETE" => return Ok(Method::DELETE),
                        "WS" => return Ok(Method::WS),
                        "SSE" => return Ok(Method::SSE),
                        _ => {
                            return Err(mlua::Error::FromLuaConversionError {
                        from: "String",
                        to: "pico::route::Method".to_string(),
                        message: Some("invalid route method type, expected GET | POST | PUT | DELETE | WS | SSE".to_string()),
                            });
                        }
                    };
                }
                _ => {
                    return Err(mlua::Error::FromLuaConversionError {
                        from: "String",
                        to: "pico::route::Method".to_string(),
                        message: Some("invalid route method type, expected string".to_string()),
                    });
                }
            }
        }
    }
}
