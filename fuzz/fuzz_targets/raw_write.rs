#![no_main]

use adf::{Adf, Prospect, XmlElement, XmlNode};
use libfuzzer_sys::fuzz_target;
use std::borrow::Cow;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let extension = XmlNode::Element(XmlElement {
        name: Cow::Borrowed("partner"),
        attributes: Vec::new(),
        children: vec![XmlNode::Text(Cow::Borrowed(text))],
        span: adf::Span::default(),
    });
    let model = Adf::builder(Prospect::default())
        .extension(extension)
        .build();
    let _ = adf::to_string(&model);
});
