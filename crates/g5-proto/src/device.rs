//! Handshake response parsing (`HandsShakeInfo.AnalyzeHightDeviceInfo` in PPS).

use crate::Error;

#[derive(Debug, Clone, Default)]
pub struct DeviceInfo {
    pub product_id: u8,
    pub product_type: u8,
    pub business_id: u8,
    /// First 40 chars of the model/serial field, trimmed. `None` if the field wasn't 56 chars.
    pub model_number: Option<String>,
    pub serial_number: String,
    /// The pager reports something in this slot; PPS never uses it. Kept private.
    password_present: bool,
    pub radio_number: String,
    pub band_bits: [u8; 2],
    pub manufacture_date: String,
    pub sw_version: String,
    pub internal_version: String,
    pub u3db_version: String,
    pub protocol_version: String,
    pub hw_version: String,
    pub factory_parameter: String,
    pub protocol_mode: Option<u32>,
}

impl DeviceInfo {
    /// `content` is the handshake Response content (no session ID).
    pub fn parse(content: &[u8]) -> Result<Self, Error> {
        if content.len() < 4 {
            return Err(Error::Malformed("handshake response shorter than 4 bytes"));
        }
        let mut info = DeviceInfo {
            product_id: content[0],
            product_type: content[1],
            business_id: content[2],
            ..Default::default()
        };
        let mut r = Reader(&content[4..]);

        let model_serial = r.string()?;
        if model_serial.chars().count() == 56 {
            let (m, s) = model_serial.split_at(model_serial.char_indices().nth(40).unwrap().0);
            info.model_number = Some(m.trim_end_matches(['\0', ' ']).to_string());
            info.serial_number = s.trim_end_matches(['\0', ' ']).to_string();
        } else {
            info.serial_number = model_serial.trim_end_matches(['\0', ' ']).to_string();
        }
        info.password_present = !r.string().unwrap_or_default().is_empty();
        info.radio_number = r.string().unwrap_or_default();
        if let Some(b) = r.take(2) {
            info.band_bits = [b[0], b[1]];
        }
        info.manufacture_date = r.string().unwrap_or_default();
        let versions = r.string().unwrap_or_default();
        let parts: Vec<&str> = versions.split(':').collect();
        info.sw_version = parts.first().copied().unwrap_or_default().to_string();
        info.internal_version = parts.get(1).copied().unwrap_or(&info.sw_version).to_string();
        info.u3db_version = parts.get(2).copied().unwrap_or_default().to_string();
        info.protocol_version = parts.get(3).copied().unwrap_or_default().to_string();
        info.hw_version = r.string().unwrap_or_default();
        info.factory_parameter = r.string().unwrap_or_default();
        info.protocol_mode = r.take(4).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]));
        Ok(info)
    }

    pub fn password_present(&self) -> bool {
        self.password_present
    }

    /// PPS decodes the 2-byte band field this way (HandsShakeInfo).
    pub fn band_fields(&self) -> BandFields {
        let [b0, b1] = self.band_bits;
        BandFields {
            function_type: b0 >> 6,
            board_type: (b0 >> 2) & 0x0F,
            secondary_band: ((b0 & 3) << 3) | ((b1 >> 5) & 7),
            primary_band: b1 & 0x1F,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BandFields {
    pub function_type: u8,
    pub board_type: u8,
    pub secondary_band: u8,
    pub primary_band: u8,
}

struct Reader<'a>(&'a [u8]);

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Option<&[u8]> {
        if self.0.len() < n {
            return None;
        }
        let (a, b) = self.0.split_at(n);
        self.0 = b;
        Some(a)
    }

    /// `[len BE u32, counts trailing NUL][len-1 bytes][00]`; len 0 = empty.
    fn string(&mut self) -> Result<String, Error> {
        let len = self.take(4).ok_or(Error::Malformed("truncated string length"))?;
        let len = u32::from_be_bytes([len[0], len[1], len[2], len[3]]) as usize;
        if len == 0 {
            return Ok(String::new());
        }
        let bytes = self.take(len).ok_or(Error::Malformed("truncated string"))?;
        let bytes = bytes.strip_suffix(&[0]).unwrap_or(bytes);
        Ok(String::from_utf8_lossy(bytes).into_owned())
    }
}

/// Encode a string/path the way every file op expects it.
pub fn encode_string(s: &str) -> Vec<u8> {
    let mut v = ((s.len() + 1) as u32).to_be_bytes().to_vec();
    v.extend_from_slice(s.as_bytes());
    v.push(0);
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(s: &str) -> Vec<u8> {
        if s.is_empty() { vec![0, 0, 0, 0] } else { encode_string(s) }
    }

    #[test]
    fn parses_handshake() {
        let model = format!("{:<40}{:<16}", "G5H67AB-SXUDENBX00UNI", "1234567890ABCDEF");
        let mut c = vec![3, 1, 5, 0];
        c.extend(field(&model));
        c.extend(field(""));
        c.extend(field("42"));
        c.extend([0b0100_0101, 0b0110_0011]);
        c.extend(field("20240101"));
        c.extend(field("V1.2.3:I5:DBV2.06:7"));
        c.extend(field("HW1"));
        c.extend(field("FP"));
        c.extend(0xFFFF_FFFFu32.to_be_bytes());

        let d = DeviceInfo::parse(&c).unwrap();
        assert_eq!(d.model_number.as_deref(), Some("G5H67AB-SXUDENBX00UNI"));
        assert_eq!(d.serial_number, "1234567890ABCDEF");
        assert!(!d.password_present());
        assert_eq!(d.radio_number, "42");
        assert_eq!(d.u3db_version, "DBV2.06");
        assert_eq!(d.protocol_version, "7");
        assert_eq!(d.hw_version, "HW1");
        assert_eq!(d.protocol_mode, Some(0xFFFF_FFFF));
        let bf = d.band_fields();
        assert_eq!((bf.function_type, bf.board_type, bf.primary_band), (1, 1, 3));
    }

    #[test]
    fn encodes_path() {
        assert_eq!(encode_string("/a/"), vec![0, 0, 0, 4, b'/', b'a', b'/', 0]);
    }
}
