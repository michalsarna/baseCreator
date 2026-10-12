//! Configuration text for `ser2net`, which shares a serial port over TCP so that another computer can reach
//! the laser (LightCreator's "Network (TCP)" connection type).

/// Which ser2net generation the target computer has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Version {
    /// ser2net 4.x, YAML file `/etc/ser2net.yaml`.
    V4,
    /// ser2net 3.x, line file `/etc/ser2net.conf`.
    V3,
}

impl Version {
    pub fn file(self) -> &'static str {
        match self {
            Version::V4 => "/etc/ser2net.yaml",
            Version::V3 => "/etc/ser2net.conf",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub serial_port: String,
    pub baud: u32,
    pub tcp_port: u16,
    /// Listen only on this address (for example `127.0.0.1` for use through an SSH tunnel); `None` = all.
    pub bind: Option<String>,
    /// A new client replaces the one already connected.
    pub kick_old_user: bool,
}

/// The file content for `version`.
pub fn config(version: Version, s: &Settings) -> String {
    let bind = s.bind.as_deref().map(str::trim).filter(|b| !b.is_empty());
    match version {
        Version::V4 => {
            let accepter = match bind {
                Some(b) => format!("tcp,{b},{}", s.tcp_port),
                None => format!("tcp,{}", s.tcp_port),
            };
            let mut out = String::from("%YAML 1.1\n---\n# Written by LightCreator: the laser on a TCP port.\nconnection: &laser\n");
            out.push_str(&format!("  accepter: {accepter}\n"));
            out.push_str(&format!("  connector: serialdev,{},{}n81,local\n", s.serial_port, s.baud));
            out.push_str("  options:\n");
            out.push_str(&format!("    kickolduser: {}\n", s.kick_old_user));
            out
        }
        Version::V3 => {
            let port = match bind {
                Some(b) => format!("{b},{}", s.tcp_port),
                None => s.tcp_port.to_string(),
            };
            let mut line = format!("{port}:raw:0:{}:{} 8DATABITS NONE 1STOPBIT", s.serial_port, s.baud);
            if s.kick_old_user {
                line.push_str(" kickolduser");
            }
            format!("# Written by LightCreator: the laser on a TCP port.\n{line}\n")
        }
    }
}

/// Is `s` something safe to place inside a single-quoted shell string? (No quotes, no newlines.)
pub fn shell_safe(s: &str) -> bool {
    !s.contains(['\'', '\n', '\r', '\0'])
}

/// Guess the ser2net generation from the output of `ser2net -v`.
pub fn version_from_output(out: &str) -> Option<Version> {
    // The program name contains a digit ("ser2net"), so look at the words, not the characters.
    let token = out.split_whitespace().rev().find(|t| t.chars().next().is_some_and(|c| c.is_ascii_digit()))?;
    let major: u32 = token.split('.').next()?.parse().ok()?;
    Some(if major >= 4 { Version::V4 } else { Version::V3 })
}
