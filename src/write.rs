use crate::Result;
use crate::document::{AdfDocument, Attribute, XmlElement, XmlNode};
use crate::model::*;
use std::borrow::Cow;
use std::io::Write;

pub(crate) fn write_adf<W: Write>(mut writer: W, adf: &Adf<'_>) -> Result<()> {
    writer.write_all(br#"<?xml version="1.0"?>"#)?;
    writer.write_all(b"\n<?adf version=\"1.0\"?>\n<adf>")?;
    for prospect in &adf.prospects {
        write_prospect(&mut writer, prospect)?;
    }
    for extension in &adf.extensions {
        write_xml_node(&mut writer, extension)?;
    }
    writer.write_all(b"\n</adf>")?;
    Ok(())
}

pub(crate) fn write_original_preserving<W: Write>(
    mut writer: W,
    document: &AdfDocument<'_>,
) -> Result<()> {
    if document.dirty_all {
        return write_adf(writer, &document.adf);
    }

    let mut cursor = 0;
    for (index, dirty) in document.dirty_prospects.iter().enumerate() {
        if !dirty {
            continue;
        }
        let Some(span) = document.prospect_spans.get(index) else {
            return write_adf(writer, &document.adf);
        };
        let Some(prospect) = document.adf.prospects.get(index) else {
            return write_adf(writer, &document.adf);
        };

        writer.write_all(&document.original.as_bytes()[cursor..span.start])?;
        write_prospect(&mut writer, prospect)?;
        cursor = span.end;
    }

    writer.write_all(&document.original.as_bytes()[cursor..])?;
    Ok(())
}

pub(crate) fn write_prospect<W: Write>(writer: &mut W, prospect: &Prospect<'_>) -> Result<()> {
    start(
        writer,
        "prospect",
        &[maybe_attr("status", &prospect.status)],
    )?;
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
    for extension in &prospect.extensions {
        write_xml_node(writer, extension)?;
    }
    end(writer, "prospect")
}

fn write_vehicle<W: Write>(writer: &mut W, vehicle: &Vehicle<'_>) -> Result<()> {
    start(
        writer,
        "vehicle",
        &[
            maybe_attr("interest", &vehicle.interest),
            maybe_attr("status", &vehicle.status),
        ],
    )?;
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
    for extension in &vehicle.extensions {
        write_xml_node(writer, extension)?;
    }
    end(writer, "vehicle")
}

fn write_color_combination<W: Write>(writer: &mut W, colors: &ColorCombination<'_>) -> Result<()> {
    start(writer, "colorcombination", &[])?;
    write_opt_text(writer, "interiorcolor", &colors.interior_color)?;
    write_opt_text(writer, "exteriorcolor", &colors.exterior_color)?;
    write_opt_text(writer, "preference", &colors.preference)?;
    for extension in &colors.extensions {
        write_xml_node(writer, extension)?;
    }
    end(writer, "colorcombination")
}

fn write_option<W: Write>(writer: &mut W, option: &VehicleOption<'_>) -> Result<()> {
    start(writer, "option", &[])?;
    write_opt_text(writer, "optionname", &option.option_name)?;
    write_opt_text(writer, "manufacturercode", &option.manufacturer_code)?;
    write_opt_text(writer, "stock", &option.stock)?;
    write_opt_text(writer, "weighting", &option.weighting)?;
    for price in &option.prices {
        write_price(writer, price)?;
    }
    for extension in &option.extensions {
        write_xml_node(writer, extension)?;
    }
    end(writer, "option")
}

fn write_finance<W: Write>(writer: &mut W, finance: &Finance<'_>) -> Result<()> {
    start(writer, "finance", &[])?;
    write_opt_text(writer, "method", &finance.method)?;
    for amount in &finance.amounts {
        write_text(writer, "amount", amount)?;
    }
    for balance in &finance.balances {
        write_text(writer, "balance", balance)?;
    }
    for extension in &finance.extensions {
        write_xml_node(writer, extension)?;
    }
    end(writer, "finance")
}

fn write_customer<W: Write>(writer: &mut W, customer: &Customer<'_>) -> Result<()> {
    start(writer, "customer", &[])?;
    for id in &customer.ids {
        write_id(writer, id)?;
    }
    for contact in &customer.contacts {
        write_contact(writer, contact)?;
    }
    if let Some(timeframe) = &customer.timeframe {
        write_timeframe(writer, timeframe)?;
    }
    write_opt_text(writer, "comments", &customer.comments)?;
    for extension in &customer.extensions {
        write_xml_node(writer, extension)?;
    }
    end(writer, "customer")
}

fn write_timeframe<W: Write>(writer: &mut W, timeframe: &Timeframe<'_>) -> Result<()> {
    start(writer, "timeframe", &[])?;
    write_opt_text(writer, "description", &timeframe.description)?;
    write_opt_text(writer, "earliestdate", &timeframe.earliest_date)?;
    write_opt_text(writer, "latestdate", &timeframe.latest_date)?;
    for extension in &timeframe.extensions {
        write_xml_node(writer, extension)?;
    }
    end(writer, "timeframe")
}

fn write_vendor<W: Write>(writer: &mut W, vendor: &Vendor<'_>) -> Result<()> {
    start(writer, "vendor", &[])?;
    for id in &vendor.ids {
        write_id(writer, id)?;
    }
    write_opt_text(writer, "vendorname", &vendor.vendor_name)?;
    write_opt_text(writer, "url", &vendor.url)?;
    for contact in &vendor.contacts {
        write_contact(writer, contact)?;
    }
    for extension in &vendor.extensions {
        write_xml_node(writer, extension)?;
    }
    end(writer, "vendor")
}

fn write_provider<W: Write>(writer: &mut W, provider: &Provider<'_>) -> Result<()> {
    start(writer, "provider", &[])?;
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
    for extension in &provider.extensions {
        write_xml_node(writer, extension)?;
    }
    end(writer, "provider")
}

fn write_contact<W: Write>(writer: &mut W, contact: &Contact<'_>) -> Result<()> {
    start(
        writer,
        "contact",
        &[maybe_attr("primarycontact", &contact.primary_contact)],
    )?;
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
    for extension in &contact.extensions {
        write_xml_node(writer, extension)?;
    }
    end(writer, "contact")
}

fn write_address<W: Write>(writer: &mut W, address: &Address<'_>) -> Result<()> {
    start(
        writer,
        "address",
        &[maybe_attr("type", &address.address_type)],
    )?;
    for street in &address.streets {
        write_text(writer, "street", street)?;
    }
    write_opt_text(writer, "apartment", &address.apartment)?;
    write_opt_text(writer, "city", &address.city)?;
    write_opt_text(writer, "regioncode", &address.region_code)?;
    write_opt_text(writer, "postalcode", &address.postal_code)?;
    write_opt_text(writer, "country", &address.country)?;
    for extension in &address.extensions {
        write_xml_node(writer, extension)?;
    }
    end(writer, "address")
}

fn write_id<W: Write>(writer: &mut W, id: &Id<'_>) -> Result<()> {
    let known = [
        ("sequence", id.sequence.as_deref()),
        ("source", id.source.as_deref()),
    ];
    let attrs = attrs_preserving_known(&id.attributes, &known);
    write_value(writer, "id", &attrs, &id.value)
}

fn write_price<W: Write>(writer: &mut W, price: &Price<'_>) -> Result<()> {
    let known = [
        ("type", price.price_type.as_deref()),
        ("currency", price.currency.as_deref()),
        ("delta", price.delta.as_deref()),
        ("relativeto", price.relative_to.as_deref()),
        ("source", price.source.as_deref()),
    ];
    let attrs = attrs_preserving_known(&price.attributes, &known);
    write_value(writer, "price", &attrs, &price.value)
}

fn write_name<W: Write>(writer: &mut W, name: &Name<'_>) -> Result<()> {
    let known = [
        ("part", name.part.as_deref()),
        ("type", name.name_type.as_deref()),
    ];
    let attrs = attrs_preserving_known(&name.attributes, &known);
    write_value(writer, "name", &attrs, &name.value)
}

fn write_opt_text<W: Write>(
    writer: &mut W,
    name: &str,
    value: &Option<TextElement<'_>>,
) -> Result<()> {
    if let Some(value) = value {
        write_text(writer, name, value)?;
    }
    Ok(())
}

fn write_text<W: Write>(writer: &mut W, name: &str, value: &TextElement<'_>) -> Result<()> {
    let attrs = attrs_from_slice(&value.attributes);
    write_value(writer, name, &attrs, &value.value)
}

fn write_value<W: Write>(
    writer: &mut W,
    name: &str,
    attrs: &[Option<(&str, &str)>],
    value: &str,
) -> Result<()> {
    start(writer, name, attrs)?;
    write_escaped_text(writer, value)?;
    end(writer, name)
}

fn write_xml_node<W: Write>(writer: &mut W, node: &XmlNode<'_>) -> Result<()> {
    match node {
        XmlNode::Element(element) => write_xml_element(writer, element),
        XmlNode::Text(text) => write_escaped_text(writer, text),
        XmlNode::CData(text) => {
            writer.write_all(b"<![CDATA[")?;
            writer.write_all(text.as_bytes())?;
            writer.write_all(b"]]>")?;
            Ok(())
        }
        XmlNode::Comment(comment) => {
            writer.write_all(b"<!--")?;
            writer.write_all(comment.as_bytes())?;
            writer.write_all(b"-->")?;
            Ok(())
        }
        XmlNode::ProcessingInstruction(pi) => {
            writer.write_all(b"<?")?;
            writer.write_all(pi.as_bytes())?;
            writer.write_all(b"?>")?;
            Ok(())
        }
        XmlNode::Declaration(decl) => {
            writer.write_all(b"<?")?;
            writer.write_all(decl.as_bytes())?;
            writer.write_all(b"?>")?;
            Ok(())
        }
        XmlNode::DocType(doc_type) => {
            writer.write_all(b"<!DOCTYPE ")?;
            writer.write_all(doc_type.as_bytes())?;
            writer.write_all(b">")?;
            Ok(())
        }
    }
}

fn write_xml_element<W: Write>(writer: &mut W, element: &XmlElement<'_>) -> Result<()> {
    let attrs = attrs_from_slice(&element.attributes);
    start(writer, &element.name, &attrs)?;
    for child in &element.children {
        write_xml_node(writer, child)?;
    }
    end(writer, &element.name)
}

fn start<W: Write>(writer: &mut W, name: &str, attrs: &[Option<(&str, &str)>]) -> Result<()> {
    writer.write_all(b"\n<")?;
    writer.write_all(name.as_bytes())?;
    for attr in attrs.iter().flatten() {
        writer.write_all(b" ")?;
        writer.write_all(attr.0.as_bytes())?;
        writer.write_all(b"=\"")?;
        write_escaped_attr(writer, attr.1)?;
        writer.write_all(b"\"")?;
    }
    writer.write_all(b">")?;
    Ok(())
}

fn end<W: Write>(writer: &mut W, name: &str) -> Result<()> {
    writer.write_all(b"</")?;
    writer.write_all(name.as_bytes())?;
    writer.write_all(b">")?;
    Ok(())
}

fn maybe_attr<'a>(name: &'a str, value: &'a Option<Cow<'_, str>>) -> Option<(&'a str, &'a str)> {
    value.as_deref().map(|value| (name, value))
}

fn attrs_from_slice<'a>(attributes: &'a [Attribute<'a>]) -> Vec<Option<(&'a str, &'a str)>> {
    attributes
        .iter()
        .map(|attr| Some((attr.name.as_ref(), attr.value.as_ref())))
        .collect()
}

fn attrs_preserving_known<'a>(
    attributes: &'a [Attribute<'a>],
    known: &[(&'static str, Option<&'a str>)],
) -> Vec<Option<(&'a str, &'a str)>> {
    let mut emitted = vec![false; known.len()];
    let mut output = Vec::with_capacity(attributes.len() + known.len());

    for attr in attributes {
        if let Some(index) = known
            .iter()
            .position(|(name, _)| attr.name.as_ref() == *name)
        {
            if !emitted[index] {
                if let Some(value) = known[index].1 {
                    output.push(Some((known[index].0, value)));
                }
                emitted[index] = true;
            }
        } else {
            output.push(Some((attr.name.as_ref(), attr.value.as_ref())));
        }
    }

    for (index, (name, value)) in known.iter().enumerate() {
        if let (false, Some(value)) = (emitted[index], value) {
            output.push(Some((*name, value)));
        }
    }

    output
}

fn write_escaped_text<W: Write>(writer: &mut W, value: &str) -> Result<()> {
    for ch in value.chars() {
        match ch {
            '&' => writer.write_all(b"&amp;")?,
            '<' => writer.write_all(b"&lt;")?,
            '>' => writer.write_all(b"&gt;")?,
            _ => write!(writer, "{ch}")?,
        }
    }
    Ok(())
}

fn write_escaped_attr<W: Write>(writer: &mut W, value: &str) -> Result<()> {
    for ch in value.chars() {
        match ch {
            '&' => writer.write_all(b"&amp;")?,
            '<' => writer.write_all(b"&lt;")?,
            '>' => writer.write_all(b"&gt;")?,
            '"' => writer.write_all(b"&quot;")?,
            '\'' => writer.write_all(b"&apos;")?,
            _ => write!(writer, "{ch}")?,
        }
    }
    Ok(())
}
