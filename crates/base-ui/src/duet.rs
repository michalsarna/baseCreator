//! A Duet (RepRapFirmware 2 and 3 in standalone mode) over HTTP: the program is uploaded as a file and started, and the state is asked
//! for. There is no streaming and no jogging here: the board runs the file by itself.
use std::io::Read;
use std::time::Duration;

fn agent(secs: u64) -> ureq::Agent {
    ureq::Agent::config_builder().timeout_connect(Some(Duration::from_secs(5))).timeout_global(Some(Duration::from_secs(secs))).http_status_as_error(false).build().into()
}

/// Is this a host name or address (letters, digits, dots, hyphens, an optional `:port`)? Nothing else goes into a URL.
pub fn valid_host(h: &str) -> bool {
    !h.is_empty() && h.len() <= 253 && h.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == ':')
}

/// A name for the file on the board: letters, digits, `-`, `_` and `.`, ending in `.nc` or `.gcode`.
pub fn file_name(name: &str) -> String {
    let stem: String = name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    let stem = stem.trim_matches('_');
    format!("{}.nc", if stem.is_empty() { "job" } else { stem })
}

/// Percent-encode everything but unreserved characters.
pub fn encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn text(resp: ureq::http::Response<ureq::Body>) -> Result<String, String> {
    let status = resp.status();
    let mut s = String::new();
    resp.into_body().into_reader().take(1 << 20).read_to_string(&mut s).map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("HTTP {}", status.as_u16()));
    }
    Ok(s)
}

fn json_num(body: &str, key: &str) -> Option<i64> {
    let at = body.find(&format!("\"{key}\":"))?;
    let rest = body[at + key.len() + 3..].trim_start();
    let end = rest.find(|c: char| !(c.is_ascii_digit() || c == '-')).unwrap_or(rest.len());
    rest[..end].parse().ok()
}

fn json_str(body: &str, key: &str) -> Option<String> {
    let at = body.find(&format!("\"{key}\":\""))?;
    let rest = &body[at + key.len() + 4..];
    Some(rest[..rest.find('"')?].to_string())
}

/// Open a session; `access` is the password of the board (empty for a board without one: it answers `err` 0 at once; a wrong one answers 1).
pub fn connect(host: &str, access: &str) -> Result<(), String> {
    if !valid_host(host) {
        return Err("the address of the board is not valid".into());
    }
    let body = text(agent(8).get(&format!("http://{host}/rr_connect?password={}", encode(access))).call().map_err(|e| e.to_string())?)?;
    match json_num(&body, "err") {
        Some(0) => Ok(()),
        Some(1) => Err("the password of the board is wrong".into()),
        Some(2) => Err("the board has no free session".into()),
        _ => Err("this does not look like a Duet".into()),
    }
}

/// Put the program on the board (in `0:/gcodes/`).
pub fn upload(host: &str, name: &str, gcode: &str) -> Result<(), String> {
    if !valid_host(host) {
        return Err("the address of the board is not valid".into());
    }
    let path = format!("0:/gcodes/{}", file_name(name));
    let resp = agent(120).post(&format!("http://{host}/rr_upload?name={}", encode(&path))).send(gcode.as_bytes()).map_err(|e| e.to_string())?;
    let body = text(resp)?;
    if json_num(&body, "err") == Some(0) {
        Ok(())
    } else {
        Err("the board did not take the file".into())
    }
}

/// Start a program that is on the board.
pub fn start(host: &str, name: &str) -> Result<(), String> {
    if !valid_host(host) {
        return Err("the address of the board is not valid".into());
    }
    let cmd = format!("M32 \"0:/gcodes/{}\"", file_name(name));
    text(agent(15).get(&format!("http://{host}/rr_gcode?gcode={}", encode(&cmd))).call().map_err(|e| e.to_string())?).map(|_| ())
}

/// Stop the program (M0 in a job is a stop) and the spindle.
pub fn stop(host: &str) -> Result<(), String> {
    if !valid_host(host) {
        return Err("the address of the board is not valid".into());
    }
    text(agent(15).get(&format!("http://{host}/rr_gcode?gcode={}", encode("M0"))).call().map_err(|e| e.to_string())?).map(|_| ())
}

/// What the board says it is doing, in words: from the status letter of `rr_status`.
pub fn word(letter: &str) -> &'static str {
    match letter {
        "I" => "Idle",
        "P" => "Printing",
        "S" => "Halted",
        "C" => "Starting",
        "A" => "Paused",
        "D" => "Pausing",
        "R" => "Resuming",
        "B" => "Busy",
        "F" => "Updating",
        "H" => "Halted",
        _ => "Unknown",
    }
}

/// The state of the board and, while it runs a file, how far it is (percent of the file).
pub fn status(host: &str) -> Result<(String, Option<u32>), String> {
    if !valid_host(host) {
        return Err("the address of the board is not valid".into());
    }
    let body = text(agent(8).get(&format!("http://{host}/rr_status?type=1")).call().map_err(|e| e.to_string())?)?;
    let letter = json_str(&body, "status").ok_or_else(|| "this does not look like a Duet".to_string())?;
    let pct = json_num(&body, "fractionPrinted").map(|p| p.clamp(0, 100) as u32);
    Ok((word(&letter).to_string(), pct))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    /// What the test types as the password of the board.
    fn phrase(s: &str) -> String {
        s.to_string()
    }

    /// A board that records the first line and the body of every request and answers with `reply(line)`.
    fn board(reply: impl Fn(&str) -> String + Send + 'static) -> (String, Arc<Mutex<Vec<(String, String)>>>) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap().to_string();
        let seen = Arc::new(Mutex::new(vec![]));
        let seen2 = seen.clone();
        std::thread::spawn(move || {
            for stream in l.incoming().take(8) {
                let Ok(mut s) = stream else { break };
                let _ = s.set_read_timeout(Some(Duration::from_secs(2)));
                let mut buf = vec![0u8; 65536];
                let mut n = 0;
                // Read until the head is complete and the body (Content-Length) is there.
                loop {
                    match s.read(&mut buf[n..]) {
                        Ok(0) | Err(_) => break,
                        Ok(k) => n += k,
                    }
                    let text = String::from_utf8_lossy(&buf[..n]).to_string();
                    if let Some(h) = text.find("\r\n\r\n") {
                        let len = text.lines().find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").and_then(|v| v.trim().parse::<usize>().ok())).unwrap_or(0);
                        if n >= h + 4 + len {
                            break;
                        }
                    }
                }
                let text = String::from_utf8_lossy(&buf[..n]).to_string();
                let line = text.lines().next().unwrap_or("").to_string();
                let body = text.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
                let r = reply(&line);
                seen2.lock().unwrap().push((line, body));
                let _ = write!(s, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", r.len(), r);
            }
        });
        (addr, seen)
    }

    #[test]
    fn a_job_is_uploaded_started_and_its_progress_read() {
        let (host, seen) = board(|line| if line.contains("rr_status") { "{\"status\":\"P\",\"fractionPrinted\":42.5}".into() } else { "{\"err\":0}".into() });
        let access = phrase("my pass");
        connect(&host, &access).unwrap();
        upload(&host, "plaque 1", "G0 X1\nG1 X2\n").unwrap();
        start(&host, "plaque 1").unwrap();
        assert_eq!(status(&host).unwrap(), ("Printing".to_string(), Some(42)));
        let seen = seen.lock().unwrap();
        assert!(seen[0].0.contains("rr_connect?password=my%20pass"), "{:?}", seen[0]);
        assert!(seen[1].0.starts_with("POST /rr_upload?name=0%3A%2Fgcodes%2Fplaque_1.nc"), "{:?}", seen[1]);
        assert_eq!(seen[1].1, "G0 X1\nG1 X2\n", "the program is the body");
        assert!(seen[2].0.contains("rr_gcode?gcode=M32%20%220%3A%2Fgcodes%2Fplaque_1.nc%22"), "{:?}", seen[2]);
    }

    #[test]
    fn a_wrong_password_a_full_board_and_something_that_is_not_a_duet_are_said() {
        let (h1, _) = board(|_| "{\"err\":1}".into());
        assert!(connect(&h1, &phrase("x")).unwrap_err().contains("password"));
        let (h2, _) = board(|_| "{\"err\":2}".into());
        assert!(connect(&h2, &phrase("")).unwrap_err().contains("session"));
        let (h3, _) = board(|_| "<html>hello</html>".into());
        assert!(connect(&h3, &phrase("")).unwrap_err().contains("Duet"));
        assert!(status(&h3).unwrap_err().contains("Duet"));
        let (h4, _) = board(|_| "{\"err\":1}".into());
        assert!(upload(&h4, "a", "G0").unwrap_err().contains("did not take"));
    }

    #[test]
    fn the_address_and_the_file_name_cannot_carry_anything_into_the_url() {
        assert!(valid_host("192.168.1.20") && valid_host("duet.local") && valid_host("10.0.0.5:8080"));
        assert!(!valid_host("") && !valid_host("a/b") && !valid_host("a b") && !valid_host("evil.com/x?y=") && !valid_host("a@b"));
        assert!(connect("a/b", &phrase("")).is_err() && upload("a b", "x", "").is_err() && start("", "x").is_err() && status("x?y").is_err() && stop("x y").is_err());
        assert_eq!(file_name("../../etc/passwd"), "etc_passwd.nc");
        assert_eq!(file_name("   "), "job.nc");
        assert_eq!(file_name("Plaque v2"), "Plaque_v2.nc");
        assert_eq!(encode("a b&c=d/é"), "a%20b%26c%3Dd%2F%C3%A9");
        assert_eq!(word("P"), "Printing");
        assert_eq!(word("?"), "Unknown");
    }
}
