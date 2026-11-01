//! A datagram: checksum and data. UNVERIFIED (see the module above).
use super::cipher::{swizzle_all, unswizzle_all};

/// The most data bytes in one datagram.
pub const MAX_DATA: usize = 1000;
/// The controller got the datagram.
pub const ACK: u8 = 0xC6;
/// The controller did not like it (checksum): send it again.
pub const NAK: u8 = 0x46;

/// The sum of the bytes in 16 bits.
pub fn checksum(data: &[u8]) -> u16 {
    data.iter().fold(0u16, |acc, b| acc.wrapping_add(*b as u16))
}

/// The datagrams of a stream in plain bytes: each is the checksum (high byte first) of the scrambled data, then the scrambled data.
pub fn frames(plain: &[u8], magic: u8) -> Vec<Vec<u8>> {
    plain
        .chunks(MAX_DATA)
        .map(|chunk| {
            let data = swizzle_all(chunk, magic);
            let mut d = checksum(&data).to_be_bytes().to_vec();
            d.extend_from_slice(&data);
            d
        })
        .collect()
}

#[derive(Debug, PartialEq, Eq)]
pub enum FrameError {
    /// Shorter than the checksum.
    TooShort,
    /// More data than [`MAX_DATA`].
    TooLong,
    /// The checksum does not match the data.
    Checksum,
}

/// The plain bytes of a datagram, when it is whole.
pub fn open(datagram: &[u8], magic: u8) -> Result<Vec<u8>, FrameError> {
    if datagram.len() < 2 {
        return Err(FrameError::TooShort);
    }
    let data = &datagram[2..];
    if data.len() > MAX_DATA {
        return Err(FrameError::TooLong);
    }
    if u16::from_be_bytes([datagram[0], datagram[1]]) != checksum(data) {
        return Err(FrameError::Checksum);
    }
    Ok(unswizzle_all(data, magic))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stream_is_cut_into_datagrams_that_open_to_the_same_bytes() {
        let plain: Vec<u8> = (0..2500u32).map(|i| (i * 7 % 251) as u8).collect();
        let f = frames(&plain, 0x88);
        assert_eq!(f.len(), 3);
        assert!(f.iter().all(|d| d.len() <= MAX_DATA + 2));
        let back: Vec<u8> = f.iter().flat_map(|d| open(d, 0x88).unwrap()).collect();
        assert_eq!(back, plain);
        assert!(frames(&[], 0x88).is_empty());
    }

    #[test]
    fn a_damaged_or_short_datagram_is_not_opened() {
        let mut d = frames(&[1, 2, 3], 0x88).remove(0);
        assert_eq!(open(&d, 0x88).map(|v| v.len()), Ok(3));
        d[3] ^= 1;
        assert_eq!(open(&d, 0x88), Err(FrameError::Checksum));
        assert_eq!(open(&[1], 0x88), Err(FrameError::TooShort));
        assert_eq!(open(&vec![0u8; MAX_DATA + 3], 0x88), Err(FrameError::TooLong));
    }
}
