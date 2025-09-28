pub mod http {
    use httparse::{EMPTY_HEADER, Request};
    use serde_json::Value;
    use std::{
        collections::HashMap,
        io::{Read, Write},
        net::TcpStream,
        time::Duration,
        vec,
    };
    use url::Url;

    use crate::PicoRequest;

    const STREAM_BUFFER_SIZE: usize = 1024;
    const MAX_HEADER_SIZE: usize = 64;
    pub enum Body {
        Json(Value),
        QueryParams(HashMap<String, String>),
        Raw(Vec<u8>),
    }
    pub fn handle_stream(mut stream: TcpStream) {
        let response = loop {
            let mut headers = [EMPTY_HEADER; MAX_HEADER_SIZE];
            let mut cursor = 0;
            let mut request_headers = Request::new(&mut headers);
            let mut buf = [0; STREAM_BUFFER_SIZE];
            let n = stream.read(&mut buf[cursor..]).unwrap_or(0);

            if n == 0 {
                println!("Bad stream with no bytes");
                break b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                .to_vec();
            }

            cursor += n;

            let res = request_headers
                .parse(&buf[..cursor])
                .unwrap_or(httparse::Status::Complete(1));

            match res {
                httparse::Status::Complete(body_start) => {
                    if body_start == 1 {
                        println!("Bad request with no body");
                        break b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec();
                    }
                    let response =
                        handle_request(request_headers, &buf[body_start..cursor], &mut stream);

                    break response;
                }
                httparse::Status::Partial => {
                    if cursor > MAX_HEADER_SIZE {
                        println!("Request headers too large");
                        break b"HTTP/1.1 431 Request Header Fields Too Large\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec();
                    } else {
                        continue;
                    }
                }
            }
        };

        let written = stream.write(&response);
    }

    fn handle_request(
        request_headers: httparse::Request,
        read_body: &[u8],
        stream: &mut TcpStream,
    ) -> Vec<u8> {
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
                return vec![];
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
                let path_str = request_headers.path.unwrap_or("");
                let url =
                    Url::parse(&format!("http://localhost{}", String::from(path_str))).unwrap();
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

        // Put the request headers and the body together for a complete request
        let pico_request = PicoRequest {
            method: request_headers.method.unwrap_or("GET").to_string(),
            path: request_headers.path.unwrap_or("/").to_string(),
            version: request_headers.version.unwrap_or_default(),
            headers: header_map
                .iter()
                .map(|header| (header.0.to_string(), header.1.to_vec()))
                .collect(),
            body,
        };

        return b"HTTP/1.1 200 Success\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec();
    }
}
