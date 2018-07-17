use std;

use inflector::Inflector;
use regex::Regex;

lazy_static! {
    static ref RE_HTTP_VER: Regex = Regex::new(
        r#"^(?x)
(?P<method>\S+)
\s+
(?P<path>\S+)
"#
    ).unwrap();
    static ref RE_HTTP_RESP: Regex = Regex::new(
        r#"^(?x)
\S+
\s+
(?P<code>\S+)
\s+
(?P<status>[^\r\n]*)
$"#
    ).unwrap();
    static ref RE_HTTP_HEADER: Regex = Regex::new(
        r#"^(?x)
(?P<name>[^:]+)
:\s*
(?P<value>[^\r\n]*)
$"#
    ).unwrap();
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum RequestType {
    Request,  // actually a request, with method / path
    Response, // actually a response, with status / code
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum RequestState {
    Head,
    HeadOneNl,
    HeadParse,
    Body,
    Done,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct Request {
    pub rtype: RequestType,
    pub method: String,
    pub path: String,
    pub code: String,
    pub status: String,
    pub headers: std::collections::HashMap<String, String>,
    pub body: Vec<u8>,
    content_length: usize,
    lines: Vec<Vec<u8>>,
    cur: Vec<u8>,
    state: RequestState,
}

impl std::fmt::Debug for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut dbg = f.debug_struct("Request");
        match self.rtype {
            RequestType::Request => {
                dbg.field("method", &self.method);
                dbg.field("path", &self.path);
            }
            RequestType::Response => {
                dbg.field("code", &self.code);
                dbg.field("status", &self.status);
            }
        }
        dbg.field("headers", &self.headers);
        dbg.field("body", &format!("{} bytes", self.body.len()));
        dbg.finish()
    }
}

impl Request {
    pub fn new(rtype: RequestType) -> Self {
        Request {
            rtype,
            method: String::from(""),
            path: String::from(""),
            code: String::from(""),
            status: String::from(""),
            headers: std::collections::HashMap::new(),
            body: Vec::new(),
            content_length: 0,
            lines: Vec::new(),
            cur: Vec::new(),
            state: RequestState::Head,
        }
    }

    pub fn generate(&self) -> Vec<u8> {
        let mut out: Vec<String> = vec![];
        match self.rtype {
            RequestType::Request => {
                out.push(format!(
                    "{} {} HTTP/1.1",
                    self.method.to_uppercase(),
                    self.path
                ));
            }
            RequestType::Response => {
                out.push(format!("HTTP/1.1 {} {}", self.code, self.status));
            }
        }
        for (key, val) in &self.headers {
            if key == &String::from("content-length") {
                continue;
            }
            out.push(format!(
                "{}: {}",
                str::replace(&key.to_title_case(), " ", "-"),
                val
            ));
        }
        if !self.body.is_empty() {
            out.push(format!("Content-Length: {}", self.body.len()));
        }
        let mut out = format!("{}\r\n\r\n", out.join("\r\n")).as_bytes().to_vec();
        // let bf = out.len();
        out.append(&mut self.body.clone());
        out
    }

    pub fn is_done(&self) -> bool {
        match self.state {
            RequestState::Done => true,
            _ => false,
        }
    }

    pub fn check_parse(&mut self, data: &[u8]) -> bool {
        if let RequestState::Done = self.state {
            return true;
        }

        for c in data {
            match self.state {
                RequestState::Head => {
                    if *c == b'\r' {
                        continue;
                    } else if *c == b'\n' {
                        self.lines.push(self.cur.clone());
                        self.cur = Vec::new();
                        self.state = RequestState::HeadOneNl;
                    } else {
                        self.cur.push(*c);
                    }
                }
                RequestState::HeadOneNl => {
                    if *c == b'\r' {
                        continue;
                    } else if *c == b'\n' {
                        self.state = RequestState::HeadParse;
                    } else {
                        self.cur.push(*c);
                        self.state = RequestState::Head;
                    }
                }
                RequestState::HeadParse => {
                    self.cur.push(*c);
                }
                RequestState::Body => {
                    self.cur.push(*c);
                }
                RequestState::Done => return true,
            }
        }

        if let RequestState::HeadParse = self.state {
            let http_ver = self.lines.remove(0);
            let s = String::from_utf8_lossy(&http_ver);

            match self.rtype {
                RequestType::Request => {
                    let c = RE_HTTP_VER.captures(&s).unwrap();
                    self.method = c["method"].to_uppercase();
                    self.path = c["path"].to_lowercase();
                }
                RequestType::Response => {
                    let c = RE_HTTP_RESP.captures(&s).unwrap();
                    self.code = c["code"].to_string();
                    self.status = c["status"].to_string();
                }
            }

            for mut line in self.lines.drain(..) {
                let s = String::from_utf8_lossy(&line);
                let c = RE_HTTP_HEADER.captures(&s).unwrap();
                self.headers.insert(
                    c["name"].trim().to_lowercase(),
                    c["value"].trim().to_string(),
                );
            }

            match self.headers.get("content-length") {
                Some(v) => {
                    self.content_length = v.parse::<usize>().unwrap();
                    if self.content_length > 0 {
                        self.state = RequestState::Body;
                    } else {
                        self.state = RequestState::Done;
                        return true;
                    }
                }
                None => {
                    self.state = RequestState::Done;
                    return true;
                }
            }
        }

        if let RequestState::Body = self.state {
            if self.cur.len() >= self.content_length {
                self.body = self.cur.drain(..).collect();
                self.state = RequestState::Done;
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_parses_request() {
        let mut r = Request::new(RequestType::Request);

        assert_eq!(false, r.check_parse(b"POST /bob HTTP/1.1\r\n"));
        assert_eq!(false, r.check_parse(b"te"));
        assert_eq!(false, r.check_parse(b"st: he"));
        assert_eq!(false, r.check_parse(b"llo\r\n"));
        assert_eq!(true, r.check_parse(b"Content-Length: 4\r\n\r\ntest"));

        assert_eq!("POST", r.method);
        assert_eq!("/bob", r.path);
        assert_eq!("hello", r.headers.get("test").unwrap());
        assert_eq!("test", String::from_utf8_lossy(&r.body));
    }

    #[test]
    fn it_parses_response() {
        let mut r = Request::new(RequestType::Response);

        assert_eq!(false, r.check_parse(b"HTTP/1.1 200 OK\r\n"));
        assert_eq!(false, r.check_parse(b"Content-Type: text/plain\r\n"));
        assert_eq!(false, r.check_parse(b"Content-Length: 4\r\n\r\n"));
        assert_eq!(true, r.check_parse(b"test"));

        assert_eq!("200", r.code);
        assert_eq!("OK", r.status);
        assert_eq!("text/plain", r.headers.get("content-type").unwrap());
        assert_eq!("test", String::from_utf8_lossy(&r.body));
    }

    #[test]
    fn it_generates_request() {
        let mut r = Request::new(RequestType::Request);

        r.method = String::from("POST");
        r.path = String::from("/bob");
        r.headers
            .insert(String::from("content-type"), String::from("text/plain"));
        r.body = b"test".to_vec();

        assert_eq!(
            String::from_utf8_lossy(&r.generate()),
            String::from(
                "POST /bob HTTP/1.1\r
Content-Type: text/plain\r
Content-Length: 4\r
\r
test"
            )
        );
    }

    #[test]
    fn it_generates_response() {
        let mut r = Request::new(RequestType::Response);

        r.code = String::from("200");
        r.status = String::from("OK");
        r.headers
            .insert(String::from("content-type"), String::from("text/plain"));
        r.body = b"test".to_vec();

        assert_eq!(
            String::from_utf8_lossy(&r.generate()),
            String::from(
                "HTTP/1.1 200 OK\r
Content-Type: text/plain\r
Content-Length: 4\r
\r
test"
            )
        );
    }
}
