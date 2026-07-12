use crate::document::{AdfDocument, Attribute, Span, XmlElement, XmlNode};
use crate::error::{Error, Result};
use crate::model::{
    Address, Adf, ColorCombination, Contact, Customer, Finance, Id, Name, Price, Prospect,
    Provider, TextElement, TextPart, Timeframe, Vehicle, VehicleOption, Vendor,
    resolve_standard_entity,
};
use quick_xml::Reader;
use quick_xml::encoding::Decoder;
use quick_xml::events::{BytesStart, Event};
use std::borrow::Cow;
use std::ops::Range;
use std::str;

/// Default ceiling, in bytes, on the length of a `<!DOCTYPE …>` declaration's
/// payload. Legitimate ADF documents rarely carry a DTD at all; the cap keeps
/// entity-definition payloads bounded while leaving room for a small
/// declaration.
pub const DEFAULT_MAX_DOCTYPE_LEN: usize = 4096;

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
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            reject_doctype: false,
            max_doctype_len: Some(DEFAULT_MAX_DOCTYPE_LEN),
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
}

pub(crate) fn parse(input: &str) -> Result<AdfDocument<'_>> {
    parse_with(input, &ParseOptions::default())
}

pub(crate) fn parse_with<'a>(input: &'a str, options: &ParseOptions) -> Result<AdfDocument<'a>> {
    let span = tracing::debug_span!(
        "adf.parse",
        input_bytes = input.len(),
        reject_doctype = options.reject_doctype,
        max_doctype_len = ?options.max_doctype_len
    );
    let _span_guard = span.enter();

    let document_tree = match parse_document_tree(input, options) {
        Ok(document_tree) => document_tree,
        Err(error) => {
            crate::trace::record_error("parse", &error);
            return Err(error);
        }
    };
    let (adf, prospect_spans) = adf_from_root(document_tree.root)?;
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
        document_tree.prolog,
        document_tree.epilog,
    ))
}

pub(crate) fn parse_tree<'a>(input: &'a str, options: &ParseOptions) -> Result<XmlElement<'a>> {
    Ok(parse_document_tree(input, options)?.root)
}

struct DocumentTree<'a> {
    root: XmlElement<'a>,
    prolog: Vec<XmlNode<'a>>,
    epilog: Vec<XmlNode<'a>>,
}

fn parse_document_tree<'a>(input: &'a str, options: &ParseOptions) -> Result<DocumentTree<'a>> {
    let mut reader = Reader::from_str(input);
    {
        let config = reader.config_mut();
        config.trim_text(false);
        config.check_comments = true;
    }
    let mut stack: Vec<XmlElement<'_>> = Vec::new();
    let mut root: Option<XmlElement<'_>> = None;
    let mut prolog = Vec::new();
    let mut epilog = Vec::new();

    loop {
        let event_start = reader.buffer_position() as usize;
        let position = reader.error_position();
        match reader
            .read_event()
            .map_err(|source| Error::xml(position, source))?
        {
            Event::Start(start) => stack.push(element_from_start(
                input,
                &reader,
                start,
                position,
                event_start,
                reader.buffer_position() as usize,
            )?),
            Event::Empty(start) => {
                append_element(
                    &mut stack,
                    &mut root,
                    element_from_start(
                        input,
                        &reader,
                        start,
                        position,
                        event_start,
                        reader.buffer_position() as usize,
                    )?,
                )?;
            }
            Event::End(end) => {
                let found = name_from_bytes(end.name().as_ref(), position)?.to_owned();
                let mut element = stack.pop().ok_or_else(|| Error::UnexpectedEnd {
                    name: found.clone(),
                    position,
                })?;
                if element.name.as_ref() != found {
                    return Err(Error::MismatchedEnd {
                        expected: element.name.into_owned(),
                        found,
                        position,
                    });
                }
                element.span.end = reader.buffer_position() as usize;
                append_element(&mut stack, &mut root, element)?;
            }
            Event::Text(text) => {
                let text = text
                    .xml_content()
                    .map_err(|source| Error::encoding(position, source))?;
                ensure_xml_chars(&text, position)?;
                append_node(
                    &mut stack,
                    root.is_some(),
                    XmlNode::Text(text),
                    position,
                    &mut prolog,
                    &mut epilog,
                )?;
            }
            Event::CData(cdata) => {
                let cdata = cdata
                    .decode()
                    .map_err(|source| Error::encoding(position, source))?;
                ensure_xml_chars(&cdata, position)?;
                append_node(
                    &mut stack,
                    root.is_some(),
                    XmlNode::CData(cdata),
                    position,
                    &mut prolog,
                    &mut epilog,
                )?;
            }
            Event::Comment(comment) => {
                let comment = comment
                    .decode()
                    .map_err(|source| Error::encoding(position, source))?;
                ensure_xml_chars(&comment, position)?;
                append_node(
                    &mut stack,
                    root.is_some(),
                    XmlNode::Comment(comment),
                    position,
                    &mut prolog,
                    &mut epilog,
                )?;
            }
            Event::PI(pi) => append_node(
                &mut stack,
                root.is_some(),
                XmlNode::ProcessingInstruction(Cow::Owned(validated_name_payload(
                    pi.as_ref(),
                    position,
                )?)),
                position,
                &mut prolog,
                &mut epilog,
            )?,
            Event::Decl(decl) => append_node(
                &mut stack,
                root.is_some(),
                XmlNode::Declaration(Cow::Owned(validated_name_payload(decl.as_ref(), position)?)),
                position,
                &mut prolog,
                &mut epilog,
            )?,
            Event::DocType(doc_type) => {
                if options.reject_doctype {
                    return Err(Error::DocTypeForbidden { position });
                }
                // Bound the raw byte length before decoding so an oversized
                // declaration is rejected without paying for a full UTF-8 scan
                // (or transcode) of the payload.
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
                append_node(
                    &mut stack,
                    root.is_some(),
                    XmlNode::DocType(decoded),
                    position,
                    &mut prolog,
                    &mut epilog,
                )?;
            }
            Event::GeneralRef(general_ref) => {
                if stack.is_empty() {
                    return Err(Error::ContentOutsideRoot { position });
                }
                let entity = general_ref
                    .decode()
                    .map_err(|source| Error::encoding(position, source))?;
                append_node(
                    &mut stack,
                    root.is_some(),
                    general_ref_node(entity, position)?,
                    position,
                    &mut prolog,
                    &mut epilog,
                )?;
            }
            Event::Eof => break,
        }
    }

    if let Some(unclosed) = stack.pop() {
        return Err(Error::UnexpectedEnd {
            name: unclosed.name.into_owned(),
            position: reader.error_position(),
        });
    }

    Ok(DocumentTree {
        root: root.ok_or(Error::MissingRoot)?,
        prolog,
        epilog,
    })
}

fn element_from_start<'a>(
    input: &'a str,
    reader: &Reader<&'a [u8]>,
    start: BytesStart<'a>,
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
    node: XmlNode<'a>,
    position: u64,
    prolog: &mut Vec<XmlNode<'a>>,
    epilog: &mut Vec<XmlNode<'a>>,
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

fn adf_from_root<'a>(root: XmlElement<'a>) -> Result<(Adf<'a>, Vec<Range<usize>>)> {
    let mut prospect_spans = Vec::new();
    if root.name.as_ref() != "adf" {
        return Err(Error::UnexpectedRoot {
            found: root.name.into_owned(),
            position: root.span.start as u64,
        });
    }

    let XmlElement {
        attributes,
        children,
        span,
        ..
    } = root;
    let mut adf = Adf {
        attributes,
        span,
        ..Default::default()
    };

    for node in children {
        let Some(child) = element_child_or_extension(node, &mut adf.extensions) else {
            continue;
        };
        match child.name.as_ref() {
            "prospect" => {
                prospect_spans.push(child.span.start..child.span.end);
                adf.prospects.push(prospect_from_element(child));
            }
            _ => adf.extensions.push(XmlNode::Element(child)),
        }
    }
    Ok((adf, prospect_spans))
}

fn prospect_from_element<'a>(element: XmlElement<'a>) -> Prospect<'a> {
    let XmlElement {
        attributes,
        children,
        span,
        ..
    } = element;
    let mut prospect = Prospect {
        status: attr(&attributes, "status"),
        attributes,
        span,
        ..Default::default()
    };

    for node in children {
        let Some(child) = element_child_or_extension(node, &mut prospect.extensions) else {
            continue;
        };
        match child.name.as_ref() {
            "id" => prospect.ids.push(id_from_element(child)),
            "requestdate" => set_singleton(
                &mut prospect.request_date,
                &mut prospect.extensions,
                child,
                text_from_element,
            ),
            "vehicle" => prospect.vehicles.push(vehicle_from_element(child)),
            "customer" => set_singleton(
                &mut prospect.customer,
                &mut prospect.extensions,
                child,
                customer_from_element,
            ),
            "vendor" => set_singleton(
                &mut prospect.vendor,
                &mut prospect.extensions,
                child,
                vendor_from_element,
            ),
            "provider" => set_singleton(
                &mut prospect.provider,
                &mut prospect.extensions,
                child,
                provider_from_element,
            ),
            _ => prospect.extensions.push(XmlNode::Element(child)),
        }
    }

    prospect
}

fn vehicle_from_element<'a>(element: XmlElement<'a>) -> Vehicle<'a> {
    let XmlElement {
        attributes,
        children,
        span,
        ..
    } = element;
    let mut vehicle = Vehicle {
        interest: attr(&attributes, "interest"),
        status: attr(&attributes, "status"),
        attributes,
        span,
        ..Default::default()
    };

    for node in children {
        let Some(child) = element_child_or_extension(node, &mut vehicle.extensions) else {
            continue;
        };
        match child.name.as_ref() {
            "id" => vehicle.ids.push(id_from_element(child)),
            "year" => set_singleton(
                &mut vehicle.year,
                &mut vehicle.extensions,
                child,
                text_from_element,
            ),
            "make" => set_singleton(
                &mut vehicle.make,
                &mut vehicle.extensions,
                child,
                text_from_element,
            ),
            "model" => set_singleton(
                &mut vehicle.model,
                &mut vehicle.extensions,
                child,
                text_from_element,
            ),
            "vin" => set_singleton(
                &mut vehicle.vin,
                &mut vehicle.extensions,
                child,
                text_from_element,
            ),
            "stock" => set_singleton(
                &mut vehicle.stock,
                &mut vehicle.extensions,
                child,
                text_from_element,
            ),
            "trim" => set_singleton(
                &mut vehicle.trim,
                &mut vehicle.extensions,
                child,
                text_from_element,
            ),
            "doors" => set_singleton(
                &mut vehicle.doors,
                &mut vehicle.extensions,
                child,
                text_from_element,
            ),
            "bodystyle" => set_singleton(
                &mut vehicle.body_style,
                &mut vehicle.extensions,
                child,
                text_from_element,
            ),
            "transmission" => set_singleton(
                &mut vehicle.transmission,
                &mut vehicle.extensions,
                child,
                text_from_element,
            ),
            "odometer" => set_singleton(
                &mut vehicle.odometer,
                &mut vehicle.extensions,
                child,
                text_from_element,
            ),
            "condition" => set_singleton(
                &mut vehicle.condition,
                &mut vehicle.extensions,
                child,
                text_from_element,
            ),
            "colorcombination" => vehicle
                .color_combinations
                .push(color_combination_from_element(child)),
            "imagetag" => vehicle.image_tags.push(text_from_element(child)),
            "price" => vehicle.prices.push(price_from_element(child)),
            "pricecomments" => set_singleton(
                &mut vehicle.price_comments,
                &mut vehicle.extensions,
                child,
                text_from_element,
            ),
            "option" => vehicle.options.push(option_from_element(child)),
            "finance" => set_singleton(
                &mut vehicle.finance,
                &mut vehicle.extensions,
                child,
                finance_from_element,
            ),
            "comments" => set_singleton(
                &mut vehicle.comments,
                &mut vehicle.extensions,
                child,
                text_from_element,
            ),
            _ => vehicle.extensions.push(XmlNode::Element(child)),
        }
    }

    vehicle
}

fn color_combination_from_element<'a>(element: XmlElement<'a>) -> ColorCombination<'a> {
    let XmlElement {
        attributes,
        children,
        span,
        ..
    } = element;
    let mut colors = ColorCombination {
        attributes,
        span,
        ..Default::default()
    };
    for node in children {
        let Some(child) = element_child_or_extension(node, &mut colors.extensions) else {
            continue;
        };
        match child.name.as_ref() {
            "interiorcolor" => set_singleton(
                &mut colors.interior_color,
                &mut colors.extensions,
                child,
                text_from_element,
            ),
            "exteriorcolor" => set_singleton(
                &mut colors.exterior_color,
                &mut colors.extensions,
                child,
                text_from_element,
            ),
            "preference" => set_singleton(
                &mut colors.preference,
                &mut colors.extensions,
                child,
                text_from_element,
            ),
            _ => colors.extensions.push(XmlNode::Element(child)),
        }
    }
    colors
}

fn option_from_element<'a>(element: XmlElement<'a>) -> VehicleOption<'a> {
    let XmlElement {
        attributes,
        children,
        span,
        ..
    } = element;
    let mut option = VehicleOption {
        attributes,
        span,
        ..Default::default()
    };
    for node in children {
        let Some(child) = element_child_or_extension(node, &mut option.extensions) else {
            continue;
        };
        match child.name.as_ref() {
            "optionname" => set_singleton(
                &mut option.option_name,
                &mut option.extensions,
                child,
                text_from_element,
            ),
            "manufacturercode" => set_singleton(
                &mut option.manufacturer_code,
                &mut option.extensions,
                child,
                text_from_element,
            ),
            "stock" => set_singleton(
                &mut option.stock,
                &mut option.extensions,
                child,
                text_from_element,
            ),
            "weighting" => set_singleton(
                &mut option.weighting,
                &mut option.extensions,
                child,
                text_from_element,
            ),
            "price" => option.prices.push(price_from_element(child)),
            _ => option.extensions.push(XmlNode::Element(child)),
        }
    }
    option
}

fn finance_from_element<'a>(element: XmlElement<'a>) -> Finance<'a> {
    let XmlElement {
        attributes,
        children,
        span,
        ..
    } = element;
    let mut finance = Finance {
        attributes,
        span,
        ..Default::default()
    };
    for node in children {
        let Some(child) = element_child_or_extension(node, &mut finance.extensions) else {
            continue;
        };
        match child.name.as_ref() {
            "method" => set_singleton(
                &mut finance.method,
                &mut finance.extensions,
                child,
                text_from_element,
            ),
            "amount" => finance.amounts.push(text_from_element(child)),
            "balance" => finance.balances.push(text_from_element(child)),
            _ => finance.extensions.push(XmlNode::Element(child)),
        }
    }
    finance
}

fn customer_from_element<'a>(element: XmlElement<'a>) -> Customer<'a> {
    let XmlElement {
        attributes,
        children,
        span,
        ..
    } = element;
    let mut customer = Customer {
        attributes,
        span,
        ..Default::default()
    };
    for node in children {
        let Some(child) = element_child_or_extension(node, &mut customer.extensions) else {
            continue;
        };
        match child.name.as_ref() {
            "id" => customer.ids.push(id_from_element(child)),
            "contact" => customer.contacts.push(contact_from_element(child)),
            "timeframe" => set_singleton(
                &mut customer.timeframe,
                &mut customer.extensions,
                child,
                timeframe_from_element,
            ),
            "comments" => set_singleton(
                &mut customer.comments,
                &mut customer.extensions,
                child,
                text_from_element,
            ),
            _ => customer.extensions.push(XmlNode::Element(child)),
        }
    }
    customer
}

fn timeframe_from_element<'a>(element: XmlElement<'a>) -> Timeframe<'a> {
    let XmlElement {
        attributes,
        children,
        span,
        ..
    } = element;
    let mut timeframe = Timeframe {
        attributes,
        span,
        ..Default::default()
    };
    for node in children {
        let Some(child) = element_child_or_extension(node, &mut timeframe.extensions) else {
            continue;
        };
        match child.name.as_ref() {
            "description" => set_singleton(
                &mut timeframe.description,
                &mut timeframe.extensions,
                child,
                text_from_element,
            ),
            "earliestdate" => set_singleton(
                &mut timeframe.earliest_date,
                &mut timeframe.extensions,
                child,
                text_from_element,
            ),
            "latestdate" => set_singleton(
                &mut timeframe.latest_date,
                &mut timeframe.extensions,
                child,
                text_from_element,
            ),
            _ => timeframe.extensions.push(XmlNode::Element(child)),
        }
    }
    timeframe
}

fn vendor_from_element<'a>(element: XmlElement<'a>) -> Vendor<'a> {
    let XmlElement {
        attributes,
        children,
        span,
        ..
    } = element;
    let mut vendor = Vendor {
        attributes,
        span,
        ..Default::default()
    };
    for node in children {
        let Some(child) = element_child_or_extension(node, &mut vendor.extensions) else {
            continue;
        };
        match child.name.as_ref() {
            "id" => vendor.ids.push(id_from_element(child)),
            "vendorname" => set_singleton(
                &mut vendor.vendor_name,
                &mut vendor.extensions,
                child,
                text_from_element,
            ),
            "url" => set_singleton(
                &mut vendor.url,
                &mut vendor.extensions,
                child,
                text_from_element,
            ),
            "contact" => vendor.contacts.push(contact_from_element(child)),
            _ => vendor.extensions.push(XmlNode::Element(child)),
        }
    }
    vendor
}

fn provider_from_element<'a>(element: XmlElement<'a>) -> Provider<'a> {
    let XmlElement {
        attributes,
        children,
        span,
        ..
    } = element;
    let mut provider = Provider {
        attributes,
        span,
        ..Default::default()
    };
    for node in children {
        let Some(child) = element_child_or_extension(node, &mut provider.extensions) else {
            continue;
        };
        match child.name.as_ref() {
            "id" => provider.ids.push(id_from_element(child)),
            "name" => set_singleton(
                &mut provider.name,
                &mut provider.extensions,
                child,
                name_from_element,
            ),
            "service" => set_singleton(
                &mut provider.service,
                &mut provider.extensions,
                child,
                text_from_element,
            ),
            "url" => set_singleton(
                &mut provider.url,
                &mut provider.extensions,
                child,
                text_from_element,
            ),
            "email" => set_singleton(
                &mut provider.email,
                &mut provider.extensions,
                child,
                text_from_element,
            ),
            "phone" => set_singleton(
                &mut provider.phone,
                &mut provider.extensions,
                child,
                text_from_element,
            ),
            "contact" => provider.contacts.push(contact_from_element(child)),
            _ => provider.extensions.push(XmlNode::Element(child)),
        }
    }
    provider
}

fn contact_from_element<'a>(element: XmlElement<'a>) -> Contact<'a> {
    let XmlElement {
        attributes,
        children,
        span,
        ..
    } = element;
    let mut contact = Contact {
        primary_contact: attr(&attributes, "primarycontact"),
        attributes,
        span,
        ..Default::default()
    };
    for node in children {
        let Some(child) = element_child_or_extension(node, &mut contact.extensions) else {
            continue;
        };
        match child.name.as_ref() {
            "name" => contact.names.push(name_from_element(child)),
            "email" => contact.emails.push(text_from_element(child)),
            "phone" => contact.phones.push(text_from_element(child)),
            "address" => contact.addresses.push(address_from_element(child)),
            _ => contact.extensions.push(XmlNode::Element(child)),
        }
    }
    contact
}

fn address_from_element<'a>(element: XmlElement<'a>) -> Address<'a> {
    let XmlElement {
        attributes,
        children,
        span,
        ..
    } = element;
    let mut address = Address {
        address_type: attr(&attributes, "type"),
        attributes,
        span,
        ..Default::default()
    };
    for node in children {
        let Some(child) = element_child_or_extension(node, &mut address.extensions) else {
            continue;
        };
        match child.name.as_ref() {
            "street" => address.streets.push(text_from_element(child)),
            "apartment" => set_singleton(
                &mut address.apartment,
                &mut address.extensions,
                child,
                text_from_element,
            ),
            "city" => set_singleton(
                &mut address.city,
                &mut address.extensions,
                child,
                text_from_element,
            ),
            "regioncode" => set_singleton(
                &mut address.region_code,
                &mut address.extensions,
                child,
                text_from_element,
            ),
            "postalcode" => set_singleton(
                &mut address.postal_code,
                &mut address.extensions,
                child,
                text_from_element,
            ),
            "country" => set_singleton(
                &mut address.country,
                &mut address.extensions,
                child,
                text_from_element,
            ),
            _ => address.extensions.push(XmlNode::Element(child)),
        }
    }
    address
}

fn id_from_element<'a>(element: XmlElement<'a>) -> Id<'a> {
    let XmlElement {
        attributes,
        children,
        span,
        ..
    } = element;
    Id {
        sequence: attr(&attributes, "sequence"),
        source: attr(&attributes, "source"),
        parts: text_parts(children),
        attributes,
        span,
    }
}

fn price_from_element<'a>(element: XmlElement<'a>) -> Price<'a> {
    let XmlElement {
        attributes,
        children,
        span,
        ..
    } = element;
    Price {
        price_type: attr(&attributes, "type"),
        currency: attr(&attributes, "currency"),
        delta: attr(&attributes, "delta"),
        relative_to: attr(&attributes, "relativeto"),
        source: attr(&attributes, "source"),
        parts: text_parts(children),
        attributes,
        span,
    }
}

fn name_from_element<'a>(element: XmlElement<'a>) -> Name<'a> {
    let XmlElement {
        attributes,
        children,
        span,
        ..
    } = element;
    Name {
        part: attr(&attributes, "part"),
        name_type: attr(&attributes, "type"),
        parts: text_parts(children),
        attributes,
        span,
    }
}

fn text_from_element<'a>(element: XmlElement<'a>) -> TextElement<'a> {
    let XmlElement {
        attributes,
        children,
        span,
        ..
    } = element;
    TextElement {
        parts: text_parts(children),
        attributes,
        span,
    }
}

fn text_parts<'a>(children: Vec<XmlNode<'a>>) -> Vec<TextPart<'a>> {
    let mut parts = Vec::new();
    for child in children {
        match child {
            XmlNode::Text(text) => parts.push(TextPart::Text(text)),
            XmlNode::CData(text) => parts.push(TextPart::CData(text)),
            XmlNode::EntityRef(name) => parts.push(TextPart::EntityRef(name)),
            node => parts.push(TextPart::Node(node)),
        }
    }
    parts
}

fn set_singleton<'a, T>(
    slot: &mut Option<T>,
    extensions: &mut Vec<XmlNode<'a>>,
    element: XmlElement<'a>,
    convert: fn(XmlElement<'a>) -> T,
) {
    if slot.is_none() {
        *slot = Some(convert(element));
    } else {
        extensions.push(XmlNode::Element(element));
    }
}

fn attr<'a>(attributes: &[Attribute<'a>], name: &str) -> Option<Cow<'a, str>> {
    attributes
        .iter()
        .find(|attr| attr.name.as_ref() == name)
        .map(|attr| attr.value.clone())
}

fn element_child_or_extension<'a>(
    node: XmlNode<'a>,
    extensions: &mut Vec<XmlNode<'a>>,
) -> Option<XmlElement<'a>> {
    match node {
        XmlNode::Element(element) => Some(element),
        XmlNode::Text(text) if text.as_ref().bytes().all(is_xml_whitespace) => None,
        extension => {
            extensions.push(extension);
            None
        }
    }
}
