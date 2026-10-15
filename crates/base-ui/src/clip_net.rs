//! Network access for the clipart gallery: searching the Iconify service and downloading pictures.
//! Requests run on background threads; answers arrive through a channel. Only http(s) addresses are used and
//! every answer is size-limited.
use base_core::clipart::iconify::{self, Hit};
use std::io::Read;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const MAX_JSON: u64 = 1_000_000;
const MAX_SVG: u64 = 1_000_000;

pub enum Req {
    Search { query: String },
    /// Fetch one picture; `key` identifies it in the answer.
    Fetch { key: String, url: String },
}

pub enum Reply {
    Hits { query: String, result: Result<Vec<Hit>, String> },
    Svg { key: String, result: Result<Vec<u8>, String> },
}

pub struct ClipNet {
    tx: Sender<Req>,
    pub rx: Receiver<Reply>,
    base: String,
}

fn get(agent: &ureq::Agent, url: &str, max: u64) -> Result<Vec<u8>, String> {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("only http and https addresses are supported".into());
    }
    let resp = agent.get(url).call().map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    resp.into_body().into_reader().take(max + 1).read_to_end(&mut buf).map_err(|e| e.to_string())?;
    if buf.len() as u64 > max {
        return Err("the answer is too large".into());
    }
    Ok(buf)
}

impl ClipNet {
    /// Start the workers. `base` is the Iconify address (a test can point it at a local server).
    pub fn spawn(ctx: egui::Context, base: &str) -> ClipNet {
        let (tx, rx_req) = channel::<Req>();
        let (tx_rep, rx) = channel::<Reply>();
        let rx_req = Arc::new(Mutex::new(rx_req));
        for _ in 0..3 {
            let (rx_req, tx_rep, ctx, base) = (rx_req.clone(), tx_rep.clone(), ctx.clone(), base.to_string());
            std::thread::spawn(move || {
                let agent = ureq::Agent::from(ureq::Agent::config_builder().timeout_connect(Some(Duration::from_secs(6))).timeout_recv_response(Some(Duration::from_secs(10))).timeout_recv_body(Some(Duration::from_secs(30))).user_agent("LightCreator").build());
                loop {
                    let req = match rx_req.lock() {
                        Ok(r) => r.recv(),
                        Err(_) => return,
                    };
                    let Ok(req) = req else { return };
                    let reply = match req {
                        Req::Search { query } => {
                            let result = get(&agent, &iconify::search_url(&base, &query, 64), MAX_JSON).and_then(|b| iconify::parse_search(&String::from_utf8_lossy(&b)));
                            Reply::Hits { query, result }
                        }
                        Req::Fetch { key, url } => Reply::Svg { key, result: get(&agent, &url, MAX_SVG) },
                    };
                    if tx_rep.send(reply).is_err() {
                        return;
                    }
                    ctx.request_repaint();
                }
            });
        }
        ClipNet { tx, rx, base: base.to_string() }
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    pub fn send(&self, r: Req) {
        let _ = self.tx.send(r);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::TcpListener;

    const SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M2,2L22,2L22,22Z"/></svg>"##;

    fn serve_forever(listener: TcpListener) {
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut s) = stream else { continue };
                std::thread::spawn(move || {
                    let mut buf = [0u8; 2048];
                    let n = s.read(&mut buf).unwrap_or(0);
                    let req = String::from_utf8_lossy(&buf[..n]).to_string();
                    let line = req.lines().next().unwrap_or("").to_string();
                    let (status, ctype, body) = if line.contains("/search?query=cat") {
                        ("200 OK", "application/json", r#"{"icons":["mdi:cat"],"total":1,"collections":{"mdi":{"name":"Material Design Icons","author":{"name":"Pictogrammers"},"license":{"title":"Apache 2.0","spdx":"Apache-2.0"}}}}"#.to_string())
                    } else if line.contains("/mdi/cat.svg") {
                        ("200 OK", "image/svg+xml", SVG.to_string())
                    } else if line.contains("/huge.svg") {
                        ("200 OK", "image/svg+xml", "x".repeat(1_100_000))
                    } else {
                        ("404 Not Found", "text/plain", "nope".to_string())
                    };
                    let _ = write!(s, "HTTP/1.0 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\n\r\n{body}", body.len());
                });
            }
        });
    }

    #[test]
    fn searches_and_downloads_from_a_local_server() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
        serve_forever(listener);
        let net = ClipNet::spawn(egui::Context::default(), &base);
        net.send(Req::Search { query: "cat".into() });
        let hits = match net.rx.recv_timeout(Duration::from_secs(5)).expect("search answer") {
            Reply::Hits { result, .. } => result.expect("hits"),
            _ => panic!("unexpected reply"),
        };
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].license_spdx, "Apache-2.0");
        net.send(Req::Fetch { key: hits[0].key(), url: hits[0].svg_url(net.base()) });
        match net.rx.recv_timeout(Duration::from_secs(5)).expect("svg answer") {
            Reply::Svg { key, result } => {
                assert_eq!(key, "mdi:cat");
                let svg = String::from_utf8(result.expect("bytes")).unwrap();
                assert!(base_core::clipart::normalize_svg(&svg).is_ok());
            }
            _ => panic!("unexpected reply"),
        }
        // Failures are reported, not hidden: missing page, oversize answer, wrong scheme.
        for url in [format!("{base}/missing.svg"), format!("{base}/huge.svg"), "ftp://example.org/x.svg".to_string()] {
            net.send(Req::Fetch { key: url.clone(), url });
            match net.rx.recv_timeout(Duration::from_secs(5)).expect("answer") {
                Reply::Svg { result, .. } => assert!(result.is_err()),
                _ => panic!("unexpected reply"),
            }
        }
    }

    /// Against the real Iconify service: `cargo test -p base-ui -- --ignored real_network` (needs the internet).
    #[test]
    #[ignore = "uses the internet"]
    fn real_network_iconify_search_and_download() {
        let net = ClipNet::spawn(egui::Context::default(), iconify::API);
        net.send(Req::Search { query: "cat".into() });
        let hits = match net.rx.recv_timeout(Duration::from_secs(20)).expect("search answer") {
            Reply::Hits { result, .. } => result.expect("hits"),
            _ => panic!("unexpected reply"),
        };
        assert!(!hits.is_empty());
        net.send(Req::Fetch { key: hits[0].key(), url: hits[0].svg_url(net.base()) });
        match net.rx.recv_timeout(Duration::from_secs(20)).expect("svg answer") {
            Reply::Svg { result, .. } => assert!(base_core::clipart::normalize_svg(&String::from_utf8(result.expect("bytes")).unwrap()).is_ok()),
            _ => panic!("unexpected reply"),
        }
    }
}
