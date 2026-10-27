//! A packet as it arrives from the pager: 12-byte header, then content and
//! CRC. Header::decode and into_packet must reject bad input, not panic.
#![no_main]

use g5_proto::frame::{HEADER_LEN, Header};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Some((head, tail)) = data.split_first_chunk::<HEADER_LEN>() else { return };
    if let Ok(header) = Header::decode(head) {
        let _ = header.into_packet(tail);
    }
});
