//! The `-vv` wire trace of a real handshake and access check, as the pager
//! sends them (handshake response under COMMAND, password second), must
//! contain neither the access password nor its MD5.
//!
//! This is its own test binary on purpose: it installs a global tracing
//! subscriber before any trace callsite runs. Run alongside other tests,
//! callsites registered on other threads could cache "no interest" and the
//! trace would come out empty.

use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex};

use g5_proto::Access;
use g5_proto::client::{Client, Transport};
use g5_proto::device::encode_string;
use g5_proto::frame::{Packet, PacketType, hex, op};

/// Feeds scripted pager bytes and discards what the client sends.
struct Pager(VecDeque<u8>);

impl Read for Pager {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = buf.len().min(self.0.len());
        for (b, v) in buf.iter_mut().zip(self.0.drain(..n)) {
            *b = v;
        }
        Ok(n)
    }
}

impl Write for Pager {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Transport for Pager {}

#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl Write for Captured {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn wire_trace_never_shows_the_password() {
    let log = Captured::default();
    let writer = log.clone();
    tracing::subscriber::set_global_default(
        tracing_subscriber::fmt().with_max_level(tracing::Level::TRACE).with_ansi(false).with_writer(move || writer.clone()).finish(),
    )
    .unwrap();

    let password = "hunter2-secret";
    let hash = "0123456789ABCDEF0123456789ABCDEF";
    let mut info = vec![3, 1, 5, 0];
    info.extend(encode_string(&format!("{:<40}{:<16}", "G5N66AK-SXVXENBX00UNI", "SERIAL")));
    info.extend(encode_string(password));
    let mut access = 1u16.to_be_bytes().to_vec();
    access.push(0);
    let bytes = [
        Packet::new(op::COMMAND, 7, 1, PacketType::Response, info),
        Packet::new(op::ACCESS_PASSWORD_READ, 8, 2, PacketType::Response, access),
    ]
    .iter()
    .flat_map(Packet::encode)
    .collect();

    let mut c = Client::new(Pager(bytes));
    assert!(c.handshake().unwrap().password_present());
    assert!(matches!(c.access_read(Some(hash)).unwrap(), Access::Granted(0)));

    let out = String::from_utf8(log.0.lock().unwrap().clone()).unwrap();
    assert!(out.contains("redacted"), "{out}");
    for secret in [password.as_bytes(), hash.as_bytes()] {
        assert!(!out.contains(&hex(&secret[..4])), "{out}");
    }
}
