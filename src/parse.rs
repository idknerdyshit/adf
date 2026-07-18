use crate::document::{AdfDocument, Attribute, Span, XmlElement, XmlNode};
use crate::error::{Error, Result};
use crate::model::resolve_standard_entity;
use quick_xml::Reader;
use quick_xml::encoding::Decoder;
use quick_xml::events::{BytesStart, Event};
use std::borrow::Cow;
use std::cell::OnceCell;
use std::io::Read;
use std::str;

mod stream;

/// Default ceiling, in bytes, on the length of a `<!DOCTYPE …>` declaration's
/// payload. Legitimate ADF documents rarely carry a DTD at all; the cap keeps
/// entity-definition payloads bounded while leaving room for a small
/// declaration.
pub const DEFAULT_MAX_DOCTYPE_LEN: usize = 4096;
/// Default maximum input size accepted by the parser.
pub const DEFAULT_MAX_INPUT_LEN: usize = 16 * 1024 * 1024;
/// Default maximum XML element nesting depth.
pub const DEFAULT_MAX_DEPTH: usize = 128;
/// Default maximum number of XML nodes.
pub const DEFAULT_MAX_NODES: usize = 100_000;
/// Default maximum number of attributes on one element.
pub const DEFAULT_MAX_ATTRIBUTES_PER_ELEMENT: usize = 256;

/// Parser resource whose configured limit was exceeded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseLimit {
    InputLength,
    Depth,
    Nodes,
    AttributesPerElement,
}

/// Options controlling how strictly [`crate::parse_with`] treats the input.
///
/// The defaults preserve partner data (DOCTYPE declarations are kept, not
/// rejected) while still bounding the size of any DTD declaration payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParseOptions {
    /// Reject any document that contains a `<!DOCTYPE …>` declaration.
    pub reject_doctype: bool,
    /// Maximum allowed length, in bytes, of a `<!DOCTYPE …>` declaration's
    /// payload. `None` disables the limit. Ignored when
    /// `reject_doctype` is set, since the declaration is rejected outright.
    pub max_doctype_len: Option<usize>,
    /// Maximum input length in bytes.
    pub max_input_len: Option<usize>,
    /// Maximum element nesting depth.
    pub max_depth: Option<usize>,
    /// Maximum total XML node count.
    pub max_nodes: Option<usize>,
    /// Maximum attributes allowed on one element.
    pub max_attributes_per_element: Option<usize>,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            reject_doctype: false,
            max_doctype_len: Some(DEFAULT_MAX_DOCTYPE_LEN),
            max_input_len: Some(DEFAULT_MAX_INPUT_LEN),
            max_depth: Some(DEFAULT_MAX_DEPTH),
            max_nodes: Some(DEFAULT_MAX_NODES),
            max_attributes_per_element: Some(DEFAULT_MAX_ATTRIBUTES_PER_ELEMENT),
        }
    }
}

impl ParseOptions {
    /// Reject any document that contains a `<!DOCTYPE …>` declaration.
    #[must_use]
    pub fn reject_doctype(mut self, reject: bool) -> Self {
        self.reject_doctype = reject;
        self
    }

    /// Cap the byte length of a `<!DOCTYPE …>` declaration's payload.
    #[must_use]
    pub fn max_doctype_len(mut self, limit: usize) -> Self {
        self.max_doctype_len = Some(limit);
        self
    }

    /// Remove the limit on `<!DOCTYPE …>` declaration length.
    #[must_use]
    pub fn without_doctype_limit(mut self) -> Self {
        self.max_doctype_len = None;
        self
    }

    /// Set the maximum input length in bytes.
    #[must_use]
    pub fn max_input_len(mut self, limit: usize) -> Self {
        self.max_input_len = Some(limit);
        self
    }

    /// Disable the maximum input length.
    #[must_use]
    pub fn without_input_limit(mut self) -> Self {
        self.max_input_len = None;
        self
    }

    /// Set the maximum element nesting depth.
    #[must_use]
    pub fn max_depth(mut self, limit: usize) -> Self {
        self.max_depth = Some(limit);
        self
    }

    /// Disable the maximum element nesting depth.
    #[must_use]
    pub fn without_depth_limit(mut self) -> Self {
        self.max_depth = None;
        self
    }

    /// Set the maximum total XML node count.
    #[must_use]
    pub fn max_nodes(mut self, limit: usize) -> Self {
        self.max_nodes = Some(limit);
        self
    }

    /// Disable the maximum XML node count.
    #[must_use]
    pub fn without_node_limit(mut self) -> Self {
        self.max_nodes = None;
        self
    }

    /// Set the maximum attributes allowed on one element.
    #[must_use]
    pub fn max_attributes_per_element(mut self, limit: usize) -> Self {
        self.max_attributes_per_element = Some(limit);
        self
    }

    /// Disable the per-element attribute limit.
    #[must_use]
    pub fn without_attribute_limit(mut self) -> Self {
        self.max_attributes_per_element = None;
        self
    }
}

pub(crate) fn parse(input: &str) -> Result<AdfDocument<'_>> {
    parse_with(input, &ParseOptions::default())
}

pub(crate) fn parse_owned(input: String, options: &ParseOptions) -> Result<AdfDocument<'static>> {
    let document = parse_with(&input, options)?;
    let AdfDocument {
        parse_options,
        raw_root,
        prolog,
        epilog,
        adf,
        prospect_spans,
        dirty_prospects,
        dirty_all,
        ..
    } = document;
    let owned_root = raw_root.into_inner().map(XmlElement::into_owned);
    let owned_adf = adf.into_owned();
    let owned_prolog = prolog.into_iter().map(XmlNode::into_owned).collect();
    let owned_epilog = epilog.into_iter().map(XmlNode::into_owned).collect();
    let raw_root = OnceCell::new();
    if let Some(root) = owned_root {
        let _ = raw_root.set(root);
    }
    Ok(AdfDocument {
        original: Cow::Owned(input),
        parse_options,
        raw_root,
        prolog: owned_prolog,
        epilog: owned_epilog,
        adf: owned_adf,
        prospect_spans,
        dirty_prospects,
        dirty_all,
    })
}

pub(crate) fn parse_bytes(input: &[u8], options: &ParseOptions) -> Result<AdfDocument<'static>> {
    parse_byte_vec(input.to_vec(), options)
}

fn parse_byte_vec(bytes: Vec<u8>, options: &ParseOptions) -> Result<AdfDocument<'static>> {
    check_limit(
        options.max_input_len,
        bytes.len(),
        ParseLimit::InputLength,
        0,
    )?;
    let value = String::from_utf8(bytes).map_err(|error| Error::Utf8 {
        position: error.utf8_error().valid_up_to() as u64,
        source: error.utf8_error(),
    })?;
    parse_owned(value, options)
}

pub(crate) fn parse_reader<R: Read>(
    reader: R,
    options: &ParseOptions,
) -> Result<AdfDocument<'static>> {
    let mut bytes = Vec::new();
    match options.max_input_len {
        Some(maximum) => reader
            .take(maximum.saturating_add(1) as u64)
            .read_to_end(&mut bytes)?,
        None => reader.take(u64::MAX).read_to_end(&mut bytes)?,
    };
    parse_byte_vec(bytes, options)
}

pub(crate) fn parse_with<'a>(input: &'a str, options: &ParseOptions) -> Result<AdfDocument<'a>> {
    let span = tracing::debug_span!(
        "adf.parse",
        input_bytes = input.len(),
        reject_doctype = options.reject_doctype,
        max_doctype_len = ?options.max_doctype_len
    );
    let _span_guard = span.enter();

    let parsed = match drive_events(input, options, stream::TypedAdfBuilder::default()) {
        Ok(parsed) => parsed,
        Err(error) => {
            crate::trace::record_error("parse", &error);
            return Err(error);
        }
    };
    let stream::ParsedTypedDocument {
        adf,
        prospect_spans,
        prolog,
        epilog,
    } = parsed;
    if tracing::enabled!(tracing::Level::DEBUG) {
        let stats = crate::trace::DocumentStats::from_adf(&adf);
        tracing::debug!(
            prospects = stats.prospects,
            vehicles = stats.vehicles,
            contacts = stats.contacts,
            addresses = stats.addresses,
            extensions = stats.extensions,
            "ADF parse complete"
        );
    }
    Ok(AdfDocument::new(
        input,
        *options,
        adf,
        prospect_spans,
        prolog,
        epilog,
    ))
}

pub(crate) fn parse_tree<'a>(input: &'a str, options: &ParseOptions) -> Result<XmlElement<'a>> {
    Ok(parse_document_tree(input, options)?.root)
}

pub(crate) struct ParsedDocumentTree<'a> {
    pub root: XmlElement<'a>,
    pub prolog: Vec<XmlNode<'a>>,
    _epilog: Vec<XmlNode<'a>>,
}

pub(crate) fn parse_document_tree<'a>(
    input: &'a str,
    options: &ParseOptions,
) -> Result<ParsedDocumentTree<'a>> {
    drive_events(input, options, RawTreeBuilder::default())
}

pub(super) trait EventConsumer<'a> {
    type Output;

    fn start(&mut self, element: XmlElement<'a>, position: u64) -> Result<()>;
    fn empty(&mut self, element: XmlElement<'a>, position: u64) -> Result<()>;
    fn end(&mut self, span_end: usize, position: u64) -> Result<()>;
    fn node(&mut self, node: XmlNode<'a>, position: u64) -> Result<()>;
    fn finish(self, position: u64) -> Result<Self::Output>;
}

pub(super) fn drive_events<'a, C>(
    input: &'a str,
    options: &ParseOptions,
    mut consumer: C,
) -> Result<C::Output>
where
    C: EventConsumer<'a>,
{
    check_limit(
        options.max_input_len,
        input.len(),
        ParseLimit::InputLength,
        0,
    )?;
    let mut reader = Reader::from_str(input);
    {
        let config = reader.config_mut();
        config.trim_text(false);
        config.check_comments = true;
    }
    let mut node_count = 0_usize;
    let mut depth = 0_usize;

    loop {
        let event_start = reader.buffer_position() as usize;
        let position = reader.error_position();
        match reader
            .read_event()
            .map_err(|source| Error::xml(position, source))?
        {
            Event::Start(start) => {
                check_limit(options.max_depth, depth + 1, ParseLimit::Depth, position)?;
                record_node(&mut node_count, options, position)?;
                consumer.start(
                    element_from_start(
                        input,
                        &reader,
                        start,
                        options,
                        position,
                        event_start,
                        reader.buffer_position() as usize,
                    )?,
                    position,
                )?;
                depth += 1;
            }
            Event::Empty(start) => {
                check_limit(options.max_depth, depth + 1, ParseLimit::Depth, position)?;
                record_node(&mut node_count, options, position)?;
                consumer.empty(
                    element_from_start(
                        input,
                        &reader,
                        start,
                        options,
                        position,
                        event_start,
                        reader.buffer_position() as usize,
                    )?,
                    position,
                )?;
            }
            Event::End(_) => {
                consumer.end(reader.buffer_position() as usize, position)?;
                depth = depth.saturating_sub(1);
            }
            Event::Text(text) => {
                record_node(&mut node_count, options, position)?;
                let text = text
                    .xml_content()
                    .map_err(|source| Error::encoding(position, source))?;
                ensure_xml_chars(&text, position)?;
                consumer.node(XmlNode::Text(text), position)?;
            }
            Event::CData(cdata) => {
                record_node(&mut node_count, options, position)?;
                let cdata = cdata
                    .decode()
                    .map_err(|source| Error::encoding(position, source))?;
                ensure_xml_chars(&cdata, position)?;
                consumer.node(XmlNode::CData(cdata), position)?;
            }
            Event::Comment(comment) => {
                record_node(&mut node_count, options, position)?;
                let comment = comment
                    .decode()
                    .map_err(|source| Error::encoding(position, source))?;
                ensure_xml_chars(&comment, position)?;
                consumer.node(XmlNode::Comment(comment), position)?;
            }
            Event::PI(pi) => {
                record_node(&mut node_count, options, position)?;
                consumer.node(
                    XmlNode::ProcessingInstruction(Cow::Owned(validated_name_payload(
                        pi.as_ref(),
                        position,
                    )?)),
                    position,
                )?;
            }
            Event::Decl(decl) => {
                record_node(&mut node_count, options, position)?;
                let declaration = validated_name_payload(decl.as_ref(), position)?;
                ensure_utf8_declaration(&declaration, position)?;
                consumer.node(XmlNode::Declaration(Cow::Owned(declaration)), position)?;
            }
            Event::DocType(doc_type) => {
                record_node(&mut node_count, options, position)?;
                if options.reject_doctype {
                    return Err(Error::DocTypeForbidden { position });
                }
                if let Some(limit) = options.max_doctype_len {
                    let length = doc_type.len();
                    if length > limit {
                        return Err(Error::DocTypeTooLong {
                            length,
                            limit,
                            position,
                        });
                    }
                }
                let decoded = doc_type
                    .decode()
                    .map_err(|source| Error::encoding(position, source))?;
                ensure_xml_chars(&decoded, position)?;
                consumer.node(XmlNode::DocType(decoded), position)?;
            }
            Event::GeneralRef(general_ref) => {
                record_node(&mut node_count, options, position)?;
                if depth == 0 {
                    return Err(Error::ContentOutsideRoot { position });
                }
                let entity = general_ref
                    .decode()
                    .map_err(|source| Error::encoding(position, source))?;
                consumer.node(general_ref_node(entity, position)?, position)?;
            }
            Event::Eof => return consumer.finish(reader.error_position()),
        }
    }
}

#[derive(Default)]
struct RawTreeBuilder<'a> {
    stack: Vec<XmlElement<'a>>,
    root: Option<XmlElement<'a>>,
    prolog: Vec<XmlNode<'a>>,
    epilog: Vec<XmlNode<'a>>,
}

impl<'a> EventConsumer<'a> for RawTreeBuilder<'a> {
    type Output = ParsedDocumentTree<'a>;

    fn start(&mut self, element: XmlElement<'a>, _position: u64) -> Result<()> {
        self.stack.push(element);
        Ok(())
    }

    fn empty(&mut self, element: XmlElement<'a>, _position: u64) -> Result<()> {
        append_element(&mut self.stack, &mut self.root, element)
    }

    fn end(&mut self, span_end: usize, position: u64) -> Result<()> {
        let mut element = self.stack.pop().ok_or_else(|| Error::UnexpectedEnd {
            name: String::new(),
            position,
        })?;
        element.span.end = span_end;
        append_element(&mut self.stack, &mut self.root, element)
    }

    fn node(&mut self, node: XmlNode<'a>, position: u64) -> Result<()> {
        append_node(
            &mut self.stack,
            self.root.is_some(),
            &mut self.prolog,
            &mut self.epilog,
            node,
            position,
        )
    }

    fn finish(mut self, position: u64) -> Result<Self::Output> {
        if let Some(unclosed) = self.stack.pop() {
            return Err(Error::UnexpectedEnd {
                name: unclosed.name.into_owned(),
                position,
            });
        }
        Ok(ParsedDocumentTree {
            root: self.root.ok_or(Error::MissingRoot)?,
            prolog: self.prolog,
            _epilog: self.epilog,
        })
    }
}

fn ensure_utf8_declaration(declaration: &str, position: u64) -> Result<()> {
    let lowercase = declaration.to_ascii_lowercase();
    let Some(index) = lowercase.find("encoding") else {
        return Ok(());
    };
    let rest = declaration[index + "encoding".len()..].trim_start();
    let Some(rest) = rest.strip_prefix('=') else {
        return Ok(());
    };
    let rest = rest.trim_start();
    let Some(quote) = rest.chars().next().filter(|ch| matches!(ch, '\'' | '"')) else {
        return Ok(());
    };
    let value = rest[quote.len_utf8()..]
        .split(quote)
        .next()
        .unwrap_or_default();
    if value.eq_ignore_ascii_case("utf-8") || value.eq_ignore_ascii_case("utf8") {
        Ok(())
    } else {
        Err(Error::UnsupportedEncoding {
            encoding: value.to_owned(),
            position,
        })
    }
}

fn element_from_start<'a>(
    input: &'a str,
    reader: &Reader<&'a [u8]>,
    start: BytesStart<'a>,
    options: &ParseOptions,
    position: u64,
    span_start: usize,
    span_end: usize,
) -> Result<XmlElement<'a>> {
    let name = borrowed_name(input, start.name().as_ref(), position)?;
    let mut attributes = Vec::new();
    for attr in start.attributes() {
        let attr = attr.map_err(|source| Error::Attribute { position, source })?;
        let attr_name = borrowed_name(input, attr.key.as_ref(), position)?;
        let value = decode_attribute_value(input, attr.value.as_ref(), reader.decoder(), position)?;
        attributes.push(Attribute {
            name: attr_name,
            value,
        });
        check_limit(
            options.max_attributes_per_element,
            attributes.len(),
            ParseLimit::AttributesPerElement,
            position,
        )?;
    }

    Ok(XmlElement {
        name,
        attributes,
        children: Vec::new(),
        span: Span {
            start: span_start,
            end: span_end,
        },
    })
}

fn append_element<'a>(
    stack: &mut [XmlElement<'a>],
    root: &mut Option<XmlElement<'a>>,
    element: XmlElement<'a>,
) -> Result<()> {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(XmlNode::Element(element));
    } else if root.is_some() {
        return Err(Error::MultipleRoots);
    } else {
        *root = Some(element);
    }
    Ok(())
}

fn append_node<'a>(
    stack: &mut [XmlElement<'a>],
    has_root: bool,
    prolog: &mut Vec<XmlNode<'a>>,
    epilog: &mut Vec<XmlNode<'a>>,
    node: XmlNode<'a>,
    position: u64,
) -> Result<()> {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(node);
        return Ok(());
    }

    if is_document_misc(&node, has_root) {
        if has_root {
            epilog.push(node);
        } else {
            prolog.push(node);
        }
        return Ok(());
    }

    Err(Error::ContentOutsideRoot { position })
}

fn record_node(count: &mut usize, options: &ParseOptions, position: u64) -> Result<()> {
    *count = count.saturating_add(1);
    check_limit(options.max_nodes, *count, ParseLimit::Nodes, position)
}

fn check_limit(limit: Option<usize>, actual: usize, kind: ParseLimit, position: u64) -> Result<()> {
    if let Some(maximum) = limit {
        if actual > maximum {
            return Err(Error::LimitExceeded {
                limit: kind,
                maximum,
                actual,
                position,
            });
        }
    }
    Ok(())
}

fn name_from_bytes(bytes: &[u8], position: u64) -> Result<&str> {
    str::from_utf8(bytes).map_err(|source| Error::Utf8 { position, source })
}

fn validated_name_payload(bytes: &[u8], position: u64) -> Result<String> {
    let value = name_from_bytes(bytes, position)?;
    ensure_xml_chars(value, position)?;
    Ok(value.to_owned())
}

fn borrowed_name<'a>(input: &'a str, bytes: &[u8], position: u64) -> Result<Cow<'a, str>> {
    let name = name_from_bytes(bytes, position)?;
    Ok(match borrowed_from_input(input, bytes) {
        Some(borrowed) => Cow::Borrowed(borrowed),
        None => Cow::Owned(name.to_owned()),
    })
}

fn borrowed_from_input<'a>(input: &'a str, bytes: &[u8]) -> Option<&'a str> {
    let input_bytes = input.as_bytes();
    let input_start = input_bytes.as_ptr() as usize;
    let input_end = input_start + input_bytes.len();
    let bytes_start = bytes.as_ptr() as usize;
    let bytes_end = bytes_start + bytes.len();

    if bytes_start < input_start || bytes_end > input_end {
        return None;
    }

    let offset = bytes_start - input_start;
    let end = offset + bytes.len();
    input.get(offset..end)
}

fn is_document_misc(node: &XmlNode<'_>, has_root: bool) -> bool {
    match node {
        XmlNode::Text(text) => text.as_ref().bytes().all(is_xml_whitespace),
        XmlNode::Comment(_) | XmlNode::ProcessingInstruction(_) => true,
        XmlNode::Declaration(_) | XmlNode::DocType(_) => !has_root,
        XmlNode::CData(_) | XmlNode::EntityRef(_) | XmlNode::Element(_) => false,
    }
}

fn is_xml_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}

fn general_ref_node<'a>(entity: Cow<'a, str>, position: u64) -> Result<XmlNode<'a>> {
    if let Some(resolved) = resolve_standard_entity(&entity) {
        return Ok(XmlNode::Text(Cow::Borrowed(resolved)));
    }
    if entity.starts_with('#') {
        Ok(XmlNode::Text(decode_character_reference(entity, position)?))
    } else {
        ensure_entity_name(&entity, position)?;
        Ok(XmlNode::EntityRef(entity))
    }
}

fn decode_character_reference(entity: Cow<'_, str>, position: u64) -> Result<Cow<'_, str>> {
    let Some(value) = entity.strip_prefix('#') else {
        return Ok(Cow::Owned(format!("&{entity};")));
    };

    let codepoint =
        if let Some(hex) = value.strip_prefix('x').or_else(|| value.strip_prefix('X')) {
            u32::from_str_radix(hex, 16)
        } else {
            value.parse()
        }
        .map_err(|_| Error::InvalidCharacterReference {
            reference: entity.to_string(),
            position,
        })?;

    let Some(ch) = char::from_u32(codepoint) else {
        return Err(Error::InvalidCharacterReference {
            reference: entity.to_string(),
            position,
        });
    };
    if !is_xml_char(ch) {
        return Err(Error::InvalidCharacterReference {
            reference: entity.to_string(),
            position,
        });
    }

    Ok(Cow::Owned(ch.to_string()))
}

fn decode_attribute_value<'a>(
    input: &'a str,
    raw: &[u8],
    decoder: Decoder,
    position: u64,
) -> Result<Cow<'a, str>> {
    let decoded = decoder
        .decode(raw)
        .map_err(|source| Error::encoding(position, source))?;
    ensure_xml_chars(&decoded, position)?;

    let decoded = match decode_entities_preserving_unknown(&decoded, position)? {
        Cow::Borrowed(_) => decoded,
        Cow::Owned(value) => Cow::Owned(value),
    };

    Ok(match decoded {
        Cow::Borrowed(slice) => match borrowed_from_input(input, slice.as_bytes()) {
            Some(borrowed) => Cow::Borrowed(borrowed),
            None => Cow::Owned(slice.to_owned()),
        },
        Cow::Owned(owned) => Cow::Owned(owned),
    })
}

fn decode_entities_preserving_unknown(value: &str, position: u64) -> Result<Cow<'_, str>> {
    if !value.as_bytes().contains(&b'&') {
        return Ok(Cow::Borrowed(value));
    }

    let mut decoded = String::with_capacity(value.len());
    let mut remaining = value;
    loop {
        let Some(start) = remaining.find('&') else {
            decoded.push_str(remaining);
            return Ok(Cow::Owned(decoded));
        };
        decoded.push_str(&remaining[..start]);
        let entity_start = start + 1;
        let after_amp = &remaining[entity_start..];
        let Some(end) = after_amp.find(';') else {
            return Err(Error::InvalidEntityReference {
                reference: after_amp.to_owned(),
                position,
            });
        };
        let entity = &after_amp[..end];
        if entity.is_empty() {
            return Err(Error::InvalidEntityReference {
                reference: String::new(),
                position,
            });
        }
        if let Some(resolved) = resolve_standard_entity(entity) {
            decoded.push_str(resolved);
        } else if entity.starts_with('#') {
            decoded.push_str(&decode_character_reference(
                Cow::Borrowed(entity),
                position,
            )?);
        } else {
            ensure_entity_name(entity, position)?;
            decoded.push('&');
            decoded.push_str(entity);
            decoded.push(';');
        }
        remaining = &after_amp[end + 1..];
    }
}

pub(crate) fn ensure_xml_chars(value: &str, position: u64) -> Result<()> {
    if let Some(character) = value.chars().find(|ch| !is_xml_char(*ch)) {
        return Err(Error::IllegalCharacter {
            character,
            position,
        });
    }
    Ok(())
}

fn is_xml_char(ch: char) -> bool {
    matches!(
        ch as u32,
        0x09 | 0x0A | 0x0D | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF
    )
}

pub(crate) fn ensure_entity_name(name: &str, position: u64) -> Result<()> {
    if is_xml_name(name) {
        Ok(())
    } else {
        Err(Error::InvalidEntityReference {
            reference: name.to_owned(),
            position,
        })
    }
}

pub(crate) fn is_xml_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    is_xml_name_start_char(first) && chars.all(is_xml_name_char)
}

fn is_xml_name_start_char(ch: char) -> bool {
    matches!(
        ch as u32,
        0x3A
            | 0x41..=0x5A
            | 0x5F
            | 0x61..=0x7A
            | 0xC0..=0xD6
            | 0xD8..=0xF6
            | 0xF8..=0x2FF
            | 0x370..=0x37D
            | 0x37F..=0x1FFF
            | 0x200C..=0x200D
            | 0x2070..=0x218F
            | 0x2C00..=0x2FEF
            | 0x3001..=0xD7FF
            | 0xF900..=0xFDCF
            | 0xFDF0..=0xFFFD
            | 0x10000..=0xEFFFF
    )
}

fn is_xml_name_char(ch: char) -> bool {
    is_xml_name_start_char(ch)
        || matches!(
            ch as u32,
            0x2D
                | 0x2E
                | 0x30..=0x39
                | 0xB7
                | 0x0300..=0x036F
                | 0x203F..=0x2040
        )
}

fn attr<'a>(attributes: &[Attribute<'a>], name: &str) -> Option<Cow<'a, str>> {
    attributes
        .iter()
        .find(|attr| attr.name.as_ref() == name)
        .map(|attr| attr.value.clone())
}
