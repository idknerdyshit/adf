use crate::document::{AdfDocument, Attribute, Span, XmlElement, XmlNode};
use crate::model::{
    Address, Adf, ColorCombination, Contact, Customer, Finance, Id, Name, Price, Prospect,
    Provider, TextElement, TextPart, Timeframe, Vehicle, VehicleOption, Vendor,
};
use crate::{Error, Result};
use std::collections::HashSet;
use std::io::Write;

/// Handling policy for unresolved custom entity references during typed writing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum UnknownEntityPolicy {
    #[default]
    Error,
    Escape,
    Preserve,
}

/// Options for normalized typed ADF output.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct WriteOptions {
    pub xml_declaration: bool,
    pub adf_processing_instruction: bool,
    pub doctype: Option<String>,
    pub unknown_entity_policy: UnknownEntityPolicy,
}

impl Default for WriteOptions {
    fn default() -> Self {
        Self {
            xml_declaration: true,
            adf_processing_instruction: true,
            doctype: None,
            unknown_entity_policy: UnknownEntityPolicy::Error,
        }
    }
}

impl WriteOptions {
    #[must_use]
    pub fn xml_declaration(mut self, enabled: bool) -> Self {
        self.xml_declaration = enabled;
        self
    }
    #[must_use]
    pub fn adf_processing_instruction(mut self, enabled: bool) -> Self {
        self.adf_processing_instruction = enabled;
        self
    }
    #[must_use]
    pub fn doctype(mut self, value: impl Into<String>) -> Self {
        self.doctype = Some(value.into());
        self
    }
    #[must_use]
    pub fn without_doctype(mut self) -> Self {
        self.doctype = None;
        self
    }
    #[must_use]
    pub fn unknown_entity_policy(mut self, policy: UnknownEntityPolicy) -> Self {
        self.unknown_entity_policy = policy;
        self
    }
}

struct Output<'a> {
    writer: &'a mut dyn Write,
    entity_policy: UnknownEntityPolicy,
    declared_entities: Option<&'a HashSet<String>>,
}

impl Write for Output<'_> {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.writer.write(buffer)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

pub(crate) fn write_adf<W: Write>(writer: W, adf: &Adf<'_>) -> Result<()> {
    write_adf_with(writer, adf, &WriteOptions::default())
}

pub(crate) fn write_adf_with<W: Write>(
    mut writer: W,
    adf: &Adf<'_>,
    options: &WriteOptions,
) -> Result<()> {
    let declared_entities = options
        .doctype
        .as_deref()
        .map(declared_general_entities_from_doctype);
    let mut output = Output {
        writer: &mut writer,
        entity_policy: options.unknown_entity_policy,
        declared_entities: declared_entities.as_ref(),
    };
    if options.xml_declaration {
        output.write_all(br#"<?xml version="1.0"?>"#)?;
    }
    if options.adf_processing_instruction {
        output.write_all(b"\n<?adf version=\"1.0\"?>")?;
    }
    if let Some(doctype) = &options.doctype {
        validate_doctype(doctype)?;
        output.write_all(b"\n<!DOCTYPE ")?;
        output.write_all(doctype.as_bytes())?;
        output.write_all(b">")?;
    }
    write_adf_root(&mut output, adf)
}

fn write_adf_root(writer: &mut Output<'_>, adf: &Adf<'_>) -> Result<()> {
    start_with_attrs(writer, "adf", &adf.attributes)?;
    if adf.extensions.is_empty() {
        for prospect in &adf.prospects {
            write_prospect(writer, prospect)?;
        }
        writer.write_all(b"\n</adf>")?;
        return Ok(());
    }
    let mut children = Vec::new();
    for prospect in &adf.prospects {
        children.push(Child::Prospect {
            value: prospect,
            order: 0,
        });
    }
    for extension in &adf.extensions {
        children.push(Child::Extension {
            value: extension,
            order: 1,
        });
    }
    write_children(writer, &mut children)?;
    writer.write_all(b"\n</adf>")?;
    Ok(())
}

pub(crate) fn write_original_preserving<W: Write>(
    mut writer: W,
    document: &AdfDocument<'_>,
) -> Result<()> {
    if document.dirty_all {
        tracing::debug!(
            reason = "dirty_all",
            "original-preserving write using typed writer"
        );
        return write_document_typed(writer, document);
    }
    if document
        .dirty_prospects
        .iter()
        .enumerate()
        .any(|(index, dirty)| {
            *dirty
                && (document.prospect_spans.get(index).is_none()
                    || document.adf.prospects.get(index).is_none())
        })
    {
        tracing::debug!(
            reason = "prospect_index_mismatch",
            "original-preserving write using typed writer"
        );
        return write_document_typed(writer, document);
    }

    let mut output = Output {
        writer: &mut writer,
        entity_policy: UnknownEntityPolicy::Preserve,
        declared_entities: None,
    };
    let mut cursor = 0;
    for (index, dirty) in document.dirty_prospects.iter().enumerate() {
        if !dirty {
            continue;
        }
        let span = &document.prospect_spans[index];
        let prospect = &document.adf.prospects[index];

        output.write_all(&document.original.as_bytes()[cursor..span.start])?;
        write_prospect(&mut output, prospect)?;
        cursor = span.end;
    }

    output.write_all(&document.original.as_bytes()[cursor..])?;
    Ok(())
}

pub(crate) fn write_document_typed<W: Write>(
    mut writer: W,
    document: &AdfDocument<'_>,
) -> Result<()> {
    let declared_entities = declared_general_entities(&document.prolog);
    let mut output = Output {
        writer: &mut writer,
        entity_policy: UnknownEntityPolicy::Error,
        declared_entities: Some(&declared_entities),
    };
    write_document_typed_inner(&mut output, document)
}

fn declared_general_entities(prolog: &[XmlNode<'_>]) -> HashSet<String> {
    let mut entities = HashSet::new();
    for node in prolog {
        let XmlNode::DocType(value) = node else {
            continue;
        };
        entities.extend(declared_general_entities_from_doctype(value));
    }
    entities
}

fn declared_general_entities_from_doctype(value: &str) -> HashSet<String> {
    let mut entities = HashSet::new();
    let bytes = value.as_bytes();
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor..].starts_with(b"<!--") {
            cursor = bytes[cursor + 4..]
                .windows(3)
                .position(|window| window == b"-->")
                .map_or(bytes.len(), |offset| cursor + 4 + offset + 3);
            continue;
        }
        if bytes[cursor..].starts_with(b"<?") {
            cursor = bytes[cursor + 2..]
                .windows(2)
                .position(|window| window == b"?>")
                .map_or(bytes.len(), |offset| cursor + 2 + offset + 2);
            continue;
        }
        if matches!(bytes[cursor], b'\'' | b'"') {
            let quote = bytes[cursor];
            cursor += 1;
            while cursor < bytes.len() && bytes[cursor] != quote {
                cursor += 1;
            }
            cursor = cursor.saturating_add(1);
            continue;
        }
        if !bytes[cursor..].starts_with(b"<!ENTITY") {
            cursor += 1;
            continue;
        }

        cursor += b"<!ENTITY".len();
        if !bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
            continue;
        }
        while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
            cursor += 1;
        }
        if bytes.get(cursor) == Some(&b'%') {
            continue;
        }
        let name_start = cursor;
        while bytes
            .get(cursor)
            .is_some_and(|byte| !byte.is_ascii_whitespace() && *byte != b'>')
        {
            cursor += 1;
        }
        let Some(name) = value.get(name_start..cursor) else {
            continue;
        };
        if bytes.get(cursor).is_some_and(u8::is_ascii_whitespace)
            && crate::parse::ensure_entity_name(name, 0).is_ok()
        {
            entities.insert(name.to_owned());
        }
    }
    entities
}

fn write_document_typed_inner(output: &mut Output<'_>, document: &AdfDocument<'_>) -> Result<()> {
    if let Some(declaration) = document
        .prolog
        .iter()
        .find(|node| matches!(node, XmlNode::Declaration(_)))
    {
        write_xml_node(output, declaration)?;
    } else {
        output.write_all(br#"<?xml version="1.0"?>"#)?;
    }
    output.write_all(b"\n")?;
    if let Some(adf_pi) = document
        .prolog
        .iter()
        .find(|node| is_adf_processing_instruction(node))
    {
        write_xml_node(output, adf_pi)?;
    } else {
        output.write_all(b"<?adf version=\"1.0\"?>")?;
    }
    for node in &document.prolog {
        if is_document_whitespace(node)
            || matches!(node, XmlNode::Declaration(_))
            || is_adf_processing_instruction(node)
        {
            continue;
        }
        output.write_all(b"\n")?;
        write_xml_node(output, node)?;
    }
    write_adf_root(output, &document.adf)?;
    for node in &document.epilog {
        if !is_document_whitespace(node) {
            output.write_all(b"\n")?;
            write_xml_node(output, node)?;
        }
    }
    Ok(())
}

fn is_adf_processing_instruction(node: &XmlNode<'_>) -> bool {
    let XmlNode::ProcessingInstruction(value) = node else {
        return false;
    };
    value
        .split_ascii_whitespace()
        .next()
        .is_some_and(|target| target.eq_ignore_ascii_case("adf"))
}

fn is_document_whitespace(node: &XmlNode<'_>) -> bool {
    matches!(node, XmlNode::Text(value) if value.trim().is_empty())
}

fn write_prospect(writer: &mut Output<'_>, prospect: &Prospect<'_>) -> Result<()> {
    let known = [("status", prospect.status.as_deref())];
    start_preserving_known(writer, "prospect", &prospect.attributes, &known)?;
    if prospect.extensions.is_empty() {
        for id in &prospect.ids {
            write_id(writer, id)?;
        }
        write_opt_text(writer, "requestdate", &prospect.request_date)?;
        for vehicle in &prospect.vehicles {
            write_vehicle(writer, vehicle)?;
        }
        if let Some(customer) = &prospect.customer {
            write_customer(writer, customer)?;
        }
        if let Some(vendor) = &prospect.vendor {
            write_vendor(writer, vendor)?;
        }
        if let Some(provider) = &prospect.provider {
            write_provider(writer, provider)?;
        }
        return end(writer, "prospect");
    }
    let mut children = Vec::new();
    for id in &prospect.ids {
        children.push(Child::Id {
            value: id,
            order: 0,
        });
    }
    push_opt_text(&mut children, "requestdate", &prospect.request_date, 1);
    for vehicle in &prospect.vehicles {
        children.push(Child::Vehicle {
            value: vehicle,
            order: 2,
        });
    }
    if let Some(customer) = &prospect.customer {
        children.push(Child::Customer {
            value: customer,
            order: 3,
        });
    }
    if let Some(vendor) = &prospect.vendor {
        children.push(Child::Vendor {
            value: vendor,
            order: 4,
        });
    }
    if let Some(provider) = &prospect.provider {
        children.push(Child::Provider {
            value: provider,
            order: 5,
        });
    }
    for extension in &prospect.extensions {
        children.push(Child::Extension {
            value: extension,
            order: 6,
        });
    }
    write_children(writer, &mut children)?;
    end(writer, "prospect")
}

fn write_vehicle(writer: &mut Output<'_>, vehicle: &Vehicle<'_>) -> Result<()> {
    let known = [
        ("interest", vehicle.interest.as_deref()),
        ("status", vehicle.status.as_deref()),
    ];
    start_preserving_known(writer, "vehicle", &vehicle.attributes, &known)?;
    if vehicle.extensions.is_empty() {
        for id in &vehicle.ids {
            write_id(writer, id)?;
        }
        write_opt_text(writer, "year", &vehicle.year)?;
        write_opt_text(writer, "make", &vehicle.make)?;
        write_opt_text(writer, "model", &vehicle.model)?;
        write_opt_text(writer, "vin", &vehicle.vin)?;
        write_opt_text(writer, "stock", &vehicle.stock)?;
        write_opt_text(writer, "trim", &vehicle.trim)?;
        write_opt_text(writer, "doors", &vehicle.doors)?;
        write_opt_text(writer, "bodystyle", &vehicle.body_style)?;
        write_opt_text(writer, "transmission", &vehicle.transmission)?;
        write_opt_text(writer, "odometer", &vehicle.odometer)?;
        write_opt_text(writer, "condition", &vehicle.condition)?;
        for colors in &vehicle.color_combinations {
            write_color_combination(writer, colors)?;
        }
        for image in &vehicle.image_tags {
            write_text(writer, "imagetag", image)?;
        }
        for price in &vehicle.prices {
            write_price(writer, price)?;
        }
        write_opt_text(writer, "pricecomments", &vehicle.price_comments)?;
        for option in &vehicle.options {
            write_option(writer, option)?;
        }
        if let Some(finance) = &vehicle.finance {
            write_finance(writer, finance)?;
        }
        write_opt_text(writer, "comments", &vehicle.comments)?;
        return end(writer, "vehicle");
    }
    let mut children = Vec::new();
    for id in &vehicle.ids {
        children.push(Child::Id {
            value: id,
            order: 0,
        });
    }
    push_opt_text(&mut children, "year", &vehicle.year, 1);
    push_opt_text(&mut children, "make", &vehicle.make, 2);
    push_opt_text(&mut children, "model", &vehicle.model, 3);
    push_opt_text(&mut children, "vin", &vehicle.vin, 4);
    push_opt_text(&mut children, "stock", &vehicle.stock, 5);
    push_opt_text(&mut children, "trim", &vehicle.trim, 6);
    push_opt_text(&mut children, "doors", &vehicle.doors, 7);
    push_opt_text(&mut children, "bodystyle", &vehicle.body_style, 8);
    push_opt_text(&mut children, "transmission", &vehicle.transmission, 9);
    push_opt_text(&mut children, "odometer", &vehicle.odometer, 10);
    push_opt_text(&mut children, "condition", &vehicle.condition, 11);
    for colors in &vehicle.color_combinations {
        children.push(Child::ColorCombination {
            value: colors,
            order: 12,
        });
    }
    for image in &vehicle.image_tags {
        children.push(Child::Text {
            name: "imagetag",
            value: image,
            order: 13,
        });
    }
    for price in &vehicle.prices {
        children.push(Child::Price {
            value: price,
            order: 14,
        });
    }
    push_opt_text(&mut children, "pricecomments", &vehicle.price_comments, 15);
    for option in &vehicle.options {
        children.push(Child::VehicleOption {
            value: option,
            order: 16,
        });
    }
    if let Some(finance) = &vehicle.finance {
        children.push(Child::Finance {
            value: finance,
            order: 17,
        });
    }
    push_opt_text(&mut children, "comments", &vehicle.comments, 18);
    for extension in &vehicle.extensions {
        children.push(Child::Extension {
            value: extension,
            order: 19,
        });
    }
    write_children(writer, &mut children)?;
    end(writer, "vehicle")
}

fn write_color_combination(writer: &mut Output<'_>, colors: &ColorCombination<'_>) -> Result<()> {
    start_with_attrs(writer, "colorcombination", &colors.attributes)?;
    if colors.extensions.is_empty() {
        write_opt_text(writer, "interiorcolor", &colors.interior_color)?;
        write_opt_text(writer, "exteriorcolor", &colors.exterior_color)?;
        write_opt_text(writer, "preference", &colors.preference)?;
        return end(writer, "colorcombination");
    }
    let mut children = Vec::new();
    push_opt_text(&mut children, "interiorcolor", &colors.interior_color, 0);
    push_opt_text(&mut children, "exteriorcolor", &colors.exterior_color, 1);
    push_opt_text(&mut children, "preference", &colors.preference, 2);
    for extension in &colors.extensions {
        children.push(Child::Extension {
            value: extension,
            order: 3,
        });
    }
    write_children(writer, &mut children)?;
    end(writer, "colorcombination")
}

fn write_option(writer: &mut Output<'_>, option: &VehicleOption<'_>) -> Result<()> {
    start_with_attrs(writer, "option", &option.attributes)?;
    if option.extensions.is_empty() {
        write_opt_text(writer, "optionname", &option.option_name)?;
        write_opt_text(writer, "manufacturercode", &option.manufacturer_code)?;
        write_opt_text(writer, "stock", &option.stock)?;
        write_opt_text(writer, "weighting", &option.weighting)?;
        for price in &option.prices {
            write_price(writer, price)?;
        }
        return end(writer, "option");
    }
    let mut children = Vec::new();
    push_opt_text(&mut children, "optionname", &option.option_name, 0);
    push_opt_text(
        &mut children,
        "manufacturercode",
        &option.manufacturer_code,
        1,
    );
    push_opt_text(&mut children, "stock", &option.stock, 2);
    push_opt_text(&mut children, "weighting", &option.weighting, 3);
    for price in &option.prices {
        children.push(Child::Price {
            value: price,
            order: 4,
        });
    }
    for extension in &option.extensions {
        children.push(Child::Extension {
            value: extension,
            order: 5,
        });
    }
    write_children(writer, &mut children)?;
    end(writer, "option")
}

fn write_finance(writer: &mut Output<'_>, finance: &Finance<'_>) -> Result<()> {
    start_with_attrs(writer, "finance", &finance.attributes)?;
    if finance.extensions.is_empty() {
        write_opt_text(writer, "method", &finance.method)?;
        for amount in &finance.amounts {
            write_text(writer, "amount", amount)?;
        }
        for balance in &finance.balances {
            write_text(writer, "balance", balance)?;
        }
        return end(writer, "finance");
    }
    let mut children = Vec::new();
    push_opt_text(&mut children, "method", &finance.method, 0);
    for amount in &finance.amounts {
        children.push(Child::Text {
            name: "amount",
            value: amount,
            order: 1,
        });
    }
    for balance in &finance.balances {
        children.push(Child::Text {
            name: "balance",
            value: balance,
            order: 2,
        });
    }
    for extension in &finance.extensions {
        children.push(Child::Extension {
            value: extension,
            order: 3,
        });
    }
    write_children(writer, &mut children)?;
    end(writer, "finance")
}

fn write_customer(writer: &mut Output<'_>, customer: &Customer<'_>) -> Result<()> {
    start_with_attrs(writer, "customer", &customer.attributes)?;
    if customer.extensions.is_empty() {
        for contact in &customer.contacts {
            write_contact(writer, contact)?;
        }
        for id in &customer.ids {
            write_id(writer, id)?;
        }
        if let Some(timeframe) = &customer.timeframe {
            write_timeframe(writer, timeframe)?;
        }
        write_opt_text(writer, "comments", &customer.comments)?;
        return end(writer, "customer");
    }
    let mut children = Vec::new();
    for contact in &customer.contacts {
        children.push(Child::Contact {
            value: contact,
            order: 0,
        });
    }
    for id in &customer.ids {
        children.push(Child::Id {
            value: id,
            order: 1,
        });
    }
    if let Some(timeframe) = &customer.timeframe {
        children.push(Child::Timeframe {
            value: timeframe,
            order: 2,
        });
    }
    push_opt_text(&mut children, "comments", &customer.comments, 3);
    for extension in &customer.extensions {
        children.push(Child::Extension {
            value: extension,
            order: 4,
        });
    }
    write_children(writer, &mut children)?;
    end(writer, "customer")
}

fn write_timeframe(writer: &mut Output<'_>, timeframe: &Timeframe<'_>) -> Result<()> {
    start_with_attrs(writer, "timeframe", &timeframe.attributes)?;
    if timeframe.extensions.is_empty() {
        write_opt_text(writer, "description", &timeframe.description)?;
        write_opt_text(writer, "earliestdate", &timeframe.earliest_date)?;
        write_opt_text(writer, "latestdate", &timeframe.latest_date)?;
        return end(writer, "timeframe");
    }
    let mut children = Vec::new();
    push_opt_text(&mut children, "description", &timeframe.description, 0);
    push_opt_text(&mut children, "earliestdate", &timeframe.earliest_date, 1);
    push_opt_text(&mut children, "latestdate", &timeframe.latest_date, 2);
    for extension in &timeframe.extensions {
        children.push(Child::Extension {
            value: extension,
            order: 3,
        });
    }
    write_children(writer, &mut children)?;
    end(writer, "timeframe")
}

fn write_vendor(writer: &mut Output<'_>, vendor: &Vendor<'_>) -> Result<()> {
    start_with_attrs(writer, "vendor", &vendor.attributes)?;
    if vendor.extensions.is_empty() {
        for id in &vendor.ids {
            write_id(writer, id)?;
        }
        write_opt_text(writer, "vendorname", &vendor.vendor_name)?;
        write_opt_text(writer, "url", &vendor.url)?;
        for contact in &vendor.contacts {
            write_contact(writer, contact)?;
        }
        return end(writer, "vendor");
    }
    let mut children = Vec::new();
    for id in &vendor.ids {
        children.push(Child::Id {
            value: id,
            order: 0,
        });
    }
    push_opt_text(&mut children, "vendorname", &vendor.vendor_name, 1);
    push_opt_text(&mut children, "url", &vendor.url, 2);
    for contact in &vendor.contacts {
        children.push(Child::Contact {
            value: contact,
            order: 3,
        });
    }
    for extension in &vendor.extensions {
        children.push(Child::Extension {
            value: extension,
            order: 4,
        });
    }
    write_children(writer, &mut children)?;
    end(writer, "vendor")
}

fn write_provider(writer: &mut Output<'_>, provider: &Provider<'_>) -> Result<()> {
    start_with_attrs(writer, "provider", &provider.attributes)?;
    if provider.extensions.is_empty() {
        for id in &provider.ids {
            write_id(writer, id)?;
        }
        if let Some(name) = &provider.name {
            write_name(writer, name)?;
        }
        write_opt_text(writer, "service", &provider.service)?;
        write_opt_text(writer, "url", &provider.url)?;
        write_opt_text(writer, "email", &provider.email)?;
        write_opt_text(writer, "phone", &provider.phone)?;
        for contact in &provider.contacts {
            write_contact(writer, contact)?;
        }
        return end(writer, "provider");
    }
    let mut children = Vec::new();
    for id in &provider.ids {
        children.push(Child::Id {
            value: id,
            order: 0,
        });
    }
    if let Some(name) = &provider.name {
        children.push(Child::Name {
            value: name,
            order: 1,
        });
    }
    push_opt_text(&mut children, "service", &provider.service, 2);
    push_opt_text(&mut children, "url", &provider.url, 3);
    push_opt_text(&mut children, "email", &provider.email, 4);
    push_opt_text(&mut children, "phone", &provider.phone, 5);
    for contact in &provider.contacts {
        children.push(Child::Contact {
            value: contact,
            order: 6,
        });
    }
    for extension in &provider.extensions {
        children.push(Child::Extension {
            value: extension,
            order: 7,
        });
    }
    write_children(writer, &mut children)?;
    end(writer, "provider")
}

fn write_contact(writer: &mut Output<'_>, contact: &Contact<'_>) -> Result<()> {
    let known = [("primarycontact", contact.primary_contact.as_deref())];
    start_preserving_known(writer, "contact", &contact.attributes, &known)?;
    if contact.extensions.is_empty() {
        for name in &contact.names {
            write_name(writer, name)?;
        }
        for email in &contact.emails {
            write_text(writer, "email", email)?;
        }
        for phone in &contact.phones {
            write_text(writer, "phone", phone)?;
        }
        for address in &contact.addresses {
            write_address(writer, address)?;
        }
        return end(writer, "contact");
    }
    let mut children = Vec::new();
    for name in &contact.names {
        children.push(Child::Name {
            value: name,
            order: 0,
        });
    }
    for email in &contact.emails {
        children.push(Child::Text {
            name: "email",
            value: email,
            order: 1,
        });
    }
    for phone in &contact.phones {
        children.push(Child::Text {
            name: "phone",
            value: phone,
            order: 2,
        });
    }
    for address in &contact.addresses {
        children.push(Child::Address {
            value: address,
            order: 3,
        });
    }
    for extension in &contact.extensions {
        children.push(Child::Extension {
            value: extension,
            order: 4,
        });
    }
    write_children(writer, &mut children)?;
    end(writer, "contact")
}

fn write_address(writer: &mut Output<'_>, address: &Address<'_>) -> Result<()> {
    let known = [("type", address.address_type.as_deref())];
    start_preserving_known(writer, "address", &address.attributes, &known)?;
    if address.extensions.is_empty() {
        for street in &address.streets {
            write_text(writer, "street", street)?;
        }
        write_opt_text(writer, "apartment", &address.apartment)?;
        write_opt_text(writer, "city", &address.city)?;
        write_opt_text(writer, "regioncode", &address.region_code)?;
        write_opt_text(writer, "postalcode", &address.postal_code)?;
        write_opt_text(writer, "country", &address.country)?;
        return end(writer, "address");
    }
    let mut children = Vec::new();
    for street in &address.streets {
        children.push(Child::Text {
            name: "street",
            value: street,
            order: 0,
        });
    }
    push_opt_text(&mut children, "apartment", &address.apartment, 1);
    push_opt_text(&mut children, "city", &address.city, 2);
    push_opt_text(&mut children, "regioncode", &address.region_code, 3);
    push_opt_text(&mut children, "postalcode", &address.postal_code, 4);
    push_opt_text(&mut children, "country", &address.country, 5);
    for extension in &address.extensions {
        children.push(Child::Extension {
            value: extension,
            order: 6,
        });
    }
    write_children(writer, &mut children)?;
    end(writer, "address")
}

fn write_id(writer: &mut Output<'_>, id: &Id<'_>) -> Result<()> {
    let known = [
        ("sequence", id.sequence.as_deref()),
        ("source", id.source.as_deref()),
    ];
    write_parts(writer, "id", &id.attributes, &known, &id.parts)
}

fn write_price(writer: &mut Output<'_>, price: &Price<'_>) -> Result<()> {
    let known = [
        ("type", price.price_type.as_deref()),
        ("currency", price.currency.as_deref()),
        ("delta", price.delta.as_deref()),
        ("relativeto", price.relative_to.as_deref()),
        ("source", price.source.as_deref()),
    ];
    write_parts(writer, "price", &price.attributes, &known, &price.parts)
}

fn write_name(writer: &mut Output<'_>, name: &Name<'_>) -> Result<()> {
    let known = [
        ("part", name.part.as_deref()),
        ("type", name.name_type.as_deref()),
    ];
    write_parts(writer, "name", &name.attributes, &known, &name.parts)
}

enum Child<'model, 'input> {
    Prospect {
        value: &'model Prospect<'input>,
        order: usize,
    },
    Vehicle {
        value: &'model Vehicle<'input>,
        order: usize,
    },
    ColorCombination {
        value: &'model ColorCombination<'input>,
        order: usize,
    },
    VehicleOption {
        value: &'model VehicleOption<'input>,
        order: usize,
    },
    Finance {
        value: &'model Finance<'input>,
        order: usize,
    },
    Customer {
        value: &'model Customer<'input>,
        order: usize,
    },
    Timeframe {
        value: &'model Timeframe<'input>,
        order: usize,
    },
    Vendor {
        value: &'model Vendor<'input>,
        order: usize,
    },
    Provider {
        value: &'model Provider<'input>,
        order: usize,
    },
    Contact {
        value: &'model Contact<'input>,
        order: usize,
    },
    Address {
        value: &'model Address<'input>,
        order: usize,
    },
    Id {
        value: &'model Id<'input>,
        order: usize,
    },
    Price {
        value: &'model Price<'input>,
        order: usize,
    },
    Name {
        value: &'model Name<'input>,
        order: usize,
    },
    Text {
        name: &'static str,
        value: &'model TextElement<'input>,
        order: usize,
    },
    Extension {
        value: &'model XmlNode<'input>,
        order: usize,
    },
}

impl Child<'_, '_> {
    fn span(&self) -> Span {
        match self {
            Child::Prospect { value, .. } => value.span,
            Child::Vehicle { value, .. } => value.span,
            Child::ColorCombination { value, .. } => value.span,
            Child::VehicleOption { value, .. } => value.span,
            Child::Finance { value, .. } => value.span,
            Child::Customer { value, .. } => value.span,
            Child::Timeframe { value, .. } => value.span,
            Child::Vendor { value, .. } => value.span,
            Child::Provider { value, .. } => value.span,
            Child::Contact { value, .. } => value.span,
            Child::Address { value, .. } => value.span,
            Child::Id { value, .. } => value.span,
            Child::Price { value, .. } => value.span,
            Child::Name { value, .. } => value.span,
            Child::Text { value, .. } => value.span,
            Child::Extension { value, .. } => xml_node_span(value),
        }
    }

    fn order(&self) -> usize {
        match self {
            Child::Prospect { order, .. }
            | Child::Vehicle { order, .. }
            | Child::ColorCombination { order, .. }
            | Child::VehicleOption { order, .. }
            | Child::Finance { order, .. }
            | Child::Customer { order, .. }
            | Child::Timeframe { order, .. }
            | Child::Vendor { order, .. }
            | Child::Provider { order, .. }
            | Child::Contact { order, .. }
            | Child::Address { order, .. }
            | Child::Id { order, .. }
            | Child::Price { order, .. }
            | Child::Name { order, .. }
            | Child::Text { order, .. }
            | Child::Extension { order, .. } => *order,
        }
    }

    fn is_extension(&self) -> bool {
        matches!(self, Child::Extension { .. })
    }
}

fn push_opt_text<'model, 'input>(
    children: &mut Vec<Child<'model, 'input>>,
    name: &'static str,
    value: &'model Option<TextElement<'input>>,
    order: usize,
) {
    if let Some(value) = value {
        children.push(Child::Text { name, value, order });
    }
}

fn write_children(writer: &mut Output<'_>, children: &mut [Child<'_, '_>]) -> Result<()> {
    let mut known_spans: Vec<_> = children
        .iter()
        .filter(|child| !child.is_extension())
        .filter_map(|child| {
            let span = child.span();
            if span.start == 0 && span.end == 0 {
                None
            } else {
                Some((span.start, child.order()))
            }
        })
        .collect();
    known_spans.sort_by_key(|(start, _)| *start);
    let mut max_order = 0;
    for (_, order) in &mut known_spans {
        max_order = max_order.max(*order);
        *order = max_order;
    }

    children.sort_by_cached_key(|child| {
        let span = child.span();
        if span.start == 0 && span.end == 0 {
            return (1, child.order(), usize::MAX);
        }

        if child.is_extension() {
            let index = known_spans.partition_point(|(known_start, _)| *known_start < span.start);
            let order = if index == 0 {
                1
            } else {
                known_spans[index - 1].1.saturating_mul(2).saturating_add(3)
            };
            (0, order, span.start)
        } else {
            (
                0,
                child.order().saturating_mul(2).saturating_add(2),
                span.start,
            )
        }
    });

    for child in children {
        match child {
            Child::Prospect { value, .. } => write_prospect(writer, value)?,
            Child::Vehicle { value, .. } => write_vehicle(writer, value)?,
            Child::ColorCombination { value, .. } => write_color_combination(writer, value)?,
            Child::VehicleOption { value, .. } => write_option(writer, value)?,
            Child::Finance { value, .. } => write_finance(writer, value)?,
            Child::Customer { value, .. } => write_customer(writer, value)?,
            Child::Timeframe { value, .. } => write_timeframe(writer, value)?,
            Child::Vendor { value, .. } => write_vendor(writer, value)?,
            Child::Provider { value, .. } => write_provider(writer, value)?,
            Child::Contact { value, .. } => write_contact(writer, value)?,
            Child::Address { value, .. } => write_address(writer, value)?,
            Child::Id { value, .. } => write_id(writer, value)?,
            Child::Price { value, .. } => write_price(writer, value)?,
            Child::Name { value, .. } => write_name(writer, value)?,
            Child::Text { name, value, .. } => write_text(writer, name, value)?,
            Child::Extension { value, .. } => write_xml_node(writer, value)?,
        }
    }

    Ok(())
}

fn xml_node_span(node: &XmlNode<'_>) -> Span {
    match node {
        XmlNode::Element(element) => element.span,
        XmlNode::Text(_)
        | XmlNode::CData(_)
        | XmlNode::EntityRef(_)
        | XmlNode::Comment(_)
        | XmlNode::ProcessingInstruction(_)
        | XmlNode::Declaration(_)
        | XmlNode::DocType(_) => Span::default(),
    }
}

fn write_opt_text(
    writer: &mut Output<'_>,
    name: &str,
    value: &Option<TextElement<'_>>,
) -> Result<()> {
    if let Some(value) = value {
        write_text(writer, name, value)?;
    }
    Ok(())
}

fn write_text(writer: &mut Output<'_>, name: &str, value: &TextElement<'_>) -> Result<()> {
    write_parts(writer, name, &value.attributes, &[], &value.parts)
}

fn write_parts(
    writer: &mut Output<'_>,
    name: &str,
    attributes: &[Attribute<'_>],
    known: &[(&'static str, Option<&str>)],
    parts: &[TextPart<'_>],
) -> Result<()> {
    start_preserving_known(writer, name, attributes, known)?;
    write_text_parts(writer, parts)?;
    end(writer, name)
}

fn write_text_parts(writer: &mut Output<'_>, parts: &[TextPart<'_>]) -> Result<()> {
    for part in parts {
        match part {
            TextPart::Text(text) => write_escaped_text(writer, text)?,
            TextPart::CData(text) => write_cdata(writer, text)?,
            TextPart::EntityRef(name) => {
                write_entity_ref(writer, name)?;
            }
            TextPart::Node(node) => write_xml_node(writer, node)?,
        }
    }
    Ok(())
}

fn write_cdata(writer: &mut Output<'_>, text: &str) -> Result<()> {
    validate_xml_chars(text)?;
    let mut remaining = text;
    loop {
        if let Some(index) = remaining.find("]]>") {
            let (head, tail) = remaining.split_at(index + 2);
            writer.write_all(b"<![CDATA[")?;
            writer.write_all(head.as_bytes())?;
            writer.write_all(b"]]>")?;
            remaining = tail;
        } else {
            writer.write_all(b"<![CDATA[")?;
            writer.write_all(remaining.as_bytes())?;
            writer.write_all(b"]]>")?;
            return Ok(());
        }
    }
}

fn write_xml_node(writer: &mut Output<'_>, node: &XmlNode<'_>) -> Result<()> {
    match node {
        XmlNode::Element(element) => write_xml_element(writer, element),
        XmlNode::Text(text) => write_escaped_text(writer, text),
        XmlNode::CData(text) => write_cdata(writer, text),
        XmlNode::EntityRef(name) => write_entity_ref(writer, name),
        XmlNode::Comment(comment) => {
            validate_comment(comment)?;
            writer.write_all(b"<!--")?;
            writer.write_all(comment.as_bytes())?;
            writer.write_all(b"-->")?;
            Ok(())
        }
        XmlNode::ProcessingInstruction(pi) => {
            validate_processing_instruction(pi)?;
            writer.write_all(b"<?")?;
            writer.write_all(pi.as_bytes())?;
            writer.write_all(b"?>")?;
            Ok(())
        }
        XmlNode::Declaration(decl) => {
            validate_wrapped_token("declaration", decl, "?>")?;
            writer.write_all(b"<?")?;
            writer.write_all(decl.as_bytes())?;
            writer.write_all(b"?>")?;
            Ok(())
        }
        XmlNode::DocType(doc_type) => {
            validate_doctype(doc_type)?;
            writer.write_all(b"<!DOCTYPE ")?;
            writer.write_all(doc_type.as_bytes())?;
            writer.write_all(b">")?;
            Ok(())
        }
    }
}

fn write_entity_ref(writer: &mut Output<'_>, name: &str) -> Result<()> {
    crate::parse::ensure_entity_name(name, 0)?;
    if writer
        .declared_entities
        .is_some_and(|entities| entities.contains(name))
    {
        writer.write_all(b"&")?;
        writer.write_all(name.as_bytes())?;
        writer.write_all(b";")?;
        return Ok(());
    }
    match writer.entity_policy {
        UnknownEntityPolicy::Error => Err(Error::UndeclaredEntityReference {
            name: name.to_owned(),
        }),
        UnknownEntityPolicy::Escape => write_escaped_text(writer, &format!("&{name};")),
        UnknownEntityPolicy::Preserve => {
            writer.write_all(b"&")?;
            writer.write_all(name.as_bytes())?;
            writer.write_all(b";")?;
            Ok(())
        }
    }
}

fn write_xml_element(writer: &mut Output<'_>, element: &XmlElement<'_>) -> Result<()> {
    start_with_attrs(writer, &element.name, &element.attributes)?;
    for child in &element.children {
        write_xml_node(writer, child)?;
    }
    end(writer, &element.name)
}

fn start_with_attrs(
    writer: &mut Output<'_>,
    name: &str,
    attributes: &[Attribute<'_>],
) -> Result<()> {
    start_open(writer, name)?;
    for attr in attributes {
        write_attr(writer, attr.name.as_ref(), attr.value.as_ref())?;
    }
    writer.write_all(b">")?;
    Ok(())
}

fn start_preserving_known(
    writer: &mut Output<'_>,
    name: &str,
    attributes: &[Attribute<'_>],
    known: &[(&'static str, Option<&str>)],
) -> Result<()> {
    start_open(writer, name)?;
    let mut emitted = 0_u64;
    for attr in attributes {
        if let Some(index) = known
            .iter()
            .position(|(known_name, _)| attr.name.as_ref() == *known_name)
        {
            let bit = 1_u64 << index;
            if emitted & bit == 0 {
                if let Some(value) = known[index].1 {
                    write_attr(writer, known[index].0, value)?;
                }
                emitted |= bit;
            }
        } else {
            write_attr(writer, attr.name.as_ref(), attr.value.as_ref())?;
        }
    }

    for (index, (name, value)) in known.iter().enumerate() {
        let bit = 1_u64 << index;
        if emitted & bit == 0 {
            if let Some(value) = value {
                write_attr(writer, name, value)?;
            }
        }
    }

    writer.write_all(b">")?;
    Ok(())
}

fn start_open(writer: &mut Output<'_>, name: &str) -> Result<()> {
    validate_name(name, "element")?;
    writer.write_all(b"\n<")?;
    writer.write_all(name.as_bytes())?;
    Ok(())
}

fn write_attr(writer: &mut Output<'_>, name: &str, value: &str) -> Result<()> {
    validate_name(name, "attribute")?;
    writer.write_all(b" ")?;
    writer.write_all(name.as_bytes())?;
    writer.write_all(b"=\"")?;
    write_escaped_attr(writer, value)?;
    writer.write_all(b"\"")?;
    Ok(())
}

fn end(writer: &mut Output<'_>, name: &str) -> Result<()> {
    validate_name(name, "element")?;
    writer.write_all(b"</")?;
    writer.write_all(name.as_bytes())?;
    writer.write_all(b">")?;
    Ok(())
}

fn write_escaped_text(writer: &mut Output<'_>, value: &str) -> Result<()> {
    validate_xml_chars(value)?;
    write_escaped(writer, value, text_escape)
}

fn write_escaped_attr(writer: &mut Output<'_>, value: &str) -> Result<()> {
    validate_xml_chars(value)?;
    write_escaped(writer, value, attr_escape)
}

fn validate_name(name: &str, kind: &'static str) -> Result<()> {
    if crate::parse::is_xml_name(name) {
        Ok(())
    } else {
        Err(Error::InvalidName { kind })
    }
}

fn validate_xml_chars(value: &str) -> Result<()> {
    crate::parse::ensure_xml_chars(value, 0)
}

fn validate_comment(comment: &str) -> Result<()> {
    validate_xml_chars(comment)?;
    if comment.contains("--") {
        return Err(Error::InvalidXmlToken {
            kind: "comment",
            reason: "contains forbidden `--` sequence",
        });
    }
    if comment.ends_with('-') {
        return Err(Error::InvalidXmlToken {
            kind: "comment",
            reason: "ends with `-`",
        });
    }
    Ok(())
}

fn validate_processing_instruction(pi: &str) -> Result<()> {
    validate_wrapped_token("processing instruction", pi, "?>")?;
    let target = pi.split_whitespace().next().unwrap_or(pi);
    if target.is_empty() {
        return Err(Error::InvalidXmlToken {
            kind: "processing instruction",
            reason: "missing target",
        });
    }
    if target.eq_ignore_ascii_case("xml") {
        return Err(Error::InvalidXmlToken {
            kind: "processing instruction",
            reason: "target `xml` is reserved",
        });
    }
    validate_name(target, "processing instruction target")
}

fn validate_wrapped_token(kind: &'static str, value: &str, forbidden: &'static str) -> Result<()> {
    validate_xml_chars(value)?;
    if value.contains(forbidden) {
        return Err(Error::InvalidXmlToken {
            kind,
            reason: "contains its closing delimiter",
        });
    }
    Ok(())
}

fn validate_doctype(value: &str) -> Result<()> {
    validate_xml_chars(value)?;
    if value.trim().is_empty() {
        return Err(Error::InvalidXmlToken {
            kind: "DOCTYPE",
            reason: "value is empty",
        });
    }
    let probe = format!("<!DOCTYPE {value}><adf/>");
    let options = crate::ParseOptions::default()
        .without_doctype_limit()
        .without_input_limit()
        .without_depth_limit()
        .without_node_limit()
        .without_attribute_limit();
    let valid = crate::parse::parse_document_tree(&probe, &options)
        .ok()
        .is_some_and(|document| {
            matches!(
                document.prolog.as_slice(),
                [XmlNode::DocType(parsed)] if parsed.as_ref() == value
            )
        });
    if !valid {
        return Err(Error::InvalidXmlToken {
            kind: "DOCTYPE",
            reason: "payload is not one complete XML DOCTYPE declaration",
        });
    }
    Ok(())
}

fn write_escaped(
    writer: &mut Output<'_>,
    value: &str,
    escape: fn(u8) -> Option<&'static [u8]>,
) -> Result<()> {
    let bytes = value.as_bytes();
    let mut chunk_start = 0;

    for (index, byte) in bytes.iter().copied().enumerate() {
        if let Some(replacement) = escape(byte) {
            if chunk_start < index {
                writer.write_all(&bytes[chunk_start..index])?;
            }
            writer.write_all(replacement)?;
            chunk_start = index + 1;
        }
    }

    if chunk_start < bytes.len() {
        writer.write_all(&bytes[chunk_start..])?;
    }

    Ok(())
}

fn text_escape(byte: u8) -> Option<&'static [u8]> {
    match byte {
        b'&' => Some(b"&amp;"),
        b'<' => Some(b"&lt;"),
        b'>' => Some(b"&gt;"),
        _ => None,
    }
}

fn attr_escape(byte: u8) -> Option<&'static [u8]> {
    match byte {
        b'&' => Some(b"&amp;"),
        b'<' => Some(b"&lt;"),
        b'>' => Some(b"&gt;"),
        b'"' => Some(b"&quot;"),
        b'\'' => Some(b"&apos;"),
        _ => None,
    }
}
