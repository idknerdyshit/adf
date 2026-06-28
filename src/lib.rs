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
pub use parse::{DEFAULT_MAX_DOCTYPE_LEN, ParseOptions};
pub use validate::{
    Severity, ValidationIssue, ValidationOptions, ValidationReport, validate, validate_with,
};

/// Parse an ADF-XML document with the default [`ParseOptions`].
///
/// Inputs must be well-formed XML. ADF-specific validation is intentionally
/// separate and can be requested through [`AdfDocument::validate`].
///
/// External and custom entities are never resolved or expanded: the parser
/// only substitutes the five predefined XML entities and numeric character
/// references, leaving any other entity reference intact. By default a
/// `<!DOCTYPE …>` declaration is preserved but its internal subset is capped at
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
