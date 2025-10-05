pub mod http {
    use httparse::{EMPTY_HEADER, Request};
    use regex::Regex;
    use serde_json::Value;
    use std::{collections::HashMap, io::Read, net::TcpStream, time::Duration, vec};
    use url::Url;

    use crate::{PicoRequest, route::route::Method};

    const STREAM_BUFFER_SIZE: usize = 1024;
    const MAX_HEADER_SIZE: usize = 64;
    pub enum Body {
        Json(Value),
        QueryParams(HashMap<String, String>),
        Raw(Vec<u8>),
    }

    pub enum ResponseCode {
        Ok,
        NotFound,
        InternalError,
        BadRequest,
        HeaderFieldsTooLarge,
    }

    impl ResponseCode {
        pub fn to_str(&self) -> &str {
            match self {
                ResponseCode::Ok => "OK",
                ResponseCode::NotFound => "Not Found",
                ResponseCode::InternalError => "Internal Server Error",
                ResponseCode::BadRequest => "Bad Request",
                ResponseCode::HeaderFieldsTooLarge => "Header Fields Too Large",
            }
        }
        pub fn to_bytes(&self) -> &[u8] {
            match self {
                ResponseCode::Ok => b"HTTP/1.1 200 OK\r\n\r\n",
                ResponseCode::NotFound => b"HTTP/1.1 404 Not Found\r\n\r\n",
                ResponseCode::InternalError => b"HTTP/1.1 500 Internal Server Error\r\n\r\n",
                ResponseCode::BadRequest => b"HTTP/1.1 400 Bad Request\r\n\r\n",
                ResponseCode::HeaderFieldsTooLarge => {
                    b"HTTP/1.1 431 Header Fields Too Large\r\n\r\n"
                }
            }
        }
    }

    pub fn handle_stream(stream: &mut TcpStream) -> Result<PicoRequest, ResponseCode> {
        loop {
            let mut headers = [EMPTY_HEADER; MAX_HEADER_SIZE];
            let mut cursor = 0;
            let mut request_headers = Request::new(&mut headers);
            let mut buf = [0; STREAM_BUFFER_SIZE];
            let n = stream.read(&mut buf[cursor..]).unwrap_or(0);

            if n == 0 {
                println!("Bad stream with no bytes");
                break Err(ResponseCode::BadRequest);
            }

            cursor += n;

            let res = request_headers
                .parse(&buf[..cursor])
                .unwrap_or(httparse::Status::Complete(1));

            match res {
                httparse::Status::Complete(body_start) => {
                    if body_start == 1 {
                        println!("Bad request with no body");
                        break Err(ResponseCode::BadRequest);
                    }

                    break parse_to_pico_request(request_headers, &buf[body_start..cursor], stream);
                }
                httparse::Status::Partial => {
                    if cursor > MAX_HEADER_SIZE {
                        println!("Request headers too large");
                        break Err(ResponseCode::HeaderFieldsTooLarge);
                    } else {
                        continue;
                    }
                }
            }
        }
    }

    fn parse_to_pico_request(
        request_headers: httparse::Request,
        read_body: &[u8],
        stream: &mut TcpStream,
    ) -> Result<PicoRequest, ResponseCode> {
        let header_map: HashMap<_, _> = request_headers
            .headers
            .iter()
            .map(|h| (h.name.to_lowercase(), h.value))
            .collect();
        let byte_value = header_map.get("content-length").copied().unwrap_or(b"0");
        let content_length: usize = std::str::from_utf8(byte_value)
            .unwrap_or_default()
            .parse()
            .unwrap_or_default();
        let mut body_bytes = vec![];

        body_bytes.extend_from_slice(read_body);
        let read_len = body_bytes.len();

        let mut remaining_body: Vec<u8> = vec![0u8; content_length - read_len];

        // TODO: add error handling here
        stream
            .set_read_timeout(Some(Duration::new(5, 0)))
            .unwrap_or_default();

        match stream.read_exact(&mut remaining_body) {
            Ok(()) => {
                body_bytes.extend_from_slice(&remaining_body);
            }
            Err(e) => {
                println!("error reading exact body from TcpStream: {}", e);
                return Err(ResponseCode::BadRequest);
            }
        };

        let content_type = std::str::from_utf8(
            header_map
                .get("content-type")
                .copied()
                .unwrap_or(b"application/json"),
        )
        .unwrap_or_default();

        // Parse and set body based on content-type
        // Currently only support json, urlencoded forms, and multipart forms.
        let mut body: Body = Body::Json(Value::Null);
        match content_type {
            "application/json" => {
                let json: Value = serde_json::from_slice(body_bytes.as_slice()).unwrap_or_default();
                body = Body::Json(json);
            }
            "application/x-www-form-urlencoded" => {
                let path_str = request_headers.path.unwrap_or("/");
                let url = Url::parse(&format!("http://localhost:3000{}", String::from(path_str)))
                    .unwrap();
                body = Body::QueryParams(
                    url.query_pairs()
                        .into_iter()
                        .map(|pair| (pair.0.to_string(), pair.1.to_string()))
                        .collect(),
                );
            }

            // TODO: Find multipart parsing lib since I don't want to do that. XD
            "mutipart/form-data" => {}
            _ => {
                println!("unknown content type: {}", content_type);
                body = Body::Raw(body_bytes);
            }
        }

        let mut path = String::new();
        let mut query: HashMap<String, String> = HashMap::new();
        let split_path: Vec<&str> = request_headers.path.unwrap_or("/").split('?').collect();
        if split_path.len() == 1 {
            path = split_path[0].to_string();
        } else if split_path.len() == 2 {
            path = split_path[0].to_string();
            let query_string = split_path[1];
            if query_string != "" {
                query = parse_query_parameters(query_string);
            }
        }

        let method: Method = match request_headers.method.unwrap_or("GET").parse() {
            Ok(m) => m,
            Err(_) => Method::GET,
        };

        // Put the request headers and the body together for a complete request
        Ok(PicoRequest {
            method,
            path,
            query,
            version: request_headers.version.unwrap_or_default(),
            headers: header_map
                .iter()
                .map(|header| (header.0.to_string(), header.1.to_vec()))
                .collect(),
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
