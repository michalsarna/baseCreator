//! The optional web server and API of every program: a status page, the project as an SVG, the cameras, and a few commands
//! (stop, home, cancel a jog, jog) for the programs that drive a machine. Every request needs the access phrase (HTTP Basic authentication, any user name); there
//! is no encryption, so use it on a network you trust. By default it listens only on this computer. It is a small
//! HTTP/1.1 server on the standard library: one short thread per request.
use serde_json::Value;
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// What the API asks the program to do.
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Stop,
    Home,
    CancelJog,
    /// One of the eight directions of the Device tab, by name.
    Jog(Jog),
    SelectCamera(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Jog {
    Up,
    Down,
    Left,
    Right,
    UpLeft,
    UpRight,
    DownLeft,
    DownRight,
}

impl Jog {
    pub fn parse(s: &str) -> Option<Jog> {
        Some(match s {
            "up" => Jog::Up,
            "down" => Jog::Down,
            "left" => Jog::Left,
            "right" => Jog::Right,
            "up-left" => Jog::UpLeft,
            "up-right" => Jog::UpRight,
            "down-left" => Jog::DownLeft,
            "down-right" => Jog::DownRight,
            _ => return None,
        })
    }
    /// Direction as (x, y) in steps, y down the screen like the arrows of the Device tab.
    pub fn delta(self) -> (f64, f64) {
        match self {
            Jog::Up => (0.0, -1.0),
            Jog::Down => (0.0, 1.0),
            Jog::Left => (-1.0, 0.0),
            Jog::Right => (1.0, 0.0),
            Jog::UpLeft => (-1.0, -1.0),
            Jog::UpRight => (1.0, -1.0),
            Jog::DownLeft => (-1.0, 1.0),
            Jog::DownRight => (1.0, 1.0),
        }
    }
}

/// What the program publishes for the server, and what the server hands back.
#[derive(Default)]
pub struct Shared {
    pub status: Mutex<Value>,
    /// The project as SVG, with the revision it was made for.
    pub svg: Mutex<(u64, Arc<String>)>,
    pub cameras: Mutex<Value>,
    /// The colour schemes of the program, for the page.
    pub themes: Mutex<Value>,
    /// The latest picture of the camera view in the program: width, height, RGB.
    pub frame: Mutex<Option<(u32, u32, Arc<Vec<u8>>)>>,
    pub commands: Mutex<VecDeque<Command>>,
    /// The access phrase every request must carry; the program changes it while the server runs.
    pub access: Mutex<String>,
    /// The name of the program, on the page and in the question of the browser.
    pub app: Mutex<String>,
}
/// The longest access phrase.
pub const MAX_ACCESS: usize = 100;

/// Is this a phrase that can be kept: not empty, not long, no control characters?
pub fn access_ok(s: &str) -> bool {
    !s.is_empty() && s.chars().count() <= MAX_ACCESS && !s.chars().any(|c| c.is_control())
}

/// Are the two equal? Takes the same time whatever the first difference, so the time tells nothing.
fn same_bytes(a: &[u8], b: &[u8]) -> bool {
    let mut diff = (a.len() ^ b.len()) as u32;
    for i in 0..a.len().max(b.len()) {
        diff |= (*a.get(i).unwrap_or(&0) ^ *b.get(i).unwrap_or(&0)) as u32;
    }
    diff == 0
}

/// The bytes as standard base64 (with padding).
pub fn base64_encode(bytes: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            out.push(if i <= c.len() { A[(n >> (18 - 6 * i) & 63) as usize] as char } else { '=' });
        }
    }
    out
}

/// The access phrase as it is kept in the settings (base64: not readable at a glance, but no encryption).
pub fn access_to_setting(phrase: &str) -> String {
    base64_encode(phrase.as_bytes())
}

/// The access phrase from the settings; `None` when it is not valid base64 of a usable phrase.
pub fn access_from_setting(text: &str) -> Option<String> {
    let phrase = String::from_utf8(base64_decode(text)?).ok()?;
    access_ok(&phrase).then_some(phrase)
}

/// The text of a standard base64 string (padding allowed), or `None` when it is not valid.
fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let s = s.trim().trim_end_matches('=');
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        };
        acc = (acc << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

/// Does the `Authorization: Basic ...` value carry the phrase (as the part after the first colon)?
fn access_granted(header: Option<&str>, access: &str) -> bool {
    let Some(h) = header else { return false };
    let Some(b64) = h.trim().strip_prefix("Basic ").or_else(|| h.trim().strip_prefix("basic ")) else { return false };
    let Some(raw) = base64_decode(b64) else { return false };
    let Some(i) = raw.iter().position(|b| *b == b':') else { return false };
    !access.is_empty() && same_bytes(&raw[i + 1..], access.as_bytes())
}

pub struct Server {
    pub addr: SocketAddr,
    stop: Arc<AtomicBool>,
    pub shared: Arc<Shared>,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

const MAX_HEAD: usize = 8192;
const MAX_CONNECTIONS: usize = 16;

impl Server {
    /// Listen on `port` (0 = any free one); on every interface only when `lan` is set. `app` names the program; it is also the
    /// access phrase until the program sets its own.
    pub fn start(port: u16, lan: bool, app: &str) -> Result<Server, String> {
        let ip = if lan { Ipv4Addr::UNSPECIFIED } else { Ipv4Addr::LOCALHOST };
        let listener = TcpListener::bind(SocketAddr::from((ip, port))).map_err(|e| e.to_string())?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let addr = listener.local_addr().map_err(|e| e.to_string())?;
        let stop = Arc::new(AtomicBool::new(false));
        let shared = Arc::new(Shared::default());
        if let Ok(mut a) = shared.access.lock() {
            *a = app.to_string();
        }
        if let Ok(mut a) = shared.app.lock() {
            *a = app.to_string();
        }
        let (flag, sh) = (stop.clone(), shared.clone());
        let active = Arc::new(AtomicUsize::new(0));
        std::thread::spawn(move || {
            while !flag.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((s, _)) => {
                        if active.load(Ordering::Relaxed) >= MAX_CONNECTIONS {
                            continue;
                        }
                        active.fetch_add(1, Ordering::Relaxed);
                        let (sh, active) = (sh.clone(), active.clone());
                        std::thread::spawn(move || {
                            let _ = handle(s, &sh);
                            active.fetch_sub(1, Ordering::Relaxed);
                        });
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(30)),
                    Err(_) => std::thread::sleep(Duration::from_millis(200)),
                }
            }
        });
        Ok(Server { addr, stop, shared })
    }
}

struct Request {
    method: String,
    path: String,
    query: Vec<(String, String)>,
    /// The value of the Authorization header.
    auth: Option<String>,
}

fn url_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'%' if i + 2 < b.len() => {
                match u8::from_str_radix(&s[i + 1..i + 3], 16) {
                    Ok(v) => {
                        out.push(v);
                        i += 3;
                        continue;
                    }
                    Err(_) => out.push(b'%'),
                }
            }
            b'+' => out.push(b' '),
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn parse_request(head: &str) -> Option<Request> {
    let line = head.lines().next()?;
    let mut it = line.split_whitespace();
    let (method, target, version) = (it.next()?, it.next()?, it.next()?);
    if !version.starts_with("HTTP/1.") || !target.starts_with('/') || target.len() > 2000 {
        return None;
    }
    let (path, q) = target.split_once('?').unwrap_or((target, ""));
    let query = q.split('&').filter(|p| !p.is_empty()).map(|p| {
        let (k, v) = p.split_once('=').unwrap_or((p, ""));
        (url_decode(k), url_decode(v))
    }).collect();
    let auth = head.lines().skip(1).find_map(|l| {
        let (k, v) = l.split_once(':')?;
        k.trim().eq_ignore_ascii_case("authorization").then(|| v.trim().to_string())
    });
    Some(Request { method: method.to_string(), path: url_decode(path), query, auth })
}

fn respond(s: &mut TcpStream, code: u16, ctype: &str, body: &[u8]) -> std::io::Result<()> {
    respond_as(s, code, ctype, body, "Creator")
}

fn respond_as(s: &mut TcpStream, code: u16, ctype: &str, body: &[u8], realm: &str) -> std::io::Result<()> {
    let reason = match code {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        401 => "Unauthorized",
        405 => "Method Not Allowed",
        503 => "Service Unavailable",
        _ => "Error",
    };
    let realm: String = realm.chars().filter(|c| c.is_ascii_alphanumeric() || *c == ' ').collect();
    let challenge = if code == 401 { format!("WWW-Authenticate: Basic realm=\"{realm}\", charset=\"UTF-8\"\r\n") } else { String::new() };
    let head = format!("HTTP/1.1 {code} {reason}\r\n{challenge}Content-Type: {ctype}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n", body.len());
    s.write_all(head.as_bytes())?;
    s.write_all(body)
}

fn json(s: &mut TcpStream, code: u16, v: &Value) -> std::io::Result<()> {
    respond(s, code, "application/json; charset=utf-8", v.to_string().as_bytes())
}

fn handle(mut s: TcpStream, sh: &Shared) -> std::io::Result<()> {
    s.set_read_timeout(Some(Duration::from_secs(5)))?;
    s.set_write_timeout(Some(Duration::from_secs(5)))?;
    let mut buf = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    while !buf.windows(4).any(|w| w == b"\r\n\r\n") {
        let n = s.read(&mut chunk)?;
        if n == 0 || buf.len() + n > MAX_HEAD {
            return respond(&mut s, 400, "text/plain", b"bad request");
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    let head = String::from_utf8_lossy(&buf).to_string();
    let Some(req) = parse_request(&head) else { return respond(&mut s, 400, "text/plain", b"bad request") };
    let access = sh.access.lock().map(|a| a.clone()).unwrap_or_default();
    if !access_granted(req.auth.as_deref(), &access) {
        // A wrong guess waits a moment: guessing the phrase over the network is slow.
        std::thread::sleep(Duration::from_millis(300));
        let app = sh.app.lock().map(|a| a.clone()).unwrap_or_default();
        return respond_as(&mut s, 401, "text/plain", b"access phrase needed", &app);
    }
    route(&mut s, &req, sh)
}

fn route(s: &mut TcpStream, req: &Request, sh: &Shared) -> std::io::Result<()> {
    let get = req.method == "GET" || req.method == "HEAD";
    let post = req.method == "POST";
    let q = |k: &str| req.query.iter().find(|(a, _)| a == k).map(|(_, v)| v.as_str());
    let push = |c: Command| sh.commands.lock().map(|mut v| if v.len() < 64 { v.push_back(c) }).is_ok();
    match (req.path.as_str(), get, post) {
        ("/", true, _) => {
            let app = sh.app.lock().map(|a| a.clone()).unwrap_or_default();
            respond(s, 200, "text/html; charset=utf-8", page(&app).as_bytes())
        }
        ("/api/status", true, _) => {
            let v = sh.status.lock().map(|v| v.clone()).unwrap_or(Value::Null);
            json(s, 200, &v)
        }
        ("/api/project.svg", true, _) => {
            let svg = sh.svg.lock().map(|v| v.1.clone()).unwrap_or_default();
            respond(s, 200, "image/svg+xml; charset=utf-8", svg.as_bytes())
        }
        ("/api/themes", true, _) => {
            let v = sh.themes.lock().map(|v| v.clone()).unwrap_or(Value::Null);
            json(s, 200, &v)
        }
        ("/api/cameras", true, _) => {
            let v = sh.cameras.lock().map(|v| v.clone()).unwrap_or(Value::Null);
            json(s, 200, &v)
        }
        ("/api/camera.jpg", true, _) => {
            let frame = sh.frame.lock().ok().and_then(|f| f.clone());
            match frame.and_then(|(w, h, rgb)| encode_jpeg(w, h, &rgb)) {
                Some(jpg) => respond(s, 200, "image/jpeg", &jpg),
                None => respond(s, 503, "text/plain", b"no picture: open the camera view in the program"),
            }
        }
        ("/api/stop", _, true) => {
            push(Command::Stop);
            json(s, 200, &serde_json::json!({"ok": true}))
        }
        ("/api/home", _, true) => {
            push(Command::Home);
            json(s, 200, &serde_json::json!({"ok": true}))
        }
        ("/api/cancel-jog", _, true) => {
            push(Command::CancelJog);
            json(s, 200, &serde_json::json!({"ok": true}))
        }
        ("/api/jog", _, true) => match q("dir").and_then(Jog::parse) {
            Some(d) => {
                push(Command::Jog(d));
                json(s, 200, &serde_json::json!({"ok": true}))
            }
            None => json(s, 400, &serde_json::json!({"error": "dir must be one of up, down, left, right, up-left, up-right, down-left, down-right"})),
        },
        ("/api/camera/select", _, true) => match q("index").and_then(|v| v.parse::<usize>().ok()).filter(|i| *i < 8) {
            Some(i) => {
                push(Command::SelectCamera(i));
                json(s, 200, &serde_json::json!({"ok": true}))
            }
            None => json(s, 400, &serde_json::json!({"error": "index must be a camera number"})),
        },
        (p, _, _) if p.starts_with("/api/") && !(get || post) => respond(s, 405, "text/plain", b"method not allowed"),
        ("/api/stop" | "/api/home" | "/api/cancel-jog" | "/api/jog" | "/api/camera/select", true, _) => respond(s, 405, "text/plain", b"use POST"),
        _ => respond(s, 404, "text/plain", b"not found"),
    }
}

/// An RGB picture as JPEG, or `None` when it cannot be made.
pub fn encode_jpeg(w: u32, h: u32, rgb: &[u8]) -> Option<Vec<u8>> {
    if w == 0 || h == 0 || rgb.len() < (w as usize) * (h as usize) * 3 {
        return None;
    }
    let mut out = vec![];
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 70).encode(&rgb[..(w as usize) * (h as usize) * 3], w, h, image::ExtendedColorType::Rgb8).ok()?;
    Some(out)
}

/// The page of the web server: status, the camera, the project and the commands.
const PAGE: &str = include_str!("page.html");

/// The page with the name of the program in it (its title and the key its choice of theme is kept under in the browser). A program
/// without a machine says `"machine": false` in its status, and the page shows only the project.
pub fn page(app: &str) -> String {
    let safe: String = app.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    PAGE.replace("{{APP}}", &safe)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The name of the program in the tests, and so its first access phrase.
    const DEFAULT_ACCESS: &str = "LightCreator";

    fn ask(addr: SocketAddr, raw: &str) -> (u16, String) {
        let mut s = TcpStream::connect(addr).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        s.write_all(raw.as_bytes()).unwrap();
        let mut out = vec![];
        let _ = s.read_to_end(&mut out);
        let text = String::from_utf8_lossy(&out).to_string();
        let code = text.split_whitespace().nth(1).and_then(|c| c.parse().ok()).unwrap_or(0);
        (code, text.split("\r\n\r\n").nth(1).unwrap_or("").to_string())
    }

    fn basic(user: &str, phrase: &str) -> String {
        format!("Authorization: Basic {}\r\n", base64_encode(format!("{user}:{phrase}").as_bytes()))
    }

    fn get_as(addr: SocketAddr, path: &str, auth: &str) -> (u16, String) {
        ask(addr, &format!("GET {path} HTTP/1.1\r\nHost: x\r\n{auth}\r\n"))
    }

    fn get(addr: SocketAddr, path: &str) -> (u16, String) {
        get_as(addr, path, &basic("me", DEFAULT_ACCESS))
    }

    fn post(addr: SocketAddr, path: &str) -> (u16, String) {
        ask(addr, &format!("POST {path} HTTP/1.1\r\nHost: x\r\n{}Content-Length: 0\r\n\r\n", basic("me", DEFAULT_ACCESS)))
    }

    #[test]
    fn nothing_is_answered_without_the_access_phrase_and_it_can_change_while_running() {
        let srv = Server::start(0, false, DEFAULT_ACCESS).unwrap();
        for path in ["/", "/api/status", "/api/project.svg", "/api/cameras", "/api/camera.jpg"] {
            assert_eq!(get_as(srv.addr, path, "").0, 401, "{path} without it");
            assert_eq!(get_as(srv.addr, path, &basic("me", "wrong")).0, 401, "{path} with a wrong one");
        }
        let no = ask(srv.addr, "POST /api/stop HTTP/1.1\r\nHost: x\r\nContent-Length: 0\r\n\r\n");
        assert_eq!(no.0, 401);
        assert!(srv.shared.commands.lock().unwrap().is_empty(), "a refused command is not queued");
        let raw = {
            let mut s = TcpStream::connect(srv.addr).unwrap();
            s.write_all(b"GET / HTTP/1.1\r\nHost: x\r\n\r\n").unwrap();
            let mut out = vec![];
            let _ = s.read_to_end(&mut out);
            String::from_utf8_lossy(&out).to_string()
        };
        assert!(raw.contains("WWW-Authenticate: Basic"), "the browser is asked for it");
        // Any user name, the phrase after the first colon, a phrase with colons, upper or lower case of the scheme.
        assert_eq!(get_as(srv.addr, "/api/status", &basic("", DEFAULT_ACCESS)).0, 200);
        assert_eq!(get_as(srv.addr, "/api/status", &basic("a:b", DEFAULT_ACCESS)).0, 401);
        *srv.shared.access.lock().unwrap() = "new:phrase ü".into();
        assert_eq!(get(srv.addr, "/api/status").0, 401, "the old phrase stops working at once");
        assert_eq!(get_as(srv.addr, "/api/status", &basic("x", "new:phrase ü")).0, 200);
        assert_eq!(get_as(srv.addr, "/api/status", "Authorization: Basic !!!\r\n").0, 401);
        assert_eq!(get_as(srv.addr, "/api/status", "Authorization: Bearer abc\r\n").0, 401);
    }

    #[test]
    fn phrases_and_base64() {
        assert!(access_ok("LightCreator") && access_ok("a b ü"));
        assert!(page("Mill<x>Creator").contains("<title>MillxCreator</title>"), "the name is put in as plain letters");
        assert!(!access_ok("") && !access_ok("a\nb") && !access_ok(&"x".repeat(MAX_ACCESS + 1)));
        assert!(access_ok(&"x".repeat(MAX_ACCESS)));
        assert_eq!(base64_decode("bWU6TGlnaHQ=").unwrap(), b"me:Light");
        for raw in ["", "a", "ab", "abc", "abcd", "LightCreator", "ü ž:\"x"] {
            assert_eq!(base64_decode(&base64_encode(raw.as_bytes())).unwrap(), raw.as_bytes(), "{raw}");
        }
        assert_eq!(base64_encode(b"LightCreator"), "TGlnaHRDcmVhdG9y");
        assert_eq!(access_from_setting(&access_to_setting("my phrase ü")).as_deref(), Some("my phrase ü"));
        assert_eq!(access_from_setting("!!!"), None);
        assert_eq!(access_from_setting(""), None, "an empty phrase is not usable");
        assert_eq!(access_from_setting(&base64_encode(&[0xff, 0xfe])), None, "not text");
        assert_eq!(access_from_setting(&base64_encode(b"a\nb")), None, "control characters");
        assert_eq!(base64_decode("bWU6TGlnaHQ").unwrap(), b"me:Light");
        assert!(base64_decode("a$b").is_none());
        assert!(same_bytes(b"abc", b"abc") && !same_bytes(b"abc", b"abd") && !same_bytes(b"abc", b"ab") && !same_bytes(b"", b"a"));
        assert!(!access_granted(None, "x") && !access_granted(Some("Basic bWU6"), ""), "an empty phrase never opens");
    }

    #[test]
    fn the_server_answers_status_svg_cameras_and_the_page() {
        let srv = Server::start(0, false, DEFAULT_ACCESS).unwrap();
        assert!(srv.addr.ip().is_loopback(), "only this computer unless asked");
        *srv.shared.status.lock().unwrap() = serde_json::json!({"device": {"name": "Laser", "connected": true}});
        *srv.shared.svg.lock().unwrap() = (3, Arc::new("<svg/>".into()));
        *srv.shared.cameras.lock().unwrap() = serde_json::json!({"cameras": [{"index": 0, "name": "A", "url": "http://a/x", "active": true, "enabled": true}]});
        let (c, b) = get(srv.addr, "/api/status");
        assert_eq!(c, 200);
        assert_eq!(serde_json::from_str::<Value>(&b).unwrap()["device"]["name"], "Laser");
        assert_eq!(get(srv.addr, "/api/project.svg"), (200, "<svg/>".to_string()));
        assert!(get(srv.addr, "/api/cameras").1.contains("http://a/x"));
        let (c, page) = get(srv.addr, "/");
        assert!(c == 200 && page.contains("LightCreator") && page.contains("STOP"));
        for marker in ["grid-template-areas", "@media (max-width:800px)", "dblclick", "resetView", "rotsel", "rotated", "No camera", "/api/themes", "LightCreator-theme", "st.machine!==false"] {
            assert!(page.contains(marker), "the page lacks {marker}");
        }
        assert!(page.find("id=\"cam\"").unwrap() < page.find("id=\"ctl\"").unwrap() && page.find("id=\"ctl\"").unwrap() < page.find("id=\"prev\"").unwrap());
        *srv.shared.themes.lock().unwrap() = serde_json::json!([{"id": "dark"}]);
        assert!(get(srv.addr, "/api/themes").1.contains("\"dark\""));
        assert_eq!(get(srv.addr, "/nope").0, 404);
        assert_eq!(get(srv.addr, "/api/camera.jpg").0, 503, "no picture yet");
    }

    #[test]
    fn commands_are_queued_by_post_only_and_checked() {
        let srv = Server::start(0, false, DEFAULT_ACCESS).unwrap();
        assert_eq!(get(srv.addr, "/api/stop").0, 405, "a command needs POST");
        assert_eq!(post(srv.addr, "/api/stop").0, 200);
        assert_eq!(post(srv.addr, "/api/home").0, 200);
        assert_eq!(post(srv.addr, "/api/cancel-jog").0, 200);
        assert_eq!(post(srv.addr, "/api/jog?dir=up-left").0, 200);
        assert_eq!(post(srv.addr, "/api/jog?dir=sideways").0, 400);
        assert_eq!(post(srv.addr, "/api/jog").0, 400);
        assert_eq!(post(srv.addr, "/api/camera/select?index=2").0, 200);
        assert_eq!(post(srv.addr, "/api/camera/select?index=99").0, 400);
        assert_eq!(post(srv.addr, "/api/camera/select?index=-1").0, 400);
        let cmds: Vec<Command> = srv.shared.commands.lock().unwrap().drain(..).collect();
        assert_eq!(cmds, vec![Command::Stop, Command::Home, Command::CancelJog, Command::Jog(Jog::UpLeft), Command::SelectCamera(2)]);
    }

    #[test]
    fn bad_requests_are_refused_and_the_server_keeps_running() {
        let srv = Server::start(0, false, DEFAULT_ACCESS).unwrap();
        assert_eq!(ask(srv.addr, "garbage\r\n\r\n").0, 400);
        assert_eq!(ask(srv.addr, "GET relative HTTP/1.1\r\n\r\n").0, 400);
        assert_eq!(ask(srv.addr, "GET / SPDY/3\r\n\r\n").0, 400);
        let huge = format!("GET /{} HTTP/1.1\r\n\r\n", "a".repeat(20_000));
        assert_eq!(ask(srv.addr, &huge).0, 400);
        assert_eq!(get(srv.addr, "/api/status").0, 200, "still alive");
        // The queue of commands is bounded.
        for _ in 0..200 {
            post(srv.addr, "/api/stop");
        }
        assert!(srv.shared.commands.lock().unwrap().len() <= 64);
    }

    #[test]
    fn a_port_in_use_is_reported_and_dropping_the_server_frees_it() {
        let a = Server::start(0, false, DEFAULT_ACCESS).unwrap();
        let port = a.addr.port();
        assert!(Server::start(port, false, DEFAULT_ACCESS).is_err());
        drop(a);
        std::thread::sleep(Duration::from_millis(150));
        assert!(Server::start(port, false, DEFAULT_ACCESS).is_ok());
    }

    #[test]
    fn urls_and_pictures() {
        assert_eq!(url_decode("a%20b+c%zz%4"), "a b c%zz%4");
        let r = parse_request("POST /api/jog?dir=up%2Dleft&x HTTP/1.1\r\nHost: h\r\n\r\n").unwrap();
        assert_eq!((r.method.as_str(), r.path.as_str(), r.query[0].1.as_str()), ("POST", "/api/jog", "up-left"));
        assert!(encode_jpeg(2, 2, &[200u8; 12]).is_some_and(|j| j.starts_with(&[0xFF, 0xD8])));
        assert!(encode_jpeg(0, 2, &[]).is_none() && encode_jpeg(4, 4, &[0u8; 5]).is_none());
        for d in ["up", "down", "left", "right", "up-left", "up-right", "down-left", "down-right"] {
            assert!(Jog::parse(d).is_some());
        }
    }
}
