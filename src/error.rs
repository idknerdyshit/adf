use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Xml {
        position: u64,
        source: quick_xml::Error,
    },
    Attribute {
        position: u64,
        source: quick_xml::events::attributes::AttrError,
    },
    Encoding {
        position: u64,
        source: quick_xml::encoding::EncodingError,
    },
    Utf8 {
        position: u64,
        source: std::str::Utf8Error,
    },
    MismatchedEnd {
        expected: String,
        found: String,
        position: u64,
    },
    UnexpectedEnd {
        name: String,
        position: u64,
    },
    ContentOutsideRoot {
        position: u64,
    },
    InvalidCharacterReference {
        reference: String,
        position: u64,
    },
    MissingRoot,
    MultipleRoots,
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
            Error::InvalidCharacterReference {
                reference,
                position,
            } => {
                write!(
                    f,
                    "invalid XML character reference &{reference}; at byte {position}"
                )
            }
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
