use crate::model::{Adf, Prospect};
use crate::{ParseOptions, Result, validate};
use std::borrow::Cow;
use std::cell::OnceCell;
use std::io::Write;
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute<'a> {
    pub name: Cow<'a, str>,
    pub value: Cow<'a, str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlElement<'a> {
    pub name: Cow<'a, str>,
    pub attributes: Vec<Attribute<'a>>,
    pub children: Vec<XmlNode<'a>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XmlNode<'a> {
    Element(XmlElement<'a>),
    Text(Cow<'a, str>),
    CData(Cow<'a, str>),
    EntityRef(Cow<'a, str>),
    Comment(Cow<'a, str>),
    ProcessingInstruction(Cow<'a, str>),
    Declaration(Cow<'a, str>),
    DocType(Cow<'a, str>),
}

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

    pub fn original(&self) -> &'a str {
        self.original
    }

    /// Return the raw XML root, reparsing the original input on first access.
    ///
    /// The typed model is built eagerly by [`crate::parse`], but the full raw
    /// tree is kept lazy so callers that only need typed ADF fields avoid
    /// retaining two complete document representations.
    pub fn root(&self) -> &XmlElement<'a> {
        self.raw_root.get_or_init(|| {
            crate::parse::parse_tree(self.original, &self.parse_options)
                .expect("original input was already parsed successfully")
        })
    }

    pub fn adf(&self) -> &Adf<'a> {
        &self.adf
    }

    pub fn adf_mut(&mut self) -> &mut Adf<'a> {
        self.dirty_all = true;
        &mut self.adf
    }

    pub fn prospect_mut(&mut self, index: usize) -> Option<&mut Prospect<'a>> {
        if let Some(dirty) = self.dirty_prospects.get_mut(index) {
            *dirty = true;
        }
        self.adf.prospects.get_mut(index)
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty_all || self.dirty_prospects.iter().any(|dirty| *dirty)
    }

    pub fn validate(&self) -> validate::ValidationReport<'a> {
        validate::validate(&self.adf)
    }

    pub fn validate_strict(&self) -> validate::ValidationReport<'a> {
        validate::validate_with(&self.adf, validate::ValidationOptions { strict: true })
    }

    pub fn write_original_preserving<W: Write>(&self, writer: W) -> Result<()> {
        crate::write::write_original_preserving(writer, self)
    }

    pub fn write_typed<W: Write>(&self, writer: W) -> Result<()> {
        crate::write::write_adf(writer, &self.adf)
    }

    pub fn to_original_preserving_string(&self) -> Result<String> {
        let mut bytes = Vec::new();
        self.write_original_preserving(&mut bytes)?;
        Ok(String::from_utf8(bytes).expect("ADF writer only emits UTF-8"))
    }

    pub fn to_typed_string(&self) -> Result<String> {
        let mut bytes = Vec::new();
        self.write_typed(&mut bytes)?;
        Ok(String::from_utf8(bytes).expect("ADF writer only emits UTF-8"))
    }
}
