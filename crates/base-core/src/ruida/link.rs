//! Sending a stream in datagrams, one at a time, each answered. UNVERIFIED (see the module above).
use super::frame::{self, ACK, NAK};
use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;

/// The port the controller listens on.
pub const CONTROLLER_PORT: u16 = 50200;
/// The port the controller answers to.
pub const REPLY_PORT: u16 = 40200;

#[derive(Debug, PartialEq, Eq)]
pub enum LinkError {
    /// The datagram was not answered in time, as many times as the retries allow.
    NoAnswer,
    /// The controller said no (checksum) as many times as the retries allow.
    Refused,
    Socket(String),
}

pub struct Link {
    socket: UdpSocket,
    target: SocketAddr,
    magic: u8,
    /// How many times a datagram is sent again (after no answer, or a no).
    pub retries: u32,
}

impl Link {
    /// A link to `target`. `local_port` 0 takes any port; a real controller answers to [`REPLY_PORT`].
    pub fn connect(target: SocketAddr, magic: u8, local_port: u16, wait: Duration) -> Result<Link, LinkError> {
        let socket = UdpSocket::bind(("0.0.0.0", local_port)).map_err(|e| LinkError::Socket(e.to_string()))?;
        socket.set_read_timeout(Some(wait)).map_err(|e| LinkError::Socket(e.to_string()))?;
        Ok(Link { socket, target, magic, retries: 3 })
    }

    /// Send a stream in plain bytes: each datagram waits for its answer before the next goes. Returns the number of datagrams sent,
    /// the ones sent again included.
    pub fn send_stream(&self, plain: &[u8]) -> Result<usize, LinkError> {
        let mut sent = 0;
        for d in frame::frames(plain, self.magic) {
            let mut tries = 0;
            loop {
                self.socket.send_to(&d, self.target).map_err(|e| LinkError::Socket(e.to_string()))?;
                sent += 1;
                let mut buf = [0u8; 16];
                let answer = match self.socket.recv_from(&mut buf) {
                    Ok((n, _)) if n >= 1 => Some(buf[0]),
                    _ => None,
                };
                match answer {
                    Some(ACK) => break,
                    other => {
                        tries += 1;
                        if tries > self.retries {
                            return Err(if other == Some(NAK) { LinkError::Refused } else { LinkError::NoAnswer });
                        }
                    }
                }
            }
        }
        Ok(sent)
    }
}

/// A controller that answers as the description says, for the tests: it opens every datagram, answers [`ACK`] or [`NAK`] (a bad checksum)
/// to where the datagram came from, and keeps the bytes it got. `silent_first` datagrams are not answered at all (a lost reply).
pub struct FakeController {
    pub addr: SocketAddr,
    handle: std::thread::JoinHandle<Vec<u8>>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl FakeController {
    pub fn start(magic: u8, silent_first: usize) -> FakeController {
        let socket = UdpSocket::bind("127.0.0.1:0").expect("a port");
        socket.set_read_timeout(Some(Duration::from_millis(50))).ok();
        let addr = socket.local_addr().expect("an address");
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = stop.clone();
        let handle = std::thread::spawn(move || {
            let (mut got, mut silent) = (Vec::new(), silent_first);
            let mut buf = [0u8; 2048];
            while !flag.load(std::sync::atomic::Ordering::Relaxed) {
                let Ok((n, from)) = socket.recv_from(&mut buf) else { continue };
                if silent > 0 {
                    silent -= 1;
                    continue;
                }
                match frame::open(&buf[..n], magic) {
                    Ok(plain) => {
                        got.extend(plain);
                        let _ = socket.send_to(&[ACK], from);
                    }
                    Err(_) => {
                        let _ = socket.send_to(&[NAK], from);
                    }
                }
            }
            got
        });
        FakeController { addr, handle, stop }
    }

    /// Stop it and take what it got.
    pub fn finish(self) -> Vec<u8> {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        self.handle.join().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stream_reaches_the_fake_controller_whole_and_in_order() {
        let c = FakeController::start(0x88, 0);
        let link = Link::connect(c.addr, 0x88, 0, Duration::from_millis(300)).unwrap();
        let plain: Vec<u8> = (0..2300u32).map(|i| (i % 253) as u8).collect();
        assert_eq!(link.send_stream(&plain), Ok(3));
        assert_eq!(c.finish(), plain);
    }

    #[test]
    fn a_lost_answer_is_sent_again_and_a_wrong_magic_goes_unnoticed() {
        // The first datagram is not answered: it is sent again (the controller then has it once, because the lost one was not kept).
        let c = FakeController::start(0x88, 1);
        let link = Link::connect(c.addr, 0x88, 0, Duration::from_millis(150)).unwrap();
        assert_eq!(link.send_stream(&[1, 2, 3]), Ok(2));
        assert_eq!(c.finish(), vec![1, 2, 3]);
        // A controller of another series reads other bytes, but the checksum is of the scrambled bytes, so it cannot tell: it accepts what
        // it cannot read. The test pins that down, because it is why the magic must be known before anything is sent.
        let other = FakeController::start(0x11, 0);
        let link = Link::connect(other.addr, 0x88, 0, Duration::from_millis(150)).unwrap();
        assert_eq!(link.send_stream(&[1, 2, 3]), Ok(1));
        assert_ne!(other.finish(), vec![1, 2, 3], "read with the wrong magic");
        // Nobody listens: no answer after the retries.
        let nobody = UdpSocket::bind("127.0.0.1:0").unwrap();
        let mut link = Link::connect(nobody.local_addr().unwrap(), 0x88, 0, Duration::from_millis(40)).unwrap();
        link.retries = 1;
        assert_eq!(link.send_stream(&[9]), Err(LinkError::NoAnswer));
    }
}
