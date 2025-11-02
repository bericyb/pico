pub mod http {
    use log::{debug, error, warn};
    use regex::Regex;
    use serde_json::{json, Value};
    use std::{collections::HashMap, io::Read, net::TcpStream, time::Duration, vec};

    use crate::{PicoRequest, route::route::Method};

    pub const STREAM_BUFFER_SIZE: usize = 8192;

    const MAX_HEADER_SIZE: usize = 1024;

    #[derive(Debug)]
    pub enum Body {
        Json(Value),
        Form(HashMap<String, String>),
        Raw(Vec<u8>),
    }

    #[derive(Debug, Clone)]
    pub enum ResponseCode {
        Ok,
        NotFound,
        InternalError,
        BadRequest,
        Unauthorized,
        HeaderFieldsTooLarge,
    }

    impl ResponseCode {
        pub fn to_str(&self) -> &str {
            match self {
                ResponseCode::Ok => "OK",
                ResponseCode::NotFound => "Not Found",
                ResponseCode::InternalError => "Internal Server Error",
                ResponseCode::BadRequest => "Bad Request",
                ResponseCode::Unauthorized => "Unauthorized",
                ResponseCode::HeaderFieldsTooLarge => "Header Fields Too Large",
            }
        }

        pub fn to_code(&self) -> u16 {
            match self {
                ResponseCode::Ok => 200,
                ResponseCode::NotFound => 404,
                ResponseCode::InternalError => 500,
                ResponseCode::BadRequest => 400,
                ResponseCode::Unauthorized => 401,
                ResponseCode::HeaderFieldsTooLarge => 431,
            }
        }

        pub fn to_bytes(&self) -> &[u8] {
            match self {
                ResponseCode::Ok => b"HTTP/1.1 200 OK\r\n\r\n",
                ResponseCode::NotFound => b"HTTP/1.1 404 Not Found\r\n\r\n",
                ResponseCode::InternalError => b"HTTP/1.1 500 Internal Server Error\r\n\r\n",
                ResponseCode::BadRequest => b"HTTP/1.1 400 Bad Request\r\n\r\n",
                ResponseCode::Unauthorized => b"HTTP/1.1 401 Unauthorized\r\n\r\n",
                ResponseCode::HeaderFieldsTooLarge => {
                    b"HTTP/1.1 431 Header Fields Too Large\r\n\r\n"
                }
            }
        }
    }

    pub struct PicoResponse {
        pub status: ResponseCode,
        pub body: Vec<u8>,
        pub headers: HashMap<String, Vec<String>>,
    }

    impl PicoResponse {
        pub fn success(body: Vec<u8>) -> Self {
            Self {
                status: ResponseCode::Ok,
                body,
                headers: HashMap::new(),
            }
        }

        pub fn error(status: ResponseCode, message: &str) -> Self {
            let error_json = json!({
                "error": {
                    "message": message,
                    "code": status.to_str()
                }
            });

            let mut headers = HashMap::new();
            headers.insert("Content-Type".to_string(), vec!["application/json".to_string()]);

            Self {
                status,
                body: error_json.to_string().into_bytes(),
                headers,
            }
        }

        pub fn to_http_bytes(&self) -> Vec<u8> {
            let status_line = format!(
                "HTTP/1.1 {} {}\r\n",
                self.status.to_code(),
                self.status.to_str()
            );

            let mut headers_str = String::new();
            for (key, values) in &self.headers {
                for (i, value) in values.iter().enumerate() {
                    if i == 0 {
                        headers_str.push_str(&format!("{}: {}", key, value));
                    } else {
                        headers_str.push_str(&format!("; {}", value));
                    }
                }
                headers_str.push_str("\r\n");
            }

            let response = format!("{}{}\r\n", status_line, headers_str);
            let mut bytes = response.into_bytes();
            bytes.extend_from_slice(&self.body);
            bytes
        }
    }

    pub struct PicoHeader {
        pub name: String,
        pub value: Vec<u8>,
    }

    pub struct ManualRequest {
        pub method: Option<String>,
        pub path: Option<String>,
        pub version: Option<String>,
        pub headers: Vec<PicoHeader>,
    }

    impl ManualRequest {
        pub fn new() -> Self {
            ManualRequest {
                method: None,
                path: None,
                version: None,
                headers: Vec::new(),
            }
        }
    }

    pub struct HttpRequest {
        pub method: String,
        pub path: String,
        pub version: String,
        pub headers: HashMap<String, Vec<String>>, // keeps same iterable contract
    }

    pub fn handle_stream(stream: &mut TcpStream) -> Result<PicoRequest, ResponseCode> {
        let mut buf = Vec::with_capacity(STREAM_BUFFER_SIZE);
        let mut temp = [0u8; 1024];

        // Read until we have headers
        loop {
            let n = stream.read(&mut temp).unwrap_or(0);
            if n == 0 {
                error!("Bad stream with no bytes");
                return Err(ResponseCode::BadRequest);
            }
            buf.extend_from_slice(&temp[..n]);

            if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }

            if buf.len() > MAX_HEADER_SIZE {
                warn!("Request headers too large");
                return Err(ResponseCode::HeaderFieldsTooLarge);
            }
        }

        // Split headers and body
        let header_end = buf
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .ok_or(ResponseCode::BadRequest)?;
        let (header_bytes, body_bytes) = buf.split_at(header_end + 4);

        let header_text = String::from_utf8_lossy(header_bytes);
        let mut lines = header_text.lines();

        // Parse request line
        let request_line = lines.next().ok_or(ResponseCode::BadRequest)?;
        let mut parts = request_line.split_whitespace();
        let method = parts.next().ok_or(ResponseCode::BadRequest)?.to_string();
        let path = parts.next().ok_or(ResponseCode::BadRequest)?.to_string();
        let version = parts.next().ok_or(ResponseCode::BadRequest)?.to_string();

        // Parse headers

        let mut headers: HashMap<String, Vec<String>> = HashMap::new();

        for line in lines {
            if let Some((name, value)) = line.split_once(':') {
                let name = name.trim().to_lowercase();
                let values = value.trim().split(',');
                for value_str in values.map(str::trim) {
                    headers
                        .entry(name.clone())
                        .or_insert_with(Vec::new)
                        .push(value_str.to_string());
                }
            }
        }
        // Build a minimal "request_headers" struct to satisfy parse_to_pico_request
        let simple_req = HttpRequest {
            method,
            path,
            version,
            headers,
        };

        // Call parse_to_pico_request with body slice and stream
        parse_to_pico_request(simple_req, body_bytes, stream)
    }
    fn parse_to_pico_request(
        http_request: HttpRequest,
        read_body: &[u8],
        stream: &mut TcpStream,
    ) -> Result<PicoRequest, ResponseCode> {
        let header_map: HashMap<String, Vec<String>> = http_request.headers;
        let content_length = match header_map
            .get("content-length")
            .and_then(|vals| vals.get(0))
        {
            Some(cl) => cl.to_string(),
            None => "0".to_string(),
        };
        let content_length: usize = match content_length.parse() {
            Ok(len) => len,
            Err(_) => 0,
        };
        debug!("Content length found: {}", content_length);
        let mut body_bytes = vec![];

        body_bytes.extend_from_slice(read_body);
        let read_len = body_bytes.len();

        debug!("Read body byte buffer length: {}", read_len);

        if read_len > content_length {
            let mut remaining_body: Vec<u8> = vec![0u8; content_length];

            debug!("Remaining body length to read: {}", content_length);

            // TODO: add error handling here
            stream
                .set_read_timeout(Some(Duration::new(5, 0)))
                .unwrap_or_default();

            match stream.read(&mut remaining_body) {
                Ok(rb) => {
                    body_bytes.extend_from_slice(&remaining_body[..rb]);
                }
                Err(e) => {
                    error!("Error reading exact body from TcpStream: {}", e);
                    return Err(ResponseCode::BadRequest);
                }
            };
        }

        let content_type: String = match header_map.get("content-type").and_then(|vals| vals.get(0))
        {
            Some(ct) => ct.to_string(),
            None => "application/json".to_string(),
        };

        // Parse and set body based on content-type
        // Currently only support json, urlencoded forms, and multipart forms.
        let mut body: Body = Body::Json(Value::Null);
        match content_type.as_str() {
            "application/json" => {
                let json: Value = serde_json::from_slice(body_bytes.as_slice()).unwrap_or_default();
                body = Body::Json(json);
            }
            "application/x-www-form-urlencoded" => {
                let param_map = url::form_urlencoded::parse(&body_bytes)
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect::<HashMap<String, String>>();
                body = Body::Form(param_map);
                debug!("Parsed form body: {:?}", body);
            }

            // TODO: Find multipart parsing lib since I don't want to do that. XD
            "mutipart/form-data" => {}
            _ => {
                debug!("Unknown content type: {}", content_type);
                body = Body::Raw(body_bytes);
            }
        }

        let mut path = String::new();
        let mut query: HashMap<String, String> = HashMap::new();
        let split_path: Vec<&str> = http_request.path.split('?').collect();
        if split_path.len() == 1 {
            path = split_path[0].to_string();
        } else if split_path.len() == 2 {
            path = split_path[0].to_string();
            let query_string = split_path[1];
            if query_string != "" {
                query = parse_query_parameters(query_string);
            }
        }

        let method: Method = match http_request.method.parse() {
            Ok(m) => m,
            Err(_) => Method::GET,
        };

        // Put the request headers and the body together for a complete request
        Ok(PicoRequest {
            method,
            path,
            query,
            version: http_request.version,
            headers: header_map,
            body,
        })
    }

    fn parse_query_parameters(query: &str) -> HashMap<String, String> {
        let mut queries: HashMap<String, String> = HashMap::new();

        let r = Regex::new(r"(\w+)=(\w+)").unwrap();

        for caps in r.captures_iter(query) {
            let key = match caps.get(1) {
                Some(c) => c,
                None => continue,
            };
            let value = match caps.get(2) {
                Some(c) => c,
                None => continue,
            };
            queries.insert(key.as_str().to_string(), value.as_str().to_string());
        }

        return queries;
    }
}
