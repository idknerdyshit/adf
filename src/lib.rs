//! Lightweight ADF-XML parsing and writing.
//!
//! The crate exposes a domain model for ADF 1.0 while retaining the original
//! input for byte-for-byte output when a document has not been rewritten.
//!
//! # Tracing and privacy
//!
//! This crate emits passive [`tracing`] spans and events around parsing,
//! validation, and writing. It does not install a subscriber; applications
//! choose how to collect or ignore those events.
//!
//! Trace fields intentionally contain only structural metadata such as byte
//! counts, model counts, dirty flags, validation issue counts, parse options,
//! and error categories/positions. They do not include raw XML, element text,
//! attribute values, validation messages, names, emails, phone numbers,
//! addresses, identifiers, URLs, comments, or extension payloads.
//!
//! The public model and [`AdfDocument::original`] still expose lead payloads;
//! avoid logging those values directly when handling sensitive data.

mod document;
mod error;
mod model;
mod parse;
mod trace;
mod validate;
mod write;

pub use document::{AdfDocument, Attribute, Span, XmlElement, XmlNode};
pub use error::{Error, Result};
pub use model::*;
pub use parse::{
    DEFAULT_MAX_ATTRIBUTES_PER_ELEMENT, DEFAULT_MAX_DEPTH, DEFAULT_MAX_DOCTYPE_LEN,
    DEFAULT_MAX_INPUT_LEN, DEFAULT_MAX_NODES, ParseLimit, ParseOptions,
};
pub use validate::{
    Severity, ValidationCode, ValidationIssue, ValidationOptions, ValidationProfile,
    ValidationReport, validate, validate_adf_1_0, validate_adf_1_0_extended, validate_with,
};
pub use write::{UnknownEntityPolicy, WriteOptions};

/// Parse an ADF-XML document with the default [`ParseOptions`].
///
/// Inputs must be well-formed XML rooted at `<adf>`. Other ADF-specific
/// validation is intentionally separate and can be requested through
/// [`AdfDocument::validate`].
///
/// External and custom entities are never resolved or expanded: the parser
/// only substitutes the five predefined XML entities and legal numeric
/// character references. Unknown entity references in text are retained as
/// [`TextPart::EntityRef`]; unknown entity references in attributes are kept as
/// literal `&name;` text because [`Attribute`] stores a flat string. By default
/// a `<!DOCTYPE …>` declaration is preserved but its payload is capped at
/// [`DEFAULT_MAX_DOCTYPE_LEN`] bytes; use [`parse_with`] to reject DOCTYPEs
/// outright or to change the limit.
pub fn parse(input: &str) -> Result<AdfDocument<'_>> {
    parse::parse(input)
}

/// Parse an ADF-XML document with explicit [`ParseOptions`].
///
/// See [`parse`] for the entity-handling guarantees that apply regardless of
/// options.
pub fn parse_with<'a>(input: &'a str, options: &ParseOptions) -> Result<AdfDocument<'a>> {
    parse::parse_with(input, options)
}

/// Parse an owned UTF-8 ADF document without retaining a borrow of the input.
pub fn parse_owned(input: String) -> Result<AdfDocument<'static>> {
    parse::parse_owned(input, &ParseOptions::default())
}

/// Parse owned UTF-8 bytes with default parser options.
pub fn parse_bytes(input: &[u8]) -> Result<AdfDocument<'static>> {
    parse::parse_bytes(input, &ParseOptions::default())
}

/// Parse owned UTF-8 bytes with explicit parser options.
pub fn parse_bytes_with(input: &[u8], options: &ParseOptions) -> Result<AdfDocument<'static>> {
    parse::parse_bytes(input, options)
}

/// Read and parse an owned UTF-8 ADF document with default parser options.
pub fn parse_reader<R: std::io::Read>(reader: R) -> Result<AdfDocument<'static>> {
    parse::parse_reader(reader, &ParseOptions::default())
}

/// Read and parse an owned UTF-8 ADF document with explicit parser options.
pub fn parse_reader_with<R: std::io::Read>(
    reader: R,
    options: &ParseOptions,
) -> Result<AdfDocument<'static>> {
    parse::parse_reader(reader, options)
}

/// Write a typed ADF model using canonical output defaults.
pub fn write<W: std::io::Write>(writer: W, adf: &Adf<'_>) -> Result<()> {
    write::write_adf(writer, adf)
}

/// Write a typed ADF model using explicit output options.
pub fn write_with<W: std::io::Write>(
    writer: W,
    adf: &Adf<'_>,
    options: &WriteOptions,
) -> Result<()> {
    write::write_adf_with(writer, adf, options)
}

/// Return canonical typed ADF XML as a string.
pub fn to_string(adf: &Adf<'_>) -> Result<String> {
    to_string_with(adf, &WriteOptions::default())
}

/// Return typed ADF XML using explicit output options.
pub fn to_string_with(adf: &Adf<'_>, options: &WriteOptions) -> Result<String> {
    let mut output = Vec::new();
    write_with(&mut output, adf, options)?;
    Ok(String::from_utf8(output).expect("ADF writer only emits UTF-8"))
}
