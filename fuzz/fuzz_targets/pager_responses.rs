//! Response contents from the pager: the handshake reply (device info) and
//! directory listings.
#![no_main]

use g5_proto::DeviceInfo;
use g5_proto::client::parse_listing;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(info) = DeviceInfo::parse(data) {
        let _ = info.password_present();
        let _ = info.band_fields();
    }
    let _ = parse_listing(data);
});
