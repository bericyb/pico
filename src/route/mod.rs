pub mod route {
    use std::{collections::HashMap, fmt};

    use mlua::Function;
    use serde::Deserialize;

    use crate::html::html::View;

    #[derive(Debug, PartialEq)]
    pub struct Route {
        definitions: HashMap<Method, RouteHandler>,
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
    #[derive(Debug, PartialEq)]
    pub struct RouteHandler {
        pub view: Option<View>,
        pub sql: Option<String>, // Name of sql file to execute on request
        pub set_jwt: Option<Function>, // A lua function that sets the JWT for a user
        pub transform: Option<Function>, // A lua function that transforms the data for a request
    }
}
