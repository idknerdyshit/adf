use crate::document::{AdfDocument, Attribute, Span, XmlElement, XmlNode};
use crate::error::{Error, Result};
use crate::model::{
    Address, Adf, ColorCombination, Contact, Customer, Finance, Id, Name, Price, Prospect,
    Provider, TextElement, TextPart, Timeframe, Vehicle, VehicleOption, Vendor,
    resolve_standard_entity,
};
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use std::borrow::Cow;
use std::ops::Range;
use std::str;

/// Default ceiling, in bytes, on the length of a `<!DOCTYPE …>` declaration's
/// internal subset. Legitimate ADF documents rarely carry a DTD at all; the
/// cap keeps entity-definition payloads bounded while leaving room for a small
/// internal subset.
pub const DEFAULT_MAX_DOCTYPE_LEN: usize = 4096;

/// Options controlling how strictly [`parse_with`] treats the input.
///
/// The defaults preserve partner data (DOCTYPE declarations are kept, not
/// rejected) while still bounding the size of any DTD internal subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParseOptions {
    /// Reject any document that contains a `<!DOCTYPE …>` declaration.
    pub reject_doctype: bool,
    /// Maximum allowed length, in bytes, of a `<!DOCTYPE …>` declaration's
    /// internal subset. `None` disables the limit. Ignored when
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

    /// Cap the byte length of a `<!DOCTYPE …>` declaration's internal subset.
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
    let root = parse_tree(input, options)?;
    let (adf, prospect_spans) = adf_from_root(root);
    Ok(AdfDocument::new(input, *options, adf, prospect_spans))
}

pub(crate) fn parse_tree<'a>(input: &'a str, options: &ParseOptions) -> Result<XmlElement<'a>> {
    let mut reader = Reader::from_str(input);
    reader.config_mut().trim_text(false);
    let mut stack: Vec<XmlElement<'_>> = Vec::new();
    let mut root: Option<XmlElement<'_>> = None;

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
            Event::Text(text) => append_node(
                &mut stack,
                root.is_some(),
                XmlNode::Text(
                    text.xml_content()
                        .map_err(|source| Error::encoding(position, source))?,
                ),
                position,
            )?,
            Event::CData(cdata) => append_node(
                &mut stack,
                root.is_some(),
                XmlNode::CData(
                    cdata
                        .decode()
                        .map_err(|source| Error::encoding(position, source))?,
                ),
                position,
            )?,
            Event::Comment(comment) => append_node(
                &mut stack,
                root.is_some(),
                XmlNode::Comment(
                    comment
                        .decode()
                        .map_err(|source| Error::encoding(position, source))?,
                ),
                position,
            )?,
            Event::PI(pi) => append_node(
                &mut stack,
                root.is_some(),
                XmlNode::ProcessingInstruction(Cow::Owned(
                    name_from_bytes(pi.as_ref(), position)?.to_owned(),
                )),
                position,
            )?,
            Event::Decl(decl) => append_node(
                &mut stack,
                root.is_some(),
                XmlNode::Declaration(Cow::Owned(
                    name_from_bytes(decl.as_ref(), position)?.to_owned(),
                )),
                position,
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
                append_node(
                    &mut stack,
                    root.is_some(),
                    XmlNode::DocType(decoded),
                    position,
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

    root.ok_or(Error::MissingRoot)
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
        let value = attr
            .decode_and_unescape_value(reader.decoder())
            .map_err(|source| Error::xml(position, source))?;
        let value = match value {
            Cow::Borrowed(slice) => match borrowed_from_input(input, slice.as_bytes()) {
                Some(borrowed) => Cow::Borrowed(borrowed),
                None => Cow::Owned(slice.to_owned()),
            },
            Cow::Owned(owned) => Cow::Owned(owned),
        };
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
) -> Result<()> {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(node);
        return Ok(());
    }

    if is_document_misc(&node, has_root) {
        return Ok(());
    }

    Err(Error::ContentOutsideRoot { position })
}

fn name_from_bytes(bytes: &[u8], position: u64) -> Result<&str> {
    str::from_utf8(bytes).map_err(|source| Error::Utf8 { position, source })
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

    Ok(Cow::Owned(ch.to_string()))
}

fn adf_from_root<'a>(root: XmlElement<'a>) -> (Adf<'a>, Vec<Range<usize>>) {
    let mut prospect_spans = Vec::new();
    if root.name.as_ref() != "adf" {
        let mut adf = Adf {
            span: root.span,
            ..Default::default()
        };
        adf.extensions.push(XmlNode::Element(root));
        return (adf, prospect_spans);
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

    for child in element_children(children) {
        match child.name.as_ref() {
            "prospect" => {
                prospect_spans.push(child.span.start..child.span.end);
                adf.prospects.push(prospect_from_element(child));
            }
            _ => adf.extensions.push(XmlNode::Element(child)),
        }
    }
    (adf, prospect_spans)
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

    for child in element_children(children) {
        match child.name.as_ref() {
            "id" => prospect.ids.push(id_from_element(child)),
            "requestdate" => prospect.request_date = Some(text_from_element(child)),
            "vehicle" => prospect.vehicles.push(vehicle_from_element(child)),
            "customer" => prospect.customer = Some(customer_from_element(child)),
            "vendor" => prospect.vendor = Some(vendor_from_element(child)),
            "provider" => prospect.provider = Some(provider_from_element(child)),
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

    for child in element_children(children) {
        match child.name.as_ref() {
            "id" => vehicle.ids.push(id_from_element(child)),
            "year" => vehicle.year = Some(text_from_element(child)),
            "make" => vehicle.make = Some(text_from_element(child)),
            "model" => vehicle.model = Some(text_from_element(child)),
            "vin" => vehicle.vin = Some(text_from_element(child)),
            "stock" => vehicle.stock = Some(text_from_element(child)),
            "trim" => vehicle.trim = Some(text_from_element(child)),
            "doors" => vehicle.doors = Some(text_from_element(child)),
            "bodystyle" => vehicle.body_style = Some(text_from_element(child)),
            "transmission" => vehicle.transmission = Some(text_from_element(child)),
            "odometer" => vehicle.odometer = Some(text_from_element(child)),
            "condition" => vehicle.condition = Some(text_from_element(child)),
            "colorcombination" => vehicle
                .color_combinations
                .push(color_combination_from_element(child)),
            "imagetag" => vehicle.image_tags.push(text_from_element(child)),
            "price" => vehicle.prices.push(price_from_element(child)),
            "pricecomments" => vehicle.price_comments = Some(text_from_element(child)),
            "option" => vehicle.options.push(option_from_element(child)),
            "finance" => vehicle.finance = Some(finance_from_element(child)),
            "comments" => vehicle.comments = Some(text_from_element(child)),
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
    for child in element_children(children) {
        match child.name.as_ref() {
            "interiorcolor" => colors.interior_color = Some(text_from_element(child)),
            "exteriorcolor" => colors.exterior_color = Some(text_from_element(child)),
            "preference" => colors.preference = Some(text_from_element(child)),
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
    for child in element_children(children) {
        match child.name.as_ref() {
            "optionname" => option.option_name = Some(text_from_element(child)),
            "manufacturercode" => option.manufacturer_code = Some(text_from_element(child)),
            "stock" => option.stock = Some(text_from_element(child)),
            "weighting" => option.weighting = Some(text_from_element(child)),
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
    for child in element_children(children) {
        match child.name.as_ref() {
            "method" => finance.method = Some(text_from_element(child)),
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
    for child in element_children(children) {
        match child.name.as_ref() {
            "id" => customer.ids.push(id_from_element(child)),
            "contact" => customer.contacts.push(contact_from_element(child)),
            "timeframe" => customer.timeframe = Some(timeframe_from_element(child)),
            "comments" => customer.comments = Some(text_from_element(child)),
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
    for child in element_children(children) {
        match child.name.as_ref() {
            "description" => timeframe.description = Some(text_from_element(child)),
            "earliestdate" => timeframe.earliest_date = Some(text_from_element(child)),
            "latestdate" => timeframe.latest_date = Some(text_from_element(child)),
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
    for child in element_children(children) {
        match child.name.as_ref() {
            "id" => vendor.ids.push(id_from_element(child)),
            "vendorname" => vendor.vendor_name = Some(text_from_element(child)),
            "url" => vendor.url = Some(text_from_element(child)),
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
    for child in element_children(children) {
        match child.name.as_ref() {
            "id" => provider.ids.push(id_from_element(child)),
            "name" => provider.name = Some(name_from_element(child)),
            "service" => provider.service = Some(text_from_element(child)),
            "url" => provider.url = Some(text_from_element(child)),
            "email" => provider.email = Some(text_from_element(child)),
            "phone" => provider.phone = Some(text_from_element(child)),
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
    for child in element_children(children) {
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
    for child in element_children(children) {
        match child.name.as_ref() {
            "street" => address.streets.push(text_from_element(child)),
            "apartment" => address.apartment = Some(text_from_element(child)),
            "city" => address.city = Some(text_from_element(child)),
            "regioncode" => address.region_code = Some(text_from_element(child)),
            "postalcode" => address.postal_code = Some(text_from_element(child)),
            "country" => address.country = Some(text_from_element(child)),
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

fn attr<'a>(attributes: &[Attribute<'a>], name: &str) -> Option<Cow<'a, str>> {
    attributes
        .iter()
        .find(|attr| attr.name.as_ref() == name)
        .map(|attr| attr.value.clone())
}

fn element_children<'a>(children: Vec<XmlNode<'a>>) -> impl Iterator<Item = XmlElement<'a>> {
    children.into_iter().filter_map(|child| match child {
        XmlNode::Element(element) => Some(element),
        _ => None,
    })
}
