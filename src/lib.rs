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
pub use validate::{
    Severity, ValidationIssue, ValidationOptions, ValidationReport, validate, validate_with,
};

/// Parse an ADF-XML document.
///
/// Inputs must be well-formed XML. ADF-specific validation is intentionally
/// separate and can be requested through [`AdfDocument::validate`].
pub fn parse(input: &str) -> Result<AdfDocument<'_>> {
    parse::parse(input)
}
