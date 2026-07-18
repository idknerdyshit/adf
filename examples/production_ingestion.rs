use adf::{AdfDocument, ParseOptions, parse_reader_with};
use std::io::{self, Read};

// Illustrative application policy; tune these ceilings for the expected feed.
const MAX_INPUT_BYTES: usize = 2 * 1024 * 1024;
const MAX_DEPTH: usize = 32;
const MAX_NODES: usize = 20_000;
const MAX_ATTRIBUTES_PER_ELEMENT: usize = 32;
const MAX_DOCTYPE_BYTES: usize = 1_024;

fn ingest<R: Read>(reader: R) -> adf::Result<AdfDocument<'static>> {
    let options = ParseOptions::default()
        .max_input_len(MAX_INPUT_BYTES)
        .max_depth(MAX_DEPTH)
        .max_nodes(MAX_NODES)
        .max_attributes_per_element(MAX_ATTRIBUTES_PER_ELEMENT)
        .max_doctype_len(MAX_DOCTYPE_BYTES);

    parse_reader_with(reader, &options)
}

fn main() -> adf::Result<()> {
    let stdin = io::stdin();
    ingest(stdin.lock())?;
    Ok(())
}
