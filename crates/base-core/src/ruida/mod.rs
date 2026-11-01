//! Ruida laser controllers (RDC644x and the like): the pieces of the protocol that can be written down without a machine.
//!
//! **UNVERIFIED, NOT USED BY ANY PROGRAM.** Everything here comes from public descriptions of the protocol, written again from scratch;
//! no byte of it has been checked against a real controller or a file saved by the software of the maker. Nothing sends anything to a
//! machine, and the programs do not call this module. It exists so that the work can go on the day a real `.rd` file or a recording of
//! the traffic is at hand (see `UNVERIFIED` below for what has to be checked first).
//!
//! * [`cipher`]: the byte scrambling of the files and the packets (the "magic" differs between the series).
//! * [`number`]: the numbers of 7 bits per byte (distances, speeds, powers).
//! * [`frame`]: a UDP datagram: checksum, the limit of its size, the one-byte answers.
//! * [`link`]: sending a stream in datagrams with a retry, and a fake controller that answers like the description says, for tests.
//!
//! # UNVERIFIED
//! 1. The cipher (swap of bit 7 and bit 0, xor with the magic, plus one) and the magic of each series.
//! 2. The numbers: 5 bytes for a position or a speed in micrometres (7 bits per byte, the first byte the most significant), 2 bytes for a
//!    relative move (14 bits, two's complement) and for a power (14 bits, 0x3FFF is 100 %).
//! 3. The datagram: the checksum is the sum of the scrambled bytes of the data in two bytes, high byte first, in front of the data; at most
//!    [`frame::MAX_DATA`] bytes of data; the controller answers [`frame::ACK`] or [`frame::NAK`].
//! 4. The ports: the controller listens on 50200 and answers to 40200 ([`link::CONTROLLER_PORT`], [`link::REPLY_PORT`]).
//! 5. That a file ends with the byte `0xD7` (end of file): [`cipher::magic_from_last_byte`] depends on it.
//!
//! A mistake in a power or a speed can burn a workpiece or the machine, so no code outside this module may build a job from these pieces
//! before every item above is checked against a recording.

pub mod cipher;
pub mod frame;
pub mod link;
pub mod number;
