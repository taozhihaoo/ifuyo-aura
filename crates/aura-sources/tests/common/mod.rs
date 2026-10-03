//! 测试公共设施：本地 fake HTTP server（§84/§125）。
//! 纯 std TcpListener，不发任何公网请求；记录收到的请求供 zero-request 断言。

#![allow(dead_code)]

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// 一条路由的响应规格。
#[derive(Clone)]
pub struct Route {
    pub path: &'static str,
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    /// 响应前延迟（模拟慢服务器/超时）。
    pub delay_ms: u64,
    /// `true` = 返回畸形响应字节（不是合法 HTTP）。
    pub malformed: bool,
    /// `true` = 支持 Range 请求（解析 Range 头并回 206 切片）。
    pub range: bool,
}

impl Route {
    #[must_use]
    pub fn ok(path: &'static str, content_type: &str, body: Vec<u8>) -> Self {
        Self {
            path,
            status: 200,
            headers: vec![("Content-Type".into(), content_type.into())],
            body,
            delay_ms: 0,
            malformed: false,
            range: false,
        }
    }

    /// 带 Accept-Ranges/Content-Length 的可 seek 音频响应。
    #[must_use]
    pub fn ok_seekable(path: &'static str, content_type: &str, body: Vec<u8>) -> Self {
        Self {
            headers: vec![
                ("Content-Type".into(), content_type.into()),
                ("Accept-Ranges".into(), "bytes".into()),
            ],
            range: true,
            ..Self::ok(path, content_type, body)
        }
    }

    #[must_use]
    pub fn redirect(path: &'static str, location: &str) -> Self {
        Self {
            path,
            status: 302,
            headers: vec![("Location".into(), location.into())],
            body: Vec::new(),
            delay_ms: 0,
            malformed: false,
            range: false,
        }
    }

    #[must_use]
    pub fn status(path: &'static str, status: u16) -> Self {
        Self {
            path,
            status,
            headers: Vec::new(),
            body: b"error".to_vec(),
            delay_ms: 0,
            malformed: false,
            range: false,
        }
    }
}

/// fake server 句柄。
pub struct FakeHttp {
    pub addr: SocketAddr,
    shutdown: Arc<AtomicBool>,
    requests: Arc<Mutex<Vec<String>>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl FakeHttp {
    /// 启动（127.0.0.1 随机端口）。
    pub fn start(routes: Vec<Route>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
        let addr = listener.local_addr().expect("local_addr");
        let shutdown = Arc::new(AtomicBool::new(false));
        let requests: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

        let routes: Arc<Vec<Route>> = Arc::new(routes);
        let shutdown2 = Arc::clone(&shutdown);
        let requests2 = Arc::clone(&requests);
        let thread = std::thread::spawn(move || {
            listener.set_nonblocking(true).ok();
            while !shutdown2.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let routes = Arc::clone(&routes);
                        let requests = Arc::clone(&requests2);
                        std::thread::spawn(move || {
                            handle_conn(stream, &routes, &requests);
                        });
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            addr,
            shutdown,
            requests,
            thread: Some(thread),
        }
    }

    /// 指定 path 的完整 URL。
    #[must_use]
    pub fn url(&self, path: &str) -> String {
        format!("http://{}:{}{}", self.addr.ip(), self.addr.port(), path)
    }

    /// 收到的请求路径（顺序；zero-request 断言用）。
    #[must_use]
    pub fn requested_paths(&self) -> Vec<String> {
        self.requests.lock().expect("requests lock").clone()
    }

    #[must_use]
    pub fn request_count(&self) -> usize {
        self.requested_paths().len()
    }

    pub fn shutdown(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for FakeHttp {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn handle_conn(mut stream: TcpStream, routes: &[Route], requests: &Mutex<Vec<String>>) {
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
    // 读请求头（简化：读到 CRLFCRLF）。
    let mut buf = Vec::with_capacity(1024);
    let mut chunk = [0u8; 512];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                if buf.windows(4).any(|w| w == b"\r\n\r\n") || buf.len() > 8192 {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    let text = String::from_utf8_lossy(&buf);
    let range_header = text
        .lines()
        .find(|l| l.to_ascii_lowercase().starts_with("range:"))
        .and_then(|l| l.split_once(':'))
        .map(|(_, v)| v.trim().to_string());
    let request_line = text.lines().next().unwrap_or("");
    let path = request_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .to_string();
    requests.lock().expect("requests lock").push(path.clone());

    let Some(route) = routes.iter().find(|r| r.path == path) else {
        let _ = write_response(
            &mut stream,
            404,
            &[("Content-Type".into(), "text/plain".into())],
            b"not found",
            0,
            false,
        );
        let _ = stream.flush();
        let _ = stream.shutdown(std::net::Shutdown::Write);
        return;
    };

    // 慢响应（客户端应超时）。
    let deadline = std::time::Instant::now() + Duration::from_millis(route.delay_ms);
    while std::time::Instant::now() < deadline {
        // 客户端取消时（连接关闭），read 会立即返回错误 → 提前退出。
        stream
            .set_read_timeout(Some(Duration::from_millis(20)))
            .ok();
        let mut probe = [0u8; 1];
        match stream.read(&mut probe) {
            Ok(0) => return, // 客户端断开
            Err(e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut => {}
            Err(_) => return,
            Ok(_) => {} // 忽略额外数据
        }
    }

    // Range 支持（network_stream seek 测试）：`bytes=a-` 切片回 206。
    if route.range {
        if let Some(spec) = range_header {
            if let Some(start) = spec
                .strip_prefix("bytes=")
                .and_then(|s| s.split('-').next())
                .and_then(|s| s.parse::<usize>().ok())
            {
                let start = start.min(route.body.len());
                let slice = &route.body[start..];
                let head = format!(
                    "HTTP/1.1 206 Partial Content
Content-Type: application/octet-stream
Content-Range: bytes {}-{}/{}
Content-Length: {}
Accept-Ranges: bytes
Connection: close

",
                    start,
                    route.body.len().saturating_sub(1),
                    route.body.len(),
                    slice.len()
                );
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(slice);
                let _ = stream.flush();
                let _ = stream.shutdown(std::net::Shutdown::Write);
                return;
            }
        }
    }

    let _ = write_response(
        &mut stream,
        route.status,
        &route.headers,
        &route.body,
        0,
        route.malformed,
    );
    let _ = stream.flush();
    // 干净关闭：半关写端 + 排空读端，避免客户端连接池复用已关 socket 时
    // 收到 RST（全量并行测试下的偶发 NetworkError 竞态来源）。
    let _ = stream.shutdown(std::net::Shutdown::Write);
    stream
        .set_read_timeout(Some(Duration::from_millis(50)))
        .ok();
    let mut drain = [0u8; 256];
    while let Ok(n) = stream.read(&mut drain) {
        if n == 0 {
            break;
        }
    }
}

fn write_response(
    stream: &mut TcpStream,
    status: u16,
    headers: &[(String, String)],
    body: &[u8],
    _flags: u32,
    malformed: bool,
) -> std::io::Result<()> {
    if malformed {
        return stream.write_all(b"NOT-HTTP-GARBAGE\r\n\r\n");
    }
    let reason = match status {
        200 => "OK",
        206 => "Partial Content",
        301 => "Moved Permanently",
        302 => "Found",
        403 => "Forbidden",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "Unknown",
    };
    let mut head = format!("HTTP/1.1 {status} {reason}\r\n");
    for (k, v) in headers {
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    head.push_str(&format!("Content-Length: {}\r\n", body.len()));
    head.push_str("Connection: close\r\n\r\n");
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)?;
    Ok(())
}
