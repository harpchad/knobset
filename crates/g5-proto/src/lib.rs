//! Wire protocol for Unication G-series pagers.
//!
//! The pager enumerates as a USB network device (CDC-ECM on configuration 1,
//! RNDIS on configuration 2), serves DHCP on 10.10.0.0/16, and listens on
//! TCP 10.10.0.12:32769. See `notes/protocol.md` for the byte-level spec.

pub mod client;
pub mod device;
pub mod frame;

pub use client::{Access, Client, DEFAULT_ADDR, DirEntry, WRITABLE_PATHS, is_writable_path};
pub use device::DeviceInfo;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("i/o: {0}")]
    Io(#[from] std::io::Error),
    #[error("timed out waiting for the pager")]
    Timeout,
    #[error("pager closed the connection")]
    Disconnected,
    #[error("header checksum mismatch (got {got:#04x}, want {want:#04x})")]
    Checksum { got: u8, want: u8 },
    #[error("content CRC mismatch (got {got:#06x}, want {want:#06x})")]
    Crc { got: u16, want: u16 },
    #[error("unknown packet type {0}")]
    BadPacketType(u8),
    #[error("session ID mismatch (got {got}, want {want})")]
    SessionMismatch { got: u16, want: u16 },
    #[error("pager sent NAK (state {0})")]
    Nak(u8),
    #[error("pager aborted the operation (code {0:?})")]
    Abort(Option<u8>),
    #[error("pager cancelled the operation")]
    Cancelled,
    #[error("unexpected {0:?} packet")]
    Unexpected(frame::PacketType),
    #[error("pager reported device status {0} (4 = locked)")]
    DeviceStatus(u8),
    #[error("pager refused to start the business (code {0})")]
    BusinessRefused(u8),
    #[error("unexpected access-check code {0}")]
    AccessCode(u8),
    #[error("short read: got {got} of {want} bytes")]
    ShortRead { got: usize, want: usize },
    #[error("refusing to write {0}: not a path PPS programs")]
    PathNotAllowed(String),
    #[error("pager refused to commit the write (code {0})")]
    CommitRefused(u8),
    #[error("malformed data: {0}")]
    Malformed(&'static str),
}
