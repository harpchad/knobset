//! Connecting to a pager and describing it.

use std::fmt::Write as _;
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use g5_proto::{Access, Client, DeviceInfo};
use md5::{Digest, Md5};
use serde::Serialize;

pub type Pager = Client<TcpStream>;

/// What the UI shows about a connected pager. Never includes the password slot's contents.
#[derive(Debug, Clone, Serialize)]
pub struct DeviceSummary {
    pub model: String,
    pub serial: String,
    pub radio_number: String,
    pub sw_version: String,
    pub internal_version: String,
    pub db_version: String,
    pub hw_version: String,
    /// True when this tool can generate the pager's radio DB.
    pub db_supported: bool,
}

impl From<&DeviceInfo> for DeviceSummary {
    fn from(d: &DeviceInfo) -> Self {
        Self {
            model: d.model_number.clone().unwrap_or_default(),
            serial: d.serial_number.clone(),
            radio_number: d.radio_number.clone(),
            sw_version: d.sw_version.clone(),
            internal_version: d.internal_version.clone(),
            db_version: d.u3db_version.clone(),
            hw_version: d.hw_version.clone(),
            db_supported: g5_codeplug::convert141::Variant::for_db_version(&d.u3db_version).is_some(),
        }
    }
}

/// Connect and handshake.
pub fn connect(addr: SocketAddr) -> Result<(Pager, DeviceInfo)> {
    let mut client = Client::connect(addr, Duration::from_secs(3))
        .with_context(|| format!("connecting to {addr} (is the pager plugged in and its network interface up?)"))?;
    let info = client.handshake().context("handshake")?;
    Ok((client, info))
}

/// Whether the pager accepts a TCP connection, without a handshake. The pager shows
/// "PC Connected" and stops scanning on a handshake; a bare connect and close does
/// neither (checked on pager B, 2026-10-20), so this is safe to poll.
pub fn reachable(addr: SocketAddr) -> bool {
    TcpStream::connect_timeout(&addr, Duration::from_millis(1500)).is_ok()
}

pub fn hex_upper(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect()
}

/// PPS sends `MD5(password)` as 32 uppercase hex characters.
pub fn password_hash(pw: &str) -> String {
    hex_upper(&Md5::digest(pw.trim().as_bytes()))
}

/// Pass the read-access check (empty password first, then the given one).
pub fn authorize_read(client: &mut Pager, password: Option<&str>) -> Result<()> {
    if let Access::Granted(_) = client.access_read(None).context("read access check")? {
        return Ok(());
    }
    let Some(pw) = password else { bail!("the pager requires an access password") };
    match client.access_read(Some(&password_hash(pw))).context("read access check with password")? {
        Access::Granted(_) => Ok(()),
        Access::PasswordRequired => bail!("the pager rejected the password"),
    }
}

/// Pass the write-access check.
pub fn authorize_write(client: &mut Pager, password: Option<&str>) -> Result<()> {
    if let Access::Granted(_) = client.access_write(None).context("write access check")? {
        return Ok(());
    }
    let Some(pw) = password else { bail!("the pager requires an access password for writing") };
    match client.access_write(Some(&password_hash(pw))).context("write access check with password")? {
        Access::Granted(_) => Ok(()),
        Access::PasswordRequired => bail!("the pager rejected the password for writing"),
    }
}

/// Human-readable device description (also the head of every backup MANIFEST).
pub fn describe(d: &DeviceInfo) -> String {
    let mut s = String::new();
    let b = d.band_fields();
    let _ = writeln!(s, "Model:            {}", d.model_number.as_deref().unwrap_or("<none>"));
    let _ = writeln!(s, "Serial:           {}", d.serial_number);
    let _ = writeln!(s, "Radio number:     {}", d.radio_number);
    let _ = writeln!(s, "Software:         {}", d.sw_version);
    let _ = writeln!(s, "Internal version: {}", d.internal_version);
    let _ = writeln!(s, "Radio DB version: {}", d.u3db_version);
    let _ = writeln!(s, "Protocol version: {}", d.protocol_version);
    let _ = writeln!(s, "Hardware:         {}", d.hw_version);
    let _ = writeln!(s, "Manufactured:     {}", d.manufacture_date);
    let _ = writeln!(s, "Factory param:    {}", d.factory_parameter);
    let _ = writeln!(
        s,
        "Band bits:        {:02x} {:02x} (function {}, board {}, primary {}, secondary {})",
        d.band_bits[0], d.band_bits[1], b.function_type, b.board_type, b.primary_band, b.secondary_band
    );
    let _ = writeln!(s, "Protocol mode:    {}", d.protocol_mode.map(|m| format!("{m:#010x}")).unwrap_or_else(|| "-".into()));
    let _ = writeln!(s, "Product/type/biz: {} / {} / {}", d.product_id, d.product_type, d.business_id);
    let _ = writeln!(s, "Password slot:    {}", if d.password_present() { "set" } else { "empty" });
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_hash_is_uppercase_md5() {
        // RFC 1321 test vectors.
        assert_eq!(password_hash(""), "D41D8CD98F00B204E9800998ECF8427E");
        assert_eq!(password_hash(" abc\n"), "900150983CD24FB0D6963F7D28E17F72");
    }
}
