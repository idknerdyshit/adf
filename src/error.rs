use std::fmt;

/// Crate result type.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors returned by parsing, validation-adjacent parsing checks, and writing.
#[derive(Debug)]
pub enum Error {
    /// XML syntax or well-formedness error reported by the XML reader.
    Xml {
        position: u64,
        source: quick_xml::Error,
    },
    /// XML attribute syntax error reported while reading a start tag.
    Attribute {
        position: u64,
        source: quick_xml::events::attributes::AttrError,
    },
    /// Character encoding error while decoding XML event content.
    Encoding {
        position: u64,
        source: quick_xml::encoding::EncodingError,
    },
    /// UTF-8 decoding error for XML names or event payloads.
    Utf8 {
        position: u64,
        source: std::str::Utf8Error,
    },
    /// Closing tag did not match the currently open element.
    MismatchedEnd {
        expected: String,
        found: String,
        position: u64,
    },
    /// Closing tag appeared without a matching open element.
    UnexpectedEnd { name: String, position: u64 },
    /// Non-whitespace content appeared before or after the document root.
    ContentOutsideRoot { position: u64 },
    /// XML root element was not `<adf>`.
    UnexpectedRoot { found: String, position: u64 },
    /// A numeric character reference could not be decoded to an XML character.
    InvalidCharacterReference { reference: String, position: u64 },
    /// An entity reference was malformed.
    InvalidEntityReference { reference: String, position: u64 },
    /// XML text contained a Unicode scalar value that XML does not permit.
    IllegalCharacter { character: char, position: u64 },
    /// XML name token was invalid for the context.
    InvalidName { kind: &'static str },
    /// Caller-constructed raw XML node content was not syntactically valid.
    InvalidXmlToken {
        kind: &'static str,
        reason: &'static str,
    },
    /// A DOCTYPE declaration was rejected by [`crate::ParseOptions`].
    DocTypeForbidden { position: u64 },
    /// A DOCTYPE declaration exceeded the configured byte limit.
    DocTypeTooLong {
        length: usize,
        limit: usize,
        position: u64,
    },
    /// No XML root element was found.
    MissingRoot,
    /// More than one XML root element was found.
    MultipleRoots,
    /// Error returned by an output writer.
    Io(std::io::Error),
}

impl Error {
    pub(crate) fn xml(position: u64, source: quick_xml::Error) -> Self {
        Self::Xml { position, source }
    }

    pub(crate) fn encoding(position: u64, source: quick_xml::encoding::EncodingError) -> Self {
        Self::Encoding { position, source }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Xml { position, source } => {
                write!(f, "XML error at byte {position}: {source}")
            }
            Error::Attribute { position, source } => {
                write!(f, "XML attribute error at byte {position}: {source}")
            }
            Error::Encoding { position, source } => {
                write!(f, "XML encoding error at byte {position}: {source}")
            }
            Error::Utf8 { position, source } => {
                write!(f, "UTF-8 error at byte {position}: {source}")
            }
            Error::MismatchedEnd {
                expected,
                found,
                position,
            } => write!(
                f,
                "mismatched closing tag at byte {position}: expected </{expected}>, found </{found}>"
            ),
            Error::UnexpectedEnd { name, position } => {
                write!(f, "unexpected closing tag </{name}> at byte {position}")
            }
            Error::ContentOutsideRoot { position } => {
                write!(
                    f,
                    "non-document content outside the root element at byte {position}"
                )
            }
            Error::UnexpectedRoot { found, position } => {
                write!(
                    f,
                    "unexpected root element <{found}> at byte {position}; expected <adf>"
                )
            }
            Error::InvalidCharacterReference {
                reference,
                position,
            } => {
                write!(
                    f,
                    "invalid XML character reference &{reference}; at byte {position}"
                )
            }
            Error::InvalidEntityReference {
                reference,
                position,
            } => {
                write!(
                    f,
                    "invalid XML entity reference &{reference}; at byte {position}"
                )
            }
            Error::IllegalCharacter {
                character,
                position,
            } => write!(
                f,
                "illegal XML character U+{:04X} at byte {position}",
                *character as u32
            ),
            Error::InvalidName { kind } => write!(f, "invalid XML {kind} name"),
            Error::InvalidXmlToken { kind, reason } => {
                write!(f, "invalid XML {kind}: {reason}")
            }
            Error::DocTypeForbidden { position } => {
                write!(f, "DOCTYPE declaration is not allowed at byte {position}")
            }
            Error::DocTypeTooLong {
                length,
                limit,
                position,
            } => write!(
                f,
                "DOCTYPE declaration of {length} bytes exceeds the limit of {limit} bytes at byte {position}"
            ),
            Error::MissingRoot => f.write_str("document does not contain a root element"),
            Error::MultipleRoots => f.write_str("document contains more than one root element"),
            Error::Io(source) => write!(f, "I/O error: {source}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Xml { source, .. } => Some(source),
            Error::Attribute { source, .. } => Some(source),
            Error::Encoding { source, .. } => Some(source),
            Error::Utf8 { source, .. } => Some(source),
            Error::Io(source) => Some(source),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(source: std::io::Error) -> Self {
        Self::Io(source)
    }
}
