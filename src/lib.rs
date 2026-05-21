//! Lightweight ADF-XML parsing and writing.
//!
//! The crate exposes a domain model for ADF 1.0 while retaining the original
//! input for byte-for-byte output when a document has not been rewritten.

mod document;
mod error;
mod model;
mod parse;
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
