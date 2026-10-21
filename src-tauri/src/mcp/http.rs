//! Streamable HTTP transport for the running app: one local endpoint, guarded against web pages
//! (Origin), DNS rebinding (Host) and form posts (Content-Type).

use super::protocol::{handle_payload, rpc_error};
use super::McpHost;
use serde_json::Value;
use std::{io::Read, sync::Arc, thread::JoinHandle};

const MAX_BODY_BYTES: u64 = 4 * 1024 * 1024;

pub struct McpServer {
    server: Arc<tiny_http::Server>,
    thread: Option<JoinHandle<()>>,
}

impl McpServer {
    pub fn start(port: u16, host: Arc<dyn McpHost>) -> Result<Self, String> {
        let server = tiny_http::Server::http(("127.0.0.1", port))
            .map_err(|e| format!("Cannot listen on 127.0.0.1:{port}: {e}"))?;
        let server = Arc::new(server);
        let worker = server.clone();
        let thread = std::thread::spawn(move || {
            for request in worker.incoming_requests() {
                let host = host.clone();
                // Tool calls may run git for a while; keep accepting other requests meanwhile.
                std::thread::spawn(move || serve(host.as_ref(), request));
            }
        });
        Ok(Self {
            server,
            thread: Some(thread),
        })
    }

    pub fn stop(mut self) {
        self.server.unblock();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

pub fn endpoint(port: u16) -> String {
    format!("http://127.0.0.1:{port}/mcp")
}

fn header(request: &tiny_http::Request, name: &str) -> Option<String> {
    request
        .headers()
        .iter()
        .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(name))
        .map(|h| h.value.as_str().to_string())
}

const LOCAL_HOSTS: &[&str] = &["127.0.0.1", "localhost", "[::1]"];

/// Host part of `host[:port]`, lowercased.
fn host_only(authority: &str) -> String {
    let authority = authority.trim().to_lowercase();
    if authority.starts_with('[') {
        return authority.split(']').next().map(|h| format!("{h}]")).unwrap_or_default();
    }
    authority.split(':').next().unwrap_or_default().to_string()
}

/// Blocks drive-by requests from web pages: browsers always send Origin, so it must be a local page.
fn origin_allowed(origin: Option<&str>) -> bool {
    let Some(origin) = origin else { return true };
    let origin = origin.trim().to_lowercase();
    let Some((scheme, rest)) = origin.split_once("://") else { return false };
    let host = host_only(rest.split('/').next().unwrap_or_default());
    match scheme {
        "http" | "https" => LOCAL_HOSTS.contains(&host.as_str()) || host == "tauri.localhost",
        "tauri" => host == "localhost",
        _ => false,
    }
}

/// DNS rebinding makes a hostile page "same origin" but its Host header still names the attacker.
fn host_allowed(host: Option<&str>) -> bool {
    host.is_some_and(|host| LOCAL_HOSTS.contains(&host_only(host).as_str()))
}

fn is_json_content_type(value: Option<&str>) -> bool {
    value.is_some_and(|value| {
        value.split(';').next().unwrap_or_default().trim().eq_ignore_ascii_case("application/json")
    })
}

fn respond(request: tiny_http::Request, status: u16, body: Option<String>) {
    let mut response = tiny_http::Response::from_string(body.clone().unwrap_or_default()).with_status_code(status);
    if body.is_some() {
        if let Ok(h) = tiny_http::Header::from_bytes("Content-Type", "application/json") {
            response = response.with_header(h);
        }
    }
    let _ = request.respond(response);
}

fn serve(host: &dyn McpHost, mut request: tiny_http::Request) {
    let path = request.url().split('?').next().unwrap_or_default().to_string();
    if path != "/mcp" && path != "/mcp/" && path != "/" {
        return respond(request, 404, None);
    }
    if !origin_allowed(header(&request, "Origin").as_deref())
        || !host_allowed(header(&request, "Host").as_deref())
    {
        return respond(request, 403, None);
    }
    match request.method() {
        tiny_http::Method::Post => {}
        tiny_http::Method::Get | tiny_http::Method::Delete => {
            let mut response = tiny_http::Response::empty(405);
            if let Ok(h) = tiny_http::Header::from_bytes("Allow", "POST") {
                response = response.with_header(h);
            }
            let _ = request.respond(response);
            return;
        }
        _ => return respond(request, 405, None),
    }
    // Exact type: anything a plain HTML form could send skips CORS preflight.
    if !is_json_content_type(header(&request, "Content-Type").as_deref()) {
        return respond(request, 415, None);
    }

    let mut body = String::new();
    if request
        .as_reader()
        .take(MAX_BODY_BYTES)
        .read_to_string(&mut body)
        .is_err()
    {
        return respond(request, 400, None);
    }
    let payload = match serde_json::from_str::<Value>(&body) {
        Ok(payload) => payload,
        Err(_) => {
            let error = rpc_error(Value::Null, -32700, "Parse error");
            return respond(request, 400, Some(error.to_string()));
        }
    };
    match handle_payload(host, &payload) {
        Some(response) => respond(request, 200, Some(response.to_string())),
        None => respond(request, 202, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_foreign_origins() {
        assert!(origin_allowed(None));
        assert!(origin_allowed(Some("http://localhost:3000")));
        assert!(origin_allowed(Some("http://127.0.0.1:47821")));
        assert!(!origin_allowed(Some("https://evil.example")));
        assert!(!origin_allowed(Some("http://localhost.evil.com")));
        assert!(!origin_allowed(Some("http://127.0.0.1.evil.com:47821")));
        assert!(!origin_allowed(Some("null")));
    }

    #[test]
    fn rejects_rebound_hosts_and_form_content_types() {
        assert!(host_allowed(Some("127.0.0.1:47821")));
        assert!(host_allowed(Some("localhost:47821")));
        assert!(!host_allowed(Some("evil.example:47821")));
        assert!(!host_allowed(None));
        assert!(is_json_content_type(Some("application/json; charset=utf-8")));
        assert!(!is_json_content_type(Some("text/plain;application/json")));
    }
}
