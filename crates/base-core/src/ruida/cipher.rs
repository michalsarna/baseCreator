//! The byte scrambling. UNVERIFIED (see the module above).

/// The magic of the series that is the most common (RDC644x) according to public descriptions. Others differ.
pub const MAGIC_644X: u8 = 0x88;

/// Swap bit 7 and bit 0.
fn swap_ends(b: u8) -> u8 {
    (b & 0x7E) | (b >> 7) | (b << 7)
}

/// Scramble a byte of a file or of a packet: swap bit 7 and bit 0, xor with the magic, add one.
pub fn swizzle(b: u8, magic: u8) -> u8 {
    (swap_ends(b) ^ magic).wrapping_add(1)
}

/// The way back.
pub fn unswizzle(b: u8, magic: u8) -> u8 {
    swap_ends(b.wrapping_sub(1) ^ magic)
}

pub fn swizzle_all(data: &[u8], magic: u8) -> Vec<u8> {
    data.iter().map(|b| swizzle(*b, magic)).collect()
}

pub fn unswizzle_all(data: &[u8], magic: u8) -> Vec<u8> {
    data.iter().map(|b| unswizzle(*b, magic)).collect()
}

/// The end of a file in plain bytes (UNVERIFIED).
pub const END_OF_FILE: u8 = 0xD7;

/// The magic that makes the last byte of a scrambled file the end of file. With one byte there is exactly one such magic, so the answer
/// must be confirmed by what the rest of the file turns into: it is a candidate, not a finding.
pub fn magic_from_last_byte(scrambled: &[u8]) -> Option<u8> {
    let last = *scrambled.last()?;
    Some(swap_ends(END_OF_FILE) ^ last.wrapping_sub(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_byte_goes_there_and_back_for_every_magic() {
        for magic in 0..=255u8 {
            let mut seen = [false; 256];
            for b in 0..=255u8 {
                let s = swizzle(b, magic);
                assert_eq!(unswizzle(s, magic), b);
                assert!(!seen[s as usize], "one byte to one byte");
                seen[s as usize] = true;
            }
        }
    }

    #[test]
    fn the_magic_is_found_from_the_end_of_file_and_the_whole_file_reads_back() {
        let plain = vec![0xD8, 0x12, 0x88, 0x01, 0x02, 0x03, 0x04, 0x05, 0xD7];
        for magic in [MAGIC_644X, 0x11, 0x38, 0x00, 0xFF] {
            let file = swizzle_all(&plain, magic);
            assert_eq!(magic_from_last_byte(&file), Some(magic));
            assert_eq!(unswizzle_all(&file, magic), plain);
        }
        assert_eq!(magic_from_last_byte(&[]), None);
    }
}
