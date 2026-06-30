use crate::model::{Adf, Prospect};
use crate::{ParseOptions, Result, validate};
use std::borrow::Cow;
use std::cell::OnceCell;
use std::io::Write;
use std::ops::Range;

/// Byte span into the original XML input.
///
/// A default span (`0..0`) means the value was constructed by the caller or no
/// source location is available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    /// Start byte offset in the original XML input.
    pub start: usize,
    /// End byte offset in the original XML input.
    pub end: usize,
}

/// XML attribute name/value pair preserved from input or built by callers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute<'a> {
    /// Attribute name, including any namespace prefix as written.
    pub name: Cow<'a, str>,
    /// Decoded attribute value.
    ///
    /// Standard XML entities and character references are decoded. Unknown
    /// entity references are preserved as literal `&name;` text because the
    /// public attribute model has no separate entity-reference variant.
    pub value: Cow<'a, str>,
}

/// Raw XML element used for extension content and [`AdfDocument::root`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlElement<'a> {
    /// Element name, including any namespace prefix as written.
    pub name: Cow<'a, str>,
    /// Element attributes.
    pub attributes: Vec<Attribute<'a>>,
    /// Raw child nodes.
    pub children: Vec<XmlNode<'a>>,
    /// Byte span covering the complete element in the original input.
    pub span: Span,
}

/// Raw XML node retained for extension and mixed-content round-tripping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XmlNode<'a> {
    /// Nested XML element.
    Element(XmlElement<'a>),
    /// Text content with standard XML entities decoded.
    Text(Cow<'a, str>),
    /// CDATA content without the wrapper.
    CData(Cow<'a, str>),
    /// Unresolved named entity reference, stored without `&` and `;`.
    EntityRef(Cow<'a, str>),
    /// XML comment content without `<!--` and `-->`.
    Comment(Cow<'a, str>),
    /// Processing-instruction content without `<?` and `?>`.
    ProcessingInstruction(Cow<'a, str>),
    /// XML declaration content without `<?` and `?>`.
    Declaration(Cow<'a, str>),
    /// DOCTYPE payload without `<!DOCTYPE` and `>`.
    DocType(Cow<'a, str>),
}

/// Parsed ADF document plus original-input preservation state.
///
/// Mutating through [`AdfDocument::prospect_mut`] marks a single prospect dirty
/// so original-preserving writes can replace only that byte span. Mutating
/// through [`AdfDocument::adf_mut`] marks the whole document dirty and causes
/// original-preserving writes to fall back to normalized typed output.
#[derive(Debug, Clone)]
pub struct AdfDocument<'a> {
    pub(crate) original: &'a str,
    pub(crate) parse_options: ParseOptions,
    pub(crate) raw_root: OnceCell<XmlElement<'a>>,
    pub(crate) adf: Adf<'a>,
    pub(crate) prospect_spans: Vec<Range<usize>>,
    pub(crate) dirty_prospects: Vec<bool>,
    pub(crate) dirty_all: bool,
}

impl<'a> AdfDocument<'a> {
    pub(crate) fn new(
        original: &'a str,
        parse_options: ParseOptions,
        adf: Adf<'a>,
        prospect_spans: Vec<Range<usize>>,
    ) -> Self {
        let dirty_prospects = vec![false; prospect_spans.len()];
        Self {
            original,
            parse_options,
            raw_root: OnceCell::new(),
            adf,
            prospect_spans,
            dirty_prospects,
            dirty_all: false,
        }
    }

    /// Return the exact XML input used to create this document.
    pub fn original(&self) -> &'a str {
        self.original
    }

    /// Return the raw XML root, reparsing the original input on first access.
    ///
    /// The typed model is built eagerly by [`crate::parse`], but the full raw
    /// tree is kept lazy so callers that only need typed ADF fields avoid
    /// retaining two complete document representations.
    pub fn root(&self) -> &XmlElement<'a> {
        if self.raw_root.get().is_none() {
            tracing::trace!(
                input_bytes = self.original.len(),
                "parsing lazy raw XML tree"
            );
        }
        self.raw_root.get_or_init(|| {
            crate::parse::parse_tree(self.original, &self.parse_options)
                .expect("original input was already parsed successfully")
        })
    }

    /// Return the typed ADF model.
    pub fn adf(&self) -> &Adf<'a> {
        &self.adf
    }

    /// Mutably access the typed ADF model and mark the whole document dirty.
    pub fn adf_mut(&mut self) -> &mut Adf<'a> {
        self.dirty_all = true;
        tracing::trace!("marked full ADF document dirty");
        &mut self.adf
    }

    /// Mutably access one prospect and mark only that prospect dirty.
    pub fn prospect_mut(&mut self, index: usize) -> Option<&mut Prospect<'a>> {
        let found = index < self.adf.prospects.len();
        if let Some(dirty) = self.dirty_prospects.get_mut(index) {
            *dirty = true;
        }
        tracing::trace!(prospect_index = index, found, "requested mutable prospect");
        self.adf.prospects.get_mut(index)
    }

    /// Return whether any mutation path has marked the document dirty.
    pub fn is_dirty(&self) -> bool {
        self.dirty_all || self.dirty_prospects.iter().any(|dirty| *dirty)
    }

    /// Validate the typed ADF model using lenient default options.
    pub fn validate(&self) -> validate::ValidationReport<'a> {
        validate::validate(&self.adf)
    }

    /// Validate the typed ADF model with strict structural requirements.
    ///
    /// Strict mode promotes missing required structure to errors. Enum and
    /// lightweight format issues remain warnings.
    pub fn validate_strict(&self) -> validate::ValidationReport<'a> {
        validate::validate_with(&self.adf, validate::ValidationOptions { strict: true })
    }

    /// Write XML while preserving unchanged original input where possible.
    pub fn write_original_preserving<W: Write>(&self, writer: W) -> Result<()> {
        let span = tracing::debug_span!(
            "adf.write.original_preserving",
            input_bytes = self.original.len(),
            dirty_all = self.dirty_all
        );
        let _span_guard = span.enter();

        match crate::write::write_original_preserving(writer, self) {
            Ok(()) => {
                if tracing::enabled!(tracing::Level::DEBUG) {
                    let stats = crate::trace::DocumentStats::from_adf(&self.adf);
                    let dirty_prospects = crate::trace::dirty_prospect_count(&self.dirty_prospects);
                    tracing::debug!(
                        prospects = stats.prospects,
                        vehicles = stats.vehicles,
                        contacts = stats.contacts,
                        addresses = stats.addresses,
                        extensions = stats.extensions,
                        dirty_prospects,
                        mode = self.write_preservation_mode(),
                        "ADF write complete"
                    );
                }
                Ok(())
            }
            Err(error) => {
                crate::trace::record_error("write_original_preserving", &error);
                Err(error)
            }
        }
    }

    /// Write normalized XML from the typed ADF model.
    ///
    /// Non-element extension nodes are preserved, but only element extensions
    /// carry source spans for relative ordering around typed children.
    pub fn write_typed<W: Write>(&self, writer: W) -> Result<()> {
        let span = tracing::debug_span!("adf.write.typed");
        let _span_guard = span.enter();

        match crate::write::write_adf(writer, &self.adf) {
            Ok(()) => {
                if tracing::enabled!(tracing::Level::DEBUG) {
                    let stats = crate::trace::DocumentStats::from_adf(&self.adf);
                    tracing::debug!(
                        prospects = stats.prospects,
                        vehicles = stats.vehicles,
                        contacts = stats.contacts,
                        addresses = stats.addresses,
                        extensions = stats.extensions,
                        "ADF write complete"
                    );
                }
                Ok(())
            }
            Err(error) => {
                crate::trace::record_error("write_typed", &error);
                Err(error)
            }
        }
    }

    /// Return [`AdfDocument::write_original_preserving`] output as a string.
    pub fn to_original_preserving_string(&self) -> Result<String> {
        let mut bytes = Vec::new();
        self.write_original_preserving(&mut bytes)?;
        tracing::trace!(
            output_bytes = bytes.len(),
            "original-preserving string created"
        );
        Ok(String::from_utf8(bytes).expect("ADF writer only emits UTF-8"))
    }

    /// Return [`AdfDocument::write_typed`] output as a string.
    pub fn to_typed_string(&self) -> Result<String> {
        let mut bytes = Vec::new();
        self.write_typed(&mut bytes)?;
        tracing::trace!(output_bytes = bytes.len(), "typed string created");
        Ok(String::from_utf8(bytes).expect("ADF writer only emits UTF-8"))
    }

    fn write_preservation_mode(&self) -> &'static str {
        if self.dirty_all {
            "typed"
        } else if self.dirty_prospects.iter().any(|dirty| *dirty) {
            "localized"
        } else {
            "copy"
        }
    }
}
