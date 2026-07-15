#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(document) = adf::parse(input) else {
        return;
    };
    let Ok(xml) = document.to_typed_string() else {
        return;
    };
    adf::parse(&xml).expect("typed output from a parsed document must reparse");
});
