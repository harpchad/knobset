//! A pager that speaks the wire protocol on 127.0.0.1, for testing the
//! guarded write flow end to end. It serves files from memory, stages
//! writes inside a write business and applies them on commit, and can be
//! told to misbehave.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use g5_proto::frame::{HEADER_LEN, Header, Opcode, Packet, PacketType, op};
use g5_proto::device::encode_string;

pub const MODEL: &str = "G5D66BE-SXVXENB702UNI-A001-220104";
pub const SERIAL: &str = "G5TEST0001";
pub const DB_VERSION: &str = "DBV1.41e";

#[derive(Default)]
pub struct State {
    /// Full pager path to contents.
    pub files: BTreeMap<String, Vec<u8>>,
    pub serial: String,
    /// Extra listing entries for `/defpara/Applications/PPS/`.
    pub extra_listing: Vec<String>,
    /// Answer WRITE_FILE for this path with a nonzero status.
    pub fail_write: Option<String>,
    /// Flip a byte of every file applied by a commit.
    pub corrupt_commit: bool,
    /// Serial to report after a commit (a pager swapped during the reboot).
    pub serial_after_commit: Option<String>,
    pub write_businesses: usize,
    pub commits: usize,
    pub aborts: usize,
    /// Paths requested with READ_FILE.
    pub reads: Vec<String>,
    staged: BTreeMap<String, Vec<u8>>,
    pending: Option<(String, Vec<u8>)>,
    writing: bool,
}

pub struct FakePager {
    pub addr: SocketAddr,
    pub state: Arc<Mutex<State>>,
}

impl FakePager {
    pub fn start(files: BTreeMap<String, Vec<u8>>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let state = Arc::new(Mutex::new(State { files, serial: SERIAL.into(), ..Default::default() }));
        let s = state.clone();
        thread::spawn(move || {
            for conn in listener.incoming() {
                let Ok(conn) = conn else { return };
                let s = s.clone();
                thread::spawn(move || {
                    let _ = serve(conn, &s);
                });
            }
        });
        Self { addr, state }
    }

    pub fn with<R>(&self, f: impl FnOnce(&mut State) -> R) -> R {
        f(&mut self.state.lock().unwrap())
    }
}

struct Conn {
    io: TcpStream,
    seq: u16,
}

impl Conn {
    fn send(&mut self, opcode: Opcode, ptype: PacketType, ack_seq: u16, content: Vec<u8>) -> std::io::Result<()> {
        let seq = if ptype == PacketType::Ack {
            0
        } else {
            self.seq += 1;
            self.seq
        };
        self.io.write_all(&Packet::new(opcode, seq, ack_seq, ptype, content).encode())
    }

    fn recv(&mut self) -> Option<Packet> {
        let mut h = [0u8; HEADER_LEN];
        self.io.read_exact(&mut h).ok()?;
        let header = Header::decode(&h).ok()?;
        let mut tail = vec![0u8; if header.content_len == 0 { 0 } else { header.content_len + 2 }];
        self.io.read_exact(&mut tail).ok()?;
        header.into_packet(&tail).ok()
    }

    /// A Read's Response, data chunks and Complete.
    fn stream(&mut self, opcode: Opcode, p: &Packet, session: &[u8], data: &[u8]) -> std::io::Result<()> {
        let mut status = session.to_vec();
        status.push(0);
        status.extend((data.len() as u32).to_be_bytes());
        self.send(opcode, PacketType::Response, p.seq, status)?;
        for chunk in data.chunks(g5_proto::client::MAX_CHUNK) {
            self.send(op::CONTINUE, PacketType::Inform, 0, [session, chunk].concat())?;
        }
        self.send(op::COMPLETE, PacketType::Inform, 0, [session, &[0]].concat())
    }
}

fn decode_string(b: &[u8]) -> String {
    let n = u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize;
    let s = &b[4..4 + n];
    String::from_utf8_lossy(s.strip_suffix(&[0]).unwrap_or(s)).into_owned()
}

fn handshake(serial: &str) -> Vec<u8> {
    let mut c = vec![3, 1, 5, 0];
    c.extend(encode_string(&format!("{MODEL:<40}{serial:<16}")));
    c.extend(encode_string("")); // password slot
    c.extend(encode_string("1"));
    c.extend([0, 0]);
    c.extend(encode_string(""));
    c.extend(encode_string(&format!("V1.33:V1.33:{DB_VERSION}:1")));
    c
}

fn listing(state: &State, dir: &str) -> Vec<u8> {
    let mut names: Vec<(String, u32)> =
        state.files.iter().filter_map(|(p, d)| p.strip_prefix(dir).filter(|n| !n.contains('/')).map(|n| (n.to_string(), d.len() as u32))).collect();
    if dir == "/defpara/Applications/PPS/" {
        names.extend(state.extra_listing.iter().map(|n| (n.clone(), 4)));
    }
    let mut out = Vec::new();
    for (n, size) in names {
        out.extend(encode_string(&n));
        out.extend(size.to_be_bytes());
    }
    out
}

fn serve(io: TcpStream, state: &Mutex<State>) -> std::io::Result<()> {
    let mut c = Conn { io, seq: 0 };
    while let Some(p) = c.recv() {
        match (p.ptype, p.opcode) {
            (PacketType::Ack | PacketType::Nak, _) => {}
            (PacketType::Request, op::HANDSHAKE) => {
                let serial = state.lock().unwrap().serial.clone();
                c.send(op::COMMAND, PacketType::Response, p.seq, handshake(&serial))?;
            }
            (PacketType::Request, op::BYEBYE) => return Ok(()),
            (PacketType::Request, op::SET_DATE_TIME) => c.send(op::SET_DATE_TIME, PacketType::Response, p.seq, vec![0, 0, 0])?,
            (PacketType::Request, opcode) => {
                let (session, body) = p.content.split_at(2);
                let reply = |c: &mut Conn, rc: u8| c.send(opcode, PacketType::Response, p.seq, [session, &[rc]].concat());
                match opcode {
                    op::ACCESS_PASSWORD_READ | op::ACCESS_PASSWORD_WRITE => reply(&mut c, 1)?,
                    op::START_READ => reply(&mut c, 0)?,
                    op::START_WRITE => {
                        let mut s = state.lock().unwrap();
                        s.write_businesses += 1;
                        s.writing = true;
                        s.staged.clear();
                        drop(s);
                        reply(&mut c, 0)?;
                    }
                    op::END_BUSINESS => {
                        let mut s = state.lock().unwrap();
                        if std::mem::take(&mut s.writing) {
                            let staged = std::mem::take(&mut s.staged);
                            if body == [0] {
                                s.commits += 1;
                                for (path, mut data) in staged {
                                    if s.corrupt_commit {
                                        data[0] ^= 0xFF;
                                    }
                                    s.files.insert(path, data);
                                }
                                if let Some(serial) = s.serial_after_commit.take() {
                                    s.serial = serial;
                                }
                            } else {
                                s.aborts += 1;
                            }
                        }
                        drop(s);
                        reply(&mut c, 0)?;
                    }
                    op::READ_PATH => {
                        let data = listing(&state.lock().unwrap(), &decode_string(body));
                        c.stream(opcode, &p, session, &data)?;
                    }
                    op::READ_FILE => {
                        let path = decode_string(body);
                        let mut s = state.lock().unwrap();
                        s.reads.push(path.clone());
                        let data = s.files.get(&path).cloned();
                        drop(s);
                        match data {
                            Some(d) => c.stream(opcode, &p, session, &d)?,
                            None => reply(&mut c, 3)?,
                        }
                    }
                    op::WRITE_FILE => {
                        let path = decode_string(&body[4..]);
                        let mut s = state.lock().unwrap();
                        let fail = s.fail_write.as_deref() == Some(path.as_str());
                        if !fail {
                            s.pending = Some((path, Vec::new()));
                        }
                        drop(s);
                        reply(&mut c, if fail { 4 } else { 0 })?;
                    }
                    _ => reply(&mut c, 0xFF)?,
                }
            }
            (PacketType::Inform, op::CONTINUE) => {
                if let Some((_, buf)) = state.lock().unwrap().pending.as_mut() {
                    buf.extend_from_slice(&p.content[2..]);
                }
                c.send(op::CONTINUE, PacketType::Ack, p.seq, Vec::new())?;
            }
            (PacketType::Inform, op::COMPLETE) => {
                let mut s = state.lock().unwrap();
                if let Some((path, data)) = s.pending.take() {
                    s.staged.insert(path, data);
                }
                drop(s);
                c.send(op::COMPLETE, PacketType::Ack, p.seq, Vec::new())?;
            }
            _ => {}
        }
    }
    Ok(())
}
