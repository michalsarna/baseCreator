//! The numbers of 7 bits per byte. UNVERIFIED (see the module above).

/// 5 bytes, 7 bits each, the first the most significant: 0 ..= 2^35 - 1 (micrometres, or micrometres a second).
pub fn encode35(v: u64) -> [u8; 5] {
    let v = v & ((1 << 35) - 1);
    [((v >> 28) & 0x7F) as u8, ((v >> 21) & 0x7F) as u8, ((v >> 14) & 0x7F) as u8, ((v >> 7) & 0x7F) as u8, (v & 0x7F) as u8]
}

/// `None` when a byte has its top bit set (it is not such a number).
pub fn decode35(b: &[u8]) -> Option<u64> {
    if b.len() < 5 || b[..5].iter().any(|x| x & 0x80 != 0) {
        return None;
    }
    Some(b[..5].iter().fold(0u64, |acc, x| (acc << 7) | *x as u64))
}

/// A distance in millimetres as micrometres in 5 bytes (a negative one is the two's complement of 35 bits).
pub fn mm_to_abs(mm: f64) -> [u8; 5] {
    let um = (mm * 1000.0).round() as i64;
    encode35(um as u64)
}

pub fn abs_to_mm(b: &[u8]) -> Option<f64> {
    let v = decode35(b)?;
    // Sign-extend from 35 bits.
    let signed = if v & (1 << 34) != 0 { v as i64 - (1 << 35) } else { v as i64 };
    Some(signed as f64 / 1000.0)
}

/// 2 bytes, 7 bits each: a signed 14-bit number (micrometres of a relative move).
pub fn encode14(v: i32) -> [u8; 2] {
    let u = (v as u32) & 0x3FFF;
    [((u >> 7) & 0x7F) as u8, (u & 0x7F) as u8]
}

pub fn decode14(b: &[u8]) -> Option<i32> {
    if b.len() < 2 || b[..2].iter().any(|x| x & 0x80 != 0) {
        return None;
    }
    let u = ((b[0] as i32) << 7) | b[1] as i32;
    Some(if u & 0x2000 != 0 { u - 0x4000 } else { u })
}

/// 2 bytes, 7 bits each, unsigned: 0 ..= 16383.
pub fn encode14u(v: u32) -> [u8; 2] {
    let u = v.min(0x3FFF);
    [((u >> 7) & 0x7F) as u8, (u & 0x7F) as u8]
}

pub fn decode14u(b: &[u8]) -> Option<u32> {
    if b.len() < 2 || b[..2].iter().any(|x| x & 0x80 != 0) {
        return None;
    }
    Some(((b[0] as u32) << 7) | b[1] as u32)
}

/// A power in percent (0 to 100) in 2 bytes: 0x3FFF is 100 %.
pub fn percent_to_power(p: f64) -> [u8; 2] {
    encode14u(((p.clamp(0.0, 100.0) / 100.0) * 16383.0).round() as u32)
}

pub fn power_to_percent(b: &[u8]) -> Option<f64> {
    decode14u(b).map(|v| (v as f64 * 100.0 / 16383.0).min(100.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_go_there_and_back_and_no_byte_has_its_top_bit_set() {
        for v in [0u64, 1, 127, 128, 16383, 1_000_000, (1 << 35) - 1] {
            let e = encode35(v);
            assert!(e.iter().all(|b| *b < 0x80));
            assert_eq!(decode35(&e), Some(v));
        }
        for mm in [0.0, 0.001, 12.345, 600.0, -5.5] {
            assert!((abs_to_mm(&mm_to_abs(mm)).unwrap() - mm).abs() < 0.0006, "{mm}");
        }
        for v in [0, 1, -1, 8191, -8192, 100, -100] {
            assert_eq!(decode14(&encode14(v)), Some(v));
        }
        assert_eq!(decode35(&[0x80, 0, 0, 0, 0]), None);
        assert_eq!(decode14(&[1]), None);
    }

    #[test]
    fn powers_are_percent_of_the_largest_and_never_more_than_a_hundred() {
        assert_eq!(percent_to_power(0.0), [0, 0]);
        assert_eq!(percent_to_power(100.0), [0x7F, 0x7F]);
        assert_eq!(percent_to_power(250.0), [0x7F, 0x7F], "above a hundred is a hundred");
        assert!((power_to_percent(&percent_to_power(37.5)).unwrap() - 37.5).abs() < 0.01);
    }
}
