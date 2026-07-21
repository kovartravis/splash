use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

pub fn proxy_run(url: &str) {
    let addr = url.replace("http://", "");
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut stdin_reader = BufReader::new(stdin.lock());

    loop {
        let mut line = String::new();
        match stdin_reader.read_line(&mut line) {
            Ok(0) => return, // EOF
            Ok(_) => {
                let payload = line.trim();
                if payload.is_empty() {
                    continue;
                }
                let payload_bytes = payload.as_bytes();

                if let Ok(mut stream) = TcpStream::connect(&addr) {
                    let request_str = format!(
                        "POST /mcp HTTP/1.1\r\nHost: {}\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n",
                        addr,
                        payload_bytes.len()
                    );
                    let mut full_req = request_str.into_bytes();
                    full_req.extend_from_slice(payload_bytes);
                    if stream.write_all(&full_req).is_ok() {
                        use std::io::Read;
                        let mut response = String::new();
                        if stream.read_to_string(&mut response).is_ok() {
                            if let Some(body) = response.split("\r\n\r\n").nth(1) {
                                let body_trim = body.trim();
                                if !body_trim.is_empty() {
                                    let _ = writeln!(stdout, "{}", body_trim);
                                    let _ = stdout.flush();
                                }
                            }
                        }
                    }
                }
            }
            Err(_) => return, // Read error
        }
    }
}
