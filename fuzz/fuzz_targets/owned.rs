#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(document) = adf::parse_bytes(data) {
        let _ = document.into_owned().into_adf();
    }
});
