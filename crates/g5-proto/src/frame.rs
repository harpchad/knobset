//! Packet framing: 12-byte header, optional content, CRC-16/KERMIT trailer.
//!
//! ```text
//! off size field
//!  0   1   flags        low nibble 0x8 = CRC present, high nibble = protocol version (0)
//!  1   3   opcode
//!  4   2   seq          big-endian
//!  6   2   length       big-endian, content length excluding the CRC (0 = no content, no CRC)
//!  8   2   ack_seq      big-endian, seq of the last packet received from the peer
//! 10   1   type/state   high nibble = PacketType, low nibble = state/error
//! 11   1   checksum     (sum(bytes 0..=10) & 0xFF) ^ 0xFF
//! ```

use crate::Error;

pub const HEADER_LEN: usize = 12;
pub const VERSION_FLAGS: u8 = 0x08;

pub type Opcode = [u8; 3];

/// Opcodes observed in PPS V00-04-00-48.
pub mod op {
    use super::Opcode;

    pub const HANDSHAKE: Opcode = [0x00, 0x00, 0x00];
    pub const COMMAND: Opcode = [0x00, 0x00, 0x02];
    pub const CONTINUE: Opcode = [0x00, 0x20, 0x00];
    pub const COMPLETE: Opcode = [0x00, 0x20, 0x01];
    pub const CANCEL: Opcode = [0x00, 0x20, 0x02];
    pub const ABORT: Opcode = [0x00, 0x20, 0x03];
    pub const BYEBYE: Opcode = [0x00, 0xFF, 0xFF];
    pub const START_READ: Opcode = [0x01, 0x00, 0x01];
    pub const START_WRITE: Opcode = [0x01, 0x00, 0x02];
    pub const ACCESS_PASSWORD_WRITE: Opcode = [0x01, 0x00, 0x31];
    pub const WRITE_FILE: Opcode = [0x01, 0x01, 0x02];
    pub const SET_DATE_TIME: Opcode = [0x01, 0x02, 0x01];
    pub const ACCESS_PASSWORD_READ: Opcode = [0x01, 0x00, 0x30];
    pub const READ_PATH: Opcode = [0x01, 0x01, 0x01];
    pub const READ_FILE: Opcode = [0x01, 0x01, 0x03];
    /// Ends a business bracket. Same bytes as `COMPLETE`, sent as a Request.
    pub const END_BUSINESS: Opcode = COMPLETE;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketType {
    Inform = 0,
    Request = 1,
    Response = 2,
    Accepted = 3,
    Ack = 4,
    Nak = 5,
}

impl PacketType {
    fn from_nibble(n: u8) -> Option<Self> {
        Some(match n {
            0 => Self::Inform,
            1 => Self::Request,
            2 => Self::Response,
            3 => Self::Accepted,
            4 => Self::Ack,
            5 => Self::Nak,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packet {
    pub flags: u8,
    pub opcode: Opcode,
    pub seq: u16,
    pub ack_seq: u16,
    pub ptype: PacketType,
    pub state: u8,
    /// Everything between the header and the CRC (session ID included, CRC excluded).
    pub content: Vec<u8>,
}

impl Packet {
    pub fn new(opcode: Opcode, seq: u16, ack_seq: u16, ptype: PacketType, content: Vec<u8>) -> Self {
        Self { flags: VERSION_FLAGS, opcode, seq, ack_seq, ptype, state: 0, content }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(HEADER_LEN + self.content.len() + 2);
        b.push(self.flags);
        b.extend_from_slice(&self.opcode);
        b.extend_from_slice(&self.seq.to_be_bytes());
        b.extend_from_slice(&(self.content.len() as u16).to_be_bytes());
        b.extend_from_slice(&self.ack_seq.to_be_bytes());
        b.push(((self.ptype as u8) << 4) | (self.state & 0x0F));
        b.push(checksum(&b[..11]));
        if !self.content.is_empty() {
            b.extend_from_slice(&self.content);
            b.extend_from_slice(&crc16_kermit(&self.content).to_be_bytes());
        }
        b
    }

    /// Content after the 2-byte session ID.
    pub fn body(&self, has_session: bool) -> &[u8] {
        if has_session { self.content.get(2..).unwrap_or(&[]) } else { &self.content }
    }

    pub fn session(&self) -> Option<u16> {
        (self.content.len() >= 2).then(|| u16::from_be_bytes([self.content[0], self.content[1]]))
    }
}

/// Parsed header; `content_len` tells the caller how many more bytes to read
/// (`content_len + 2` when non-zero).
#[derive(Debug, Clone, Copy)]
pub struct Header {
    pub flags: u8,
    pub opcode: Opcode,
    pub seq: u16,
    pub content_len: usize,
    pub ack_seq: u16,
    pub ptype: PacketType,
    pub state: u8,
}

impl Header {
    pub fn decode(h: &[u8; HEADER_LEN]) -> Result<Self, Error> {
        let want = checksum(&h[..11]);
        if want != h[11] {
            return Err(Error::Checksum { got: h[11], want });
        }
        let ptype = PacketType::from_nibble(h[10] >> 4).ok_or(Error::BadPacketType(h[10] >> 4))?;
        Ok(Self {
            flags: h[0],
            opcode: [h[1], h[2], h[3]],
            seq: u16::from_be_bytes([h[4], h[5]]),
            content_len: u16::from_be_bytes([h[6], h[7]]) as usize,
            ack_seq: u16::from_be_bytes([h[8], h[9]]),
            ptype,
            state: h[10] & 0x0F,
        })
    }

    /// `tail` is the content followed by its 2-byte CRC.
    pub fn into_packet(self, tail: &[u8]) -> Result<Packet, Error> {
        let content = if self.content_len == 0 {
            Vec::new()
        } else {
            let (content, crc) = tail.split_at(self.content_len);
            let got = u16::from_be_bytes([crc[0], crc[1]]);
            let want = crc16_kermit(content);
            if got != want {
                return Err(Error::Crc { got, want });
            }
            content.to_vec()
        };
        Ok(Packet {
            flags: self.flags,
            opcode: self.opcode,
            seq: self.seq,
            ack_seq: self.ack_seq,
            ptype: self.ptype,
            state: self.state,
            content,
        })
    }
}

pub fn checksum(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0u8, |a, b| a.wrapping_add(*b)) ^ 0xFF
}

/// CRC-16/KERMIT: reflected poly 0x1021 (0x8408), init 0, no final XOR.
/// PPS transmits it high byte first.
pub fn crc16_kermit(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &byte in data {
        let mut b = byte;
        for _ in 0..8 {
            crc = if (crc ^ b as u16) & 1 != 0 { (crc >> 1) ^ 0x8408 } else { crc >> 1 };
            b >>= 1;
        }
    }
    crc
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_check_value() {
        assert_eq!(crc16_kermit(b"123456789"), 0x2189);
    }

    #[test]
    fn checksum_inverts_sum() {
        assert_eq!(checksum(&[0x08, 0, 0, 0, 0, 1, 0, 7, 0, 0, 0x10]), !(0x08 + 1 + 7 + 0x10u8));
    }

    #[test]
    fn handshake_request_bytes() {
        let p = Packet::new(op::HANDSHAKE, 1, 0, PacketType::Request, vec![3, 1, 5, 0xFF, 0xFF, 0xFF, 0xFF]);
        let b = p.encode();
        assert_eq!(&b[..11], &[0x08, 0, 0, 0, 0, 1, 0, 7, 0, 0, 0x10]);
        assert_eq!(b[11], checksum(&b[..11]));
        assert_eq!(&b[12..19], &[3, 1, 5, 0xFF, 0xFF, 0xFF, 0xFF]);
        assert_eq!(u16::from_be_bytes([b[19], b[20]]), crc16_kermit(&b[12..19]));
        assert_eq!(b.len(), 21);
    }

    #[test]
    fn roundtrip() {
        let p = Packet::new(op::READ_FILE, 0x1234, 0xBEEF, PacketType::Response, vec![0, 9, 1, 2, 3]);
        let b = p.encode();
        let h = Header::decode(b[..HEADER_LEN].try_into().unwrap()).unwrap();
        assert_eq!(h.content_len, 5);
        assert_eq!(h.into_packet(&b[HEADER_LEN..]).unwrap(), p);
    }

    #[test]
    fn empty_packet_has_no_crc() {
        let b = Packet::new(op::BYEBYE, 3, 0, PacketType::Ack, vec![]).encode();
        assert_eq!(b.len(), HEADER_LEN);
    }

    #[test]
    fn detects_corruption() {
        let mut b = Packet::new(op::READ_FILE, 1, 0, PacketType::Inform, vec![0, 1, 0xAA]).encode();
        b[13] ^= 1;
        let h = Header::decode(b[..HEADER_LEN].try_into().unwrap()).unwrap();
        assert!(matches!(h.into_packet(&b[HEADER_LEN..]), Err(Error::Crc { .. })));
        b[3] ^= 1;
        assert!(matches!(Header::decode(b[..HEADER_LEN].try_into().unwrap()), Err(Error::Checksum { .. })));
    }
}
