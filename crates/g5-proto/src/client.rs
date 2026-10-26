//! Session layer: request/accept/response exchanges, ACKs, chunked reads and
//! writes.
//!
//! Writes are limited to the files PPS itself programs (`WRITABLE_PATHS`).
//! There are no delete, reboot, firmware or key-programming operations.

use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use tracing::{debug, trace, warn};

use crate::device::{DeviceInfo, encode_string};
use crate::frame::{HEADER_LEN, Header, Opcode, Packet, PacketType, hex, op};
use crate::Error;

pub const DEFAULT_ADDR: &str = "10.10.0.12:32769";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(8);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(2);
const RETRIES: usize = 3;
/// PPS sends file data in chunks of at most this many bytes.
pub const MAX_CHUNK: usize = 2048;

/// The only pager paths PPS writes when programming a profile.
pub const WRITABLE_PATHS: &[&str] = &[
    "/defpara/Applications/PPS/u3db_PC.db",
    "/defpara/Applications/PPS/u3db.db",
    "/defpara/Applications/PPS/u3db.db.md5",
    "/defpara/Applications/PPS/u3db_PC.zip",
    "/defpara/Applications/PPS/u3db_PC.zip.md5",
    "/defpara/Applications/homescreen/customer.png",
];

/// True for `WRITABLE_PATHS` and `/defpara/Applications/chvoicerecord/<name>.dat` tones.
pub fn is_writable_path(path: &str) -> bool {
    if WRITABLE_PATHS.contains(&path) {
        return true;
    }
    path.strip_prefix("/defpara/Applications/chvoicerecord/")
        .and_then(|n| n.strip_suffix(".dat"))
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'))
}

/// Byte stream to the pager. Implemented for `TcpStream`; tests use a mock.
pub trait Transport: Read + Write {
    fn set_timeout(&mut self, _timeout: Duration) -> io::Result<()> {
        Ok(())
    }
}

impl Transport for TcpStream {
    fn set_timeout(&mut self, timeout: Duration) -> io::Result<()> {
        self.set_read_timeout(Some(timeout))
    }
}

/// Access-check result for `01 00 30`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Granted(u8),
    PasswordRequired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    pub name: String,
    pub size: u32,
}

pub struct Client<T: Transport> {
    io: T,
    seq: u16,
    ack_seq: u16,
    session: u16,
}

impl Client<TcpStream> {
    pub fn connect(addr: SocketAddr, timeout: Duration) -> Result<Self, Error> {
        let stream = TcpStream::connect_timeout(&addr, timeout)?;
        stream.set_nodelay(true)?;
        Ok(Self::new(stream))
    }
}

impl<T: Transport> Client<T> {
    pub fn new(io: T) -> Self {
        Self { io, seq: 1, ack_seq: 0, session: 1 }
    }

    fn next_seq(&mut self) -> u16 {
        let s = self.seq;
        self.seq = if s == u16::MAX { 1 } else { s + 1 };
        s
    }

    fn next_session(&mut self) -> u16 {
        let s = self.session;
        self.session = if s == u16::MAX { 1 } else { s + 1 };
        s
    }

    fn send(&mut self, p: &Packet) -> Result<(), Error> {
        let bytes = p.encode();
        trace!(target: "g5::wire", "tx {:?} op={} {}", p.ptype, hex(&p.opcode), hex(&bytes));
        self.io.write_all(&bytes)?;
        self.io.flush()?;
        Ok(())
    }

    fn recv(&mut self) -> Result<Packet, Error> {
        let mut h = [0u8; HEADER_LEN];
        read_exact(&mut self.io, &mut h)?;
        let header = Header::decode(&h);
        let header = match header {
            Ok(h) => h,
            Err(e) => {
                trace!(target: "g5::wire", "rx bad header {}", hex(&h));
                return Err(e);
            }
        };
        let mut tail = vec![0u8; if header.content_len == 0 { 0 } else { header.content_len + 2 }];
        read_exact(&mut self.io, &mut tail)?;
        trace!(target: "g5::wire", "rx {:?} op={} seq={} {} {}", header.ptype, hex(&header.opcode), header.seq, hex(&h), hex(&tail));
        header.into_packet(&tail)
    }

    fn ack(&mut self, opcode: Opcode, peer_seq: u16) -> Result<(), Error> {
        self.send(&Packet::new(opcode, 0, peer_seq, PacketType::Ack, Vec::new()))
    }

    fn nak(&mut self, opcode: Opcode, state: u8) -> Result<(), Error> {
        let mut p = Packet::new(opcode, 0, self.ack_seq, PacketType::Nak, Vec::new());
        p.state = state;
        self.send(&p)
    }

    /// Receive one packet, NAKing corrupt ones (checksum state 1, CRC state 2) so the pager resends.
    fn recv_checked(&mut self, opcode: Opcode) -> Result<Packet, Error> {
        let mut last = None;
        for _ in 0..RETRIES {
            match self.recv() {
                Ok(p) => return Ok(p),
                Err(e @ Error::Checksum { .. }) => {
                    self.nak(opcode, 1)?;
                    last = Some(e);
                }
                Err(e @ Error::Crc { .. }) => {
                    self.nak(opcode, 2)?;
                    last = Some(e);
                }
                Err(e) => return Err(e),
            }
        }
        Err(last.unwrap())
    }

    fn check_session(p: &Packet, session: Option<u16>) -> Result<(), Error> {
        match (session, p.session()) {
            (Some(want), Some(got)) if got != want => Err(Error::SessionMismatch { got, want }),
            _ => Ok(()),
        }
    }

    /// Send a Request and wait for its Response (handling ACK and Accepted on the way).
    fn exchange(&mut self, opcode: Opcode, session: Option<u16>, body: &[u8], timeout: Duration) -> Result<Packet, Error> {
        self.ack_seq = 0;
        let mut content = session.map(|s| s.to_be_bytes().to_vec()).unwrap_or_default();
        content.extend_from_slice(body);
        let seq = self.next_seq();
        let request = Packet::new(opcode, seq, self.ack_seq, PacketType::Request, content);
        self.io.set_timeout(timeout)?;
        self.send(&request)?;

        let mut resends = 0;
        loop {
            let p = match self.recv_checked(opcode) {
                Ok(p) => p,
                Err(Error::Timeout) if resends + 1 < RETRIES => {
                    resends += 1;
                    debug!("timeout waiting for {}, resending ({resends})", hex(&opcode));
                    self.send(&request)?;
                    continue;
                }
                Err(e) => return Err(e),
            };
            if p.opcode == op::ABORT {
                return Err(Error::Abort(p.body(session.is_some()).first().copied()));
            }
            if p.opcode == op::CANCEL {
                return Err(Error::Cancelled);
            }
            match p.ptype {
                PacketType::Ack => continue,
                PacketType::Nak => return Err(Error::Nak(p.state)),
                PacketType::Accepted => {
                    Self::check_session(&p, session)?;
                    self.ack_seq = p.seq;
                    self.ack(opcode, p.seq)?;
                    let secs = be_uint(p.body(session.is_some()));
                    debug!("pager accepted {}, wait up to {secs}s", hex(&opcode));
                    self.io.set_timeout(Duration::from_secs(secs.max(1) as u64) + timeout)?;
                }
                PacketType::Response => {
                    Self::check_session(&p, session)?;
                    self.ack_seq = p.seq;
                    self.ack(opcode, p.seq)?;
                    self.io.set_timeout(DEFAULT_TIMEOUT)?;
                    return Ok(p);
                }
                other => {
                    warn!("unexpected {other:?} during {}", hex(&opcode));
                    return Err(Error::Unexpected(other));
                }
            }
        }
    }

    /// Identify the pager. Must be the first call on a new connection.
    pub fn handshake(&mut self) -> Result<DeviceInfo, Error> {
        self.seq = 1;
        let body = [0x03, 0x01, 0x05, 0xFF, 0xFF, 0xFF, 0xFF];
        let resp = self.exchange(op::HANDSHAKE, None, &body, HANDSHAKE_TIMEOUT)?;
        DeviceInfo::parse(&resp.content)
    }

    /// Generic Command: returns the result code byte (0 = success).
    fn command(&mut self, opcode: Opcode, params: &[u8]) -> Result<u8, Error> {
        let session = self.next_session();
        let resp = self.exchange(opcode, Some(session), params, DEFAULT_TIMEOUT)?;
        resp.body(true).first().copied().ok_or(Error::Malformed("empty command response"))
    }

    /// Generic Read: Response `[status][totalLen]`, then Inform Continue chunks, then Complete.
    fn read(&mut self, opcode: Opcode, params: &[u8]) -> Result<Vec<u8>, Error> {
        let session = self.next_session();
        let resp = self.exchange(opcode, Some(session), params, DEFAULT_TIMEOUT)?;
        let body = resp.body(true);
        let (&status, len) = body.split_first().ok_or(Error::Malformed("empty read response"))?;
        if status != 0 {
            return Err(Error::DeviceStatus(status));
        }
        let total = be_uint(len) as usize;
        debug!("read {} expects {total} bytes", hex(&opcode));

        let mut data = Vec::with_capacity(total);
        loop {
            let p = self.recv_checked(opcode)?;
            if p.ptype == PacketType::Ack {
                continue;
            }
            if p.ptype != PacketType::Inform {
                return Err(Error::Unexpected(p.ptype));
            }
            if p.seq != 0 && p.seq == self.ack_seq {
                debug!("duplicate chunk seq {}, re-ACK", p.seq);
                self.ack(opcode, p.seq)?;
                continue;
            }
            Self::check_session(&p, Some(session))?;
            self.ack_seq = p.seq;
            self.ack(opcode, p.seq)?;
            match p.opcode {
                op::CONTINUE => {
                    let chunk = p.body(true);
                    if data.len() + chunk.len() > total {
                        return Err(Error::Malformed("read exceeded announced length"));
                    }
                    data.extend_from_slice(chunk);
                }
                op::COMPLETE => break,
                op::ABORT => return Err(Error::Abort(p.body(true).first().copied())),
                op::CANCEL => return Err(Error::Cancelled),
                other => warn!("ignoring inform op {}", hex(&other)),
            }
        }
        if data.len() != total {
            return Err(Error::ShortRead { got: data.len(), want: total });
        }
        Ok(data)
    }

    /// Read-access check. Send an empty password first; PPS then sends
    /// `MD5(password)` as 32 uppercase hex chars if the pager answers 2.
    pub fn access_read(&mut self, password_md5_hex: Option<&str>) -> Result<Access, Error> {
        let pw = password_md5_hex.unwrap_or("").as_bytes();
        let mut body = (pw.len() as u32).to_be_bytes().to_vec();
        body.extend_from_slice(pw);
        let session = self.next_session();
        let resp = self.exchange(op::ACCESS_PASSWORD_READ, Some(session), &body, DEFAULT_TIMEOUT)?;
        match resp.body(true) {
            [rc @ (0 | 1)] => Ok(Access::Granted(*rc)),
            [2] => Ok(Access::PasswordRequired),
            [rc] => Err(Error::AccessCode(*rc)),
            _ => Err(Error::Malformed("access response not 1 byte")),
        }
    }

    /// Write-access check (`01 00 31`); same encoding as `access_read`.
    pub fn access_write(&mut self, password_md5_hex: Option<&str>) -> Result<Access, Error> {
        let pw = password_md5_hex.unwrap_or("").as_bytes();
        let mut body = (pw.len() as u32).to_be_bytes().to_vec();
        body.extend_from_slice(pw);
        let session = self.next_session();
        let resp = self.exchange(op::ACCESS_PASSWORD_WRITE, Some(session), &body, DEFAULT_TIMEOUT)?;
        match resp.body(true) {
            [rc @ (0 | 1)] => Ok(Access::Granted(*rc)),
            [2] => Ok(Access::PasswordRequired),
            [rc] => Err(Error::AccessCode(*rc)),
            _ => Err(Error::Malformed("access response not 1 byte")),
        }
    }

    /// Send one data packet of a Write and wait for its ACK, resending on NAK or timeout.
    fn send_chunk(&mut self, opcode: Opcode, session: u16, data: &[u8]) -> Result<(), Error> {
        let mut content = session.to_be_bytes().to_vec();
        content.extend_from_slice(data);
        let seq = self.next_seq();
        let pkt = Packet::new(opcode, seq, self.ack_seq, PacketType::Inform, content);
        let mut last = Error::Timeout;
        for attempt in 0..RETRIES {
            if attempt > 0 {
                debug!("resending chunk seq {seq} (attempt {})", attempt + 1);
            }
            self.send(&pkt)?;
            match self.recv_checked(opcode) {
                Ok(p) if p.opcode == op::ABORT => return Err(Error::Abort(p.body(true).first().copied())),
                Ok(p) if p.opcode == op::CANCEL => return Err(Error::Cancelled),
                Ok(p) if p.ptype == PacketType::Ack => {
                    if p.ack_seq != seq {
                        warn!("ACK for seq {} while sending seq {seq}", p.ack_seq);
                    }
                    return Ok(());
                }
                Ok(p) if p.ptype == PacketType::Nak => last = Error::Nak(p.state),
                Ok(p) => return Err(Error::Unexpected(p.ptype)),
                Err(Error::Timeout) => last = Error::Timeout,
                Err(e) => return Err(e),
            }
        }
        Err(last)
    }

    /// Generic Write: Request with params, Response `[status]`, then the data in
    /// Inform Continue chunks and a Complete carrying one `00` byte.
    fn write(&mut self, opcode: Opcode, params: &[u8], data: &[u8], progress: &mut dyn FnMut(usize)) -> Result<(), Error> {
        let session = self.next_session();
        let resp = self.exchange(opcode, Some(session), params, DEFAULT_TIMEOUT)?;
        match resp.body(true).first() {
            Some(0) => {}
            Some(&s) => return Err(Error::DeviceStatus(s)),
            None => return Err(Error::Malformed("empty write response")),
        }
        let mut sent = 0;
        for chunk in data.chunks(MAX_CHUNK) {
            self.send_chunk(op::CONTINUE, session, chunk)?;
            sent += chunk.len();
            progress(sent);
        }
        self.send_chunk(op::COMPLETE, session, &[0x00])
    }

    /// Set the pager clock (`01 02 01`, no session). `zone` is POSIX-style as PPS
    /// sends it (`GMT+05:00:00` means UTC-5), `datetime` is local `yyyyMMddHHmmss`.
    /// PPS ignores failures here, so callers may too.
    pub fn set_date_time(&mut self, zone: &str, datetime: &str) -> Result<(), Error> {
        self.ack_seq = 0;
        let embedded = self.next_seq();
        let mut body = embedded.to_be_bytes().to_vec();
        body.push(zone.len() as u8);
        body.extend_from_slice(zone.as_bytes());
        body.extend_from_slice(datetime.as_bytes());
        let seq = self.next_seq();
        self.io.set_timeout(Duration::from_secs(2))?;
        self.send(&Packet::new(op::SET_DATE_TIME, seq, 0, PacketType::Request, body))?;
        let result = loop {
            match self.recv() {
                Ok(p) if p.ptype == PacketType::Ack => continue,
                Ok(p) if p.ptype == PacketType::Response => {
                    self.ack_seq = p.seq;
                    break self.ack(op::SET_DATE_TIME, p.seq);
                }
                Ok(p) => break Err(Error::Unexpected(p.ptype)),
                // The pager may only ACK this request.
                Err(Error::Timeout) => break Ok(()),
                Err(e) => break Err(e),
            }
        };
        self.io.set_timeout(DEFAULT_TIMEOUT)?;
        result
    }

    /// Program files the way PPS does: one write business, each file via
    /// `01 01 02`, then the clock, then commit. Any failure aborts the business
    /// (`00 20 01 [01]`). Every path must pass `is_writable_path`.
    pub fn program_files(&mut self, files: &[(&str, &[u8])], clock: Option<(&str, &str)>) -> Result<(), Error> {
        self.program_files_with_progress(files, clock, &mut |_, _, _| {})
    }

    /// `program_files` with a callback `(file_index, bytes_sent, file_len)` after every chunk.
    pub fn program_files_with_progress(
        &mut self,
        files: &[(&str, &[u8])],
        clock: Option<(&str, &str)>,
        progress: &mut dyn FnMut(usize, usize, usize),
    ) -> Result<(), Error> {
        for (path, _) in files {
            if !is_writable_path(path) {
                return Err(Error::PathNotAllowed(path.to_string()));
            }
        }
        let rc = self.command(op::START_WRITE, &[0x01])?;
        if rc != 0 {
            return Err(Error::BusinessRefused(rc));
        }
        for (i, (path, data)) in files.iter().enumerate() {
            let mut params = (data.len() as u32).to_be_bytes().to_vec();
            params.extend_from_slice(&encode_string(path));
            debug!("writing {path} ({} bytes)", data.len());
            if let Err(e) = self.write(op::WRITE_FILE, &params, data, &mut |sent| progress(i, sent, data.len())) {
                warn!("write of {path} failed, aborting business: {e}");
                if let Err(e2) = self.command(op::END_BUSINESS, &[0x01]) {
                    warn!("abort business failed: {e2}");
                }
                return Err(e);
            }
        }
        if let Some((zone, datetime)) = clock
            && let Err(e) = self.set_date_time(zone, datetime)
        {
            warn!("setting the pager clock failed (PPS ignores this too): {e}");
        }
        let rc = self.command(op::END_BUSINESS, &[0x00])?;
        if rc != 0 {
            return Err(Error::CommitRefused(rc));
        }
        Ok(())
    }

    /// Directory listing (`01 01 01`). Path should end with `/`.
    pub fn list_dir(&mut self, path: &str) -> Result<Vec<DirEntry>, Error> {
        let data = self.read(op::READ_PATH, &encode_string(path))?;
        parse_listing(&data)
    }

    /// Read whole files inside one read business (`01 00 01` ... `00 20 01`).
    pub fn read_files(&mut self, paths: &[&str]) -> Result<Vec<Vec<u8>>, Error> {
        self.read_files_with_progress(paths, &mut |_| {})
    }

    /// `read_files` with a callback after each file (index of the file just read).
    pub fn read_files_with_progress(&mut self, paths: &[&str], progress: &mut dyn FnMut(usize)) -> Result<Vec<Vec<u8>>, Error> {
        let rc = self.command(op::START_READ, &[0x00])?;
        if rc != 0 {
            return Err(Error::BusinessRefused(rc));
        }
        let result = paths
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let r = self.read(op::READ_FILE, &encode_string(p));
                progress(i);
                r
            })
            .collect();
        if let Err(e) = self.command(op::END_BUSINESS, &[0x00]) {
            warn!("ending read business failed: {e}");
        }
        result
    }

    /// Polite disconnect. Errors are ignored; the pager may close first.
    pub fn bye(mut self) {
        let seq = self.next_seq();
        let _ = self.send(&Packet::new(op::BYEBYE, seq, 0, PacketType::Request, Vec::new()));
        let _ = self.io.set_timeout(Duration::from_millis(300));
        let _ = self.recv();
    }

    pub fn into_inner(self) -> T {
        self.io
    }
}

fn read_exact<R: Read>(r: &mut R, buf: &mut [u8]) -> Result<(), Error> {
    r.read_exact(buf).map_err(|e| match e.kind() {
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut => Error::Timeout,
        io::ErrorKind::UnexpectedEof => Error::Disconnected,
        _ => Error::Io(e),
    })
}

fn be_uint(bytes: &[u8]) -> u32 {
    bytes.iter().take(4).fold(0u32, |a, b| (a << 8) | *b as u32)
}

/// `[nameLen BE u32, counts NUL][name][00][size BE u32]` repeated.
pub fn parse_listing(data: &[u8]) -> Result<Vec<DirEntry>, Error> {
    let mut out = Vec::new();
    let mut rest = data;
    while !rest.is_empty() {
        if rest.len() < 4 {
            return Err(Error::Malformed("truncated listing entry"));
        }
        let n = u32::from_be_bytes([rest[0], rest[1], rest[2], rest[3]]) as usize;
        rest = &rest[4..];
        if rest.len() < n + 4 {
            return Err(Error::Malformed("truncated listing name"));
        }
        let name = &rest[..n];
        let name = name.strip_suffix(&[0]).unwrap_or(name);
        let size = u32::from_be_bytes([rest[n], rest[n + 1], rest[n + 2], rest[n + 3]]);
        out.push(DirEntry { name: String::from_utf8_lossy(name).into_owned(), size });
        rest = &rest[n + 4..];
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    /// Feeds scripted pager bytes; records what the client sent.
    struct Mock {
        rx: VecDeque<u8>,
        tx: Vec<u8>,
    }

    impl Read for Mock {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.rx.is_empty() {
                return Err(io::ErrorKind::TimedOut.into());
            }
            let n = buf.len().min(self.rx.len());
            for b in buf.iter_mut().take(n) {
                *b = self.rx.pop_front().unwrap();
            }
            Ok(n)
        }
    }

    impl Write for Mock {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.tx.extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl Transport for Mock {}

    fn pager(packets: &[Packet]) -> Mock {
        Mock { rx: packets.iter().flat_map(|p| p.encode()).collect(), tx: Vec::new() }
    }

    fn with_session(s: u16, body: &[u8]) -> Vec<u8> {
        let mut v = s.to_be_bytes().to_vec();
        v.extend_from_slice(body);
        v
    }

    #[test]
    fn handshake_sends_expected_request_and_acks() {
        let mut content = vec![3, 1, 5, 0];
        content.extend(encode_string(&format!("{:<40}{:<16}", "G5N66AK-SXVXENBX00UNI", "SERIAL")));
        let mock = pager(&[Packet::new(op::HANDSHAKE, 7, 1, PacketType::Response, content)]);
        let mut c = Client::new(mock);
        let info = c.handshake().unwrap();
        assert_eq!(info.model_number.as_deref(), Some("G5N66AK-SXVXENBX00UNI"));
        let tx = c.into_inner().tx;
        assert_eq!(&tx[12..19], &[3, 1, 5, 0xFF, 0xFF, 0xFF, 0xFF]);
        let ack = &tx[21..33];
        assert_eq!(ack[10] >> 4, PacketType::Ack as u8);
        assert_eq!(u16::from_be_bytes([ack[8], ack[9]]), 7);
    }

    #[test]
    fn chunked_read_reassembles() {
        let s = 1;
        let mut listing = encode_string("u3db_PC.zip");
        listing.extend(1234u32.to_be_bytes());
        listing.extend(encode_string("u3db.db"));
        listing.extend(99u32.to_be_bytes());
        let (a, b) = listing.split_at(10);
        let mut status = vec![0];
        status.extend((listing.len() as u32).to_be_bytes());
        let mock = pager(&[
            Packet::new(op::READ_PATH, 0, 1, PacketType::Ack, vec![]),
            Packet::new(op::READ_PATH, 2, 1, PacketType::Response, with_session(s, &status)),
            Packet::new(op::CONTINUE, 3, 0, PacketType::Inform, with_session(s, a)),
            Packet::new(op::CONTINUE, 3, 0, PacketType::Inform, with_session(s, a)), // duplicate
            Packet::new(op::CONTINUE, 4, 0, PacketType::Inform, with_session(s, b)),
            Packet::new(op::COMPLETE, 5, 0, PacketType::Inform, with_session(s, &[0])),
        ]);
        let mut c = Client::new(mock);
        let entries = c.list_dir("/defpara/Applications/PPS/").unwrap();
        assert_eq!(entries, vec![
            DirEntry { name: "u3db_PC.zip".into(), size: 1234 },
            DirEntry { name: "u3db.db".into(), size: 99 },
        ]);
    }

    #[test]
    fn accepted_then_response() {
        let mock = pager(&[
            Packet::new(op::ACCESS_PASSWORD_READ, 2, 1, PacketType::Accepted, with_session(1, &[0, 0, 0, 2])),
            Packet::new(op::ACCESS_PASSWORD_READ, 3, 1, PacketType::Response, with_session(1, &[2])),
        ]);
        let mut c = Client::new(mock);
        assert_eq!(c.access_read(None).unwrap(), Access::PasswordRequired);
    }

    #[test]
    fn write_path_allowlist() {
        assert!(is_writable_path("/defpara/Applications/PPS/u3db.db"));
        assert!(is_writable_path("/defpara/Applications/chvoicerecord/G4-Tone-100.dat"));
        assert!(!is_writable_path("/defpara/Applications/chvoicerecord/../../etc/passwd.dat"));
        assert!(!is_writable_path("/root/Applications/u3db/u3db.db"));
        assert!(!is_writable_path("/root/Applications/u3db/keytab"));
        let mut c = Client::new(pager(&[]));
        assert!(matches!(c.program_files(&[("/etc/passwd", b"x")], None), Err(Error::PathNotAllowed(_))));
        assert!(c.into_inner().tx.is_empty(), "nothing may be sent before the allowlist check");
    }

    #[test]
    fn program_files_sequence() {
        // 2049 bytes -> two Continue chunks (2048 + 1), then Complete.
        let data = vec![0xAB; MAX_CHUNK + 1];
        let s1 = 1; // start-write session
        let s2 = 2; // write-file session
        let s3 = 3; // end-business session
        let ack = |seq: u16| Packet::new(op::CONTINUE, 0, seq, PacketType::Ack, vec![]);
        let mock = pager(&[
            Packet::new(op::START_WRITE, 1, 1, PacketType::Response, with_session(s1, &[0])),
            Packet::new(op::WRITE_FILE, 2, 2, PacketType::Response, with_session(s2, &[0])),
            ack(3),
            ack(4),
            ack(5),
            Packet::new(op::END_BUSINESS, 3, 6, PacketType::Response, with_session(s3, &[0])),
        ]);
        let mut c = Client::new(mock);
        c.program_files(&[("/defpara/Applications/PPS/u3db.db", &data)], None).unwrap();

        // Decode everything the client sent.
        let tx = c.into_inner().tx;
        let mut sent = Vec::new();
        let mut i = 0;
        while i < tx.len() {
            let h = Header::decode(tx[i..i + HEADER_LEN].try_into().unwrap()).unwrap();
            let end = i + HEADER_LEN + if h.content_len == 0 { 0 } else { h.content_len + 2 };
            sent.push(h.into_packet(&tx[i + HEADER_LEN..end]).unwrap());
            i = end;
        }
        let kinds: Vec<(Opcode, PacketType)> = sent.iter().map(|p| (p.opcode, p.ptype)).collect();
        assert_eq!(kinds, vec![
            (op::START_WRITE, PacketType::Request),
            (op::START_WRITE, PacketType::Ack),
            (op::WRITE_FILE, PacketType::Request),
            (op::WRITE_FILE, PacketType::Ack),
            (op::CONTINUE, PacketType::Inform),
            (op::CONTINUE, PacketType::Inform),
            (op::COMPLETE, PacketType::Inform),
            (op::END_BUSINESS, PacketType::Request),
            (op::END_BUSINESS, PacketType::Ack),
        ]);
        assert_eq!(sent[0].body(true), &[0x01]);
        let params = sent[2].body(true);
        assert_eq!(&params[..4], &((MAX_CHUNK + 1) as u32).to_be_bytes());
        assert_eq!(&params[4..], &encode_string("/defpara/Applications/PPS/u3db.db")[..]);
        assert_eq!(sent[4].body(true).len(), MAX_CHUNK);
        assert_eq!(sent[5].body(true), &[0xAB]);
        assert_eq!(sent[6].body(true), &[0x00]);
        assert_eq!(sent[4].session(), Some(s2));
        assert_eq!(sent[7].body(true), &[0x00], "commit code");
    }

    #[test]
    fn session_mismatch_is_rejected() {
        let mock = pager(&[Packet::new(op::READ_PATH, 2, 1, PacketType::Response, with_session(9, &[0, 0, 0, 0, 0]))]);
        let mut c = Client::new(mock);
        assert!(matches!(c.list_dir("/x/"), Err(Error::SessionMismatch { got: 9, want: 1 })));
    }
}
