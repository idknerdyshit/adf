use crate::{Attribute, Span, XmlNode};
use std::borrow::Cow;

/// Root typed representation of an ADF document.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Adf<'a> {
    /// Parsed `<prospect>` children.
    pub prospects: Vec<Prospect<'a>>,
    /// Unknown root-level XML nodes retained for typed writing.
    pub extensions: Vec<XmlNode<'a>>,
    /// Root `<adf>` attributes, including namespace declarations.
    pub attributes: Vec<Attribute<'a>>,
    /// Byte span of the root element in the original input.
    pub span: Span,
}

/// ADF `<prospect>` lead record.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Prospect<'a> {
    /// Optional `status` attribute.
    pub status: Option<Cow<'a, str>>,
    /// Lead identifiers.
    pub ids: Vec<Id<'a>>,
    /// Request timestamp.
    pub request_date: Option<TextElement<'a>>,
    /// Requested or supplied vehicles.
    pub vehicles: Vec<Vehicle<'a>>,
    /// Customer information.
    pub customer: Option<Customer<'a>>,
    /// Vendor/dealer information.
    pub vendor: Option<Vendor<'a>>,
    /// Lead provider information.
    pub provider: Option<Provider<'a>>,
    /// Unknown prospect-level XML nodes.
    pub extensions: Vec<XmlNode<'a>>,
    /// Prospect attributes, including unknown partner attributes.
    pub attributes: Vec<Attribute<'a>>,
    /// Byte span of the prospect element in the original input.
    pub span: Span,
}

/// ADF `<vehicle>` record.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Vehicle<'a> {
    /// Optional `interest` attribute.
    pub interest: Option<Cow<'a, str>>,
    /// Optional `status` attribute.
    pub status: Option<Cow<'a, str>>,
    /// Vehicle identifiers.
    pub ids: Vec<Id<'a>>,
    /// Model year.
    pub year: Option<TextElement<'a>>,
    /// Vehicle make.
    pub make: Option<TextElement<'a>>,
    /// Vehicle model.
    pub model: Option<TextElement<'a>>,
    /// Vehicle identification number.
    pub vin: Option<TextElement<'a>>,
    /// Stock number.
    pub stock: Option<TextElement<'a>>,
    /// Trim.
    pub trim: Option<TextElement<'a>>,
    /// Door count.
    pub doors: Option<TextElement<'a>>,
    /// Body style.
    pub body_style: Option<TextElement<'a>>,
    /// Transmission.
    pub transmission: Option<TextElement<'a>>,
    /// Odometer value and attributes.
    pub odometer: Option<TextElement<'a>>,
    /// Vehicle condition.
    pub condition: Option<TextElement<'a>>,
    /// Color combinations.
    pub color_combinations: Vec<ColorCombination<'a>>,
    /// Image tags or image URLs.
    pub image_tags: Vec<TextElement<'a>>,
    /// Vehicle prices.
    pub prices: Vec<Price<'a>>,
    /// Price comments.
    pub price_comments: Option<TextElement<'a>>,
    /// Vehicle options.
    pub options: Vec<VehicleOption<'a>>,
    /// Finance information.
    pub finance: Option<Finance<'a>>,
    /// Vehicle comments.
    pub comments: Option<TextElement<'a>>,
    /// Unknown vehicle-level XML nodes.
    pub extensions: Vec<XmlNode<'a>>,
    /// Vehicle attributes, including unknown partner attributes.
    pub attributes: Vec<Attribute<'a>>,
    /// Byte span of the vehicle element in the original input.
    pub span: Span,
}

/// ADF `<colorcombination>` record.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ColorCombination<'a> {
    /// Interior color.
    pub interior_color: Option<TextElement<'a>>,
    /// Exterior color.
    pub exterior_color: Option<TextElement<'a>>,
    /// Color preference.
    pub preference: Option<TextElement<'a>>,
    /// Unknown color-combination XML nodes.
    pub extensions: Vec<XmlNode<'a>>,
    /// Color-combination attributes.
    pub attributes: Vec<Attribute<'a>>,
    /// Byte span in the original input.
    pub span: Span,
}

/// ADF `<option>` record under a vehicle.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VehicleOption<'a> {
    /// Option name.
    pub option_name: Option<TextElement<'a>>,
    /// Manufacturer option code.
    pub manufacturer_code: Option<TextElement<'a>>,
    /// Option stock status.
    pub stock: Option<TextElement<'a>>,
    /// Option weighting.
    pub weighting: Option<TextElement<'a>>,
    /// Option prices.
    pub prices: Vec<Price<'a>>,
    /// Unknown option XML nodes.
    pub extensions: Vec<XmlNode<'a>>,
    /// Option attributes.
    pub attributes: Vec<Attribute<'a>>,
    /// Byte span in the original input.
    pub span: Span,
}

/// ADF `<finance>` record.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Finance<'a> {
    /// Finance method.
    pub method: Option<TextElement<'a>>,
    /// Finance amount elements.
    pub amounts: Vec<TextElement<'a>>,
    /// Finance balance elements.
    pub balances: Vec<TextElement<'a>>,
    /// Unknown finance XML nodes.
    pub extensions: Vec<XmlNode<'a>>,
    /// Finance attributes.
    pub attributes: Vec<Attribute<'a>>,
    /// Byte span in the original input.
    pub span: Span,
}

/// ADF `<customer>` record.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Customer<'a> {
    /// Customer identifiers.
    pub ids: Vec<Id<'a>>,
    /// Customer contacts.
    pub contacts: Vec<Contact<'a>>,
    /// Preferred timeframe.
    pub timeframe: Option<Timeframe<'a>>,
    /// Customer comments.
    pub comments: Option<TextElement<'a>>,
    /// Unknown customer XML nodes.
    pub extensions: Vec<XmlNode<'a>>,
    /// Customer attributes.
    pub attributes: Vec<Attribute<'a>>,
    /// Byte span in the original input.
    pub span: Span,
}

/// ADF `<timeframe>` record.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Timeframe<'a> {
    /// Timeframe description.
    pub description: Option<TextElement<'a>>,
    /// Earliest acceptable date.
    pub earliest_date: Option<TextElement<'a>>,
    /// Latest acceptable date.
    pub latest_date: Option<TextElement<'a>>,
    /// Unknown timeframe XML nodes.
    pub extensions: Vec<XmlNode<'a>>,
    /// Timeframe attributes.
    pub attributes: Vec<Attribute<'a>>,
    /// Byte span in the original input.
    pub span: Span,
}

/// ADF `<vendor>` record.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Vendor<'a> {
    /// Vendor identifiers.
    pub ids: Vec<Id<'a>>,
    /// Vendor name.
    pub vendor_name: Option<TextElement<'a>>,
    /// Vendor URL.
    pub url: Option<TextElement<'a>>,
    /// Vendor contacts.
    pub contacts: Vec<Contact<'a>>,
    /// Unknown vendor XML nodes.
    pub extensions: Vec<XmlNode<'a>>,
    /// Vendor attributes.
    pub attributes: Vec<Attribute<'a>>,
    /// Byte span in the original input.
    pub span: Span,
}

/// ADF `<provider>` record.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Provider<'a> {
    /// Provider identifiers.
    pub ids: Vec<Id<'a>>,
    /// Provider name.
    pub name: Option<Name<'a>>,
    /// Provider service name.
    pub service: Option<TextElement<'a>>,
    /// Provider URL.
    pub url: Option<TextElement<'a>>,
    /// Provider email.
    pub email: Option<TextElement<'a>>,
    /// Provider phone.
    pub phone: Option<TextElement<'a>>,
    /// Provider contacts.
    pub contacts: Vec<Contact<'a>>,
    /// Unknown provider XML nodes.
    pub extensions: Vec<XmlNode<'a>>,
    /// Provider attributes.
    pub attributes: Vec<Attribute<'a>>,
    /// Byte span in the original input.
    pub span: Span,
}

/// ADF `<contact>` record.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Contact<'a> {
    /// Optional `primarycontact` attribute.
    pub primary_contact: Option<Cow<'a, str>>,
    /// Contact names.
    pub names: Vec<Name<'a>>,
    /// Contact email elements.
    pub emails: Vec<TextElement<'a>>,
    /// Contact phone elements.
    pub phones: Vec<TextElement<'a>>,
    /// Contact addresses.
    pub addresses: Vec<Address<'a>>,
    /// Unknown contact XML nodes.
    pub extensions: Vec<XmlNode<'a>>,
    /// Contact attributes.
    pub attributes: Vec<Attribute<'a>>,
    /// Byte span in the original input.
    pub span: Span,
}

/// ADF `<address>` record.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Address<'a> {
    /// Optional `type` attribute.
    pub address_type: Option<Cow<'a, str>>,
    /// Street lines.
    pub streets: Vec<TextElement<'a>>,
    /// Apartment or unit.
    pub apartment: Option<TextElement<'a>>,
    /// City.
    pub city: Option<TextElement<'a>>,
    /// Region or state code.
    pub region_code: Option<TextElement<'a>>,
    /// Postal code.
    pub postal_code: Option<TextElement<'a>>,
    /// Country code.
    pub country: Option<TextElement<'a>>,
    /// Unknown address XML nodes.
    pub extensions: Vec<XmlNode<'a>>,
    /// Address attributes.
    pub attributes: Vec<Attribute<'a>>,
    /// Byte span in the original input.
    pub span: Span,
}

/// ADF `<id>` value plus attributes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Id<'a> {
    /// Optional `sequence` attribute.
    pub sequence: Option<Cow<'a, str>>,
    /// Optional `source` attribute.
    pub source: Option<Cow<'a, str>>,
    /// Mixed text parts for the identifier value.
    pub parts: Vec<TextPart<'a>>,
    /// Identifier attributes, including unknown partner attributes.
    pub attributes: Vec<Attribute<'a>>,
    /// Byte span in the original input.
    pub span: Span,
}

/// ADF `<price>` value plus attributes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Price<'a> {
    /// Optional `type` attribute.
    pub price_type: Option<Cow<'a, str>>,
    /// Optional `currency` attribute.
    pub currency: Option<Cow<'a, str>>,
    /// Optional `delta` attribute.
    pub delta: Option<Cow<'a, str>>,
    /// Optional `relativeto` attribute.
    pub relative_to: Option<Cow<'a, str>>,
    /// Optional `source` attribute.
    pub source: Option<Cow<'a, str>>,
    /// Mixed text parts for the price value.
    pub parts: Vec<TextPart<'a>>,
    /// Price attributes, including unknown partner attributes.
    pub attributes: Vec<Attribute<'a>>,
    /// Byte span in the original input.
    pub span: Span,
}

/// ADF `<name>` value plus attributes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Name<'a> {
    /// Optional `part` attribute.
    pub part: Option<Cow<'a, str>>,
    /// Optional `type` attribute.
    pub name_type: Option<Cow<'a, str>>,
    /// Mixed text parts for the name value.
    pub parts: Vec<TextPart<'a>>,
    /// Name attributes, including unknown partner attributes.
    pub attributes: Vec<Attribute<'a>>,
    /// Byte span in the original input.
    pub span: Span,
}

/// Text-like ADF element that can preserve mixed XML text parts.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TextElement<'a> {
    /// Text, CDATA, entity references, and embedded XML nodes in source order.
    pub parts: Vec<TextPart<'a>>,
    /// Attributes on the text element.
    pub attributes: Vec<Attribute<'a>>,
    /// Byte span of the element in the original input.
    pub span: Span,
}

impl<'a> TextElement<'a> {
    /// Construct a plain text element with attributes and no source span.
    pub fn new(value: Cow<'a, str>, attributes: Vec<Attribute<'a>>) -> Self {
        Self {
            parts: vec![TextPart::Text(value)],
            attributes,
            span: Span::default(),
        }
    }

    /// Construct a text element from already split parts and attributes.
    pub fn from_parts(parts: Vec<TextPart<'a>>, attributes: Vec<Attribute<'a>>) -> Self {
        Self {
            parts,
            attributes,
            span: Span::default(),
        }
    }
}

macro_rules! impl_text_value {
    ($t:ident) => {
        impl<'a> $t<'a> {
            /// Return the joined textual value.
            ///
            /// Standard entity references are resolved; unknown entity
            /// references are returned as literal `&name;` text. Embedded XML
            /// nodes do not contribute to this flattened value.
            pub fn value(&self) -> Cow<'a, str> {
                text_parts_value(&self.parts)
            }

            /// Replace all existing text parts with one flat text value.
            pub fn set_value(&mut self, value: impl Into<Cow<'a, str>>) {
                self.parts = vec![TextPart::Text(value.into())];
            }
        }
    };
}

impl_text_value!(Id);
impl_text_value!(Price);
impl_text_value!(Name);
impl_text_value!(TextElement);

/// One part of a text-like element value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextPart<'a> {
    /// Plain text.
    Text(Cow<'a, str>),
    /// CDATA content without the wrapper.
    CData(Cow<'a, str>),
    /// Unresolved named entity reference, stored without `&` and `;`.
    EntityRef(Cow<'a, str>),
    /// Embedded XML node inside text-like content.
    Node(XmlNode<'a>),
}

fn text_parts_value<'a>(parts: &[TextPart<'a>]) -> Cow<'a, str> {
    match parts {
        [] => Cow::Borrowed(""),
        [TextPart::Text(text) | TextPart::CData(text)] => text.clone(),
        [TextPart::EntityRef(name)] => match resolve_standard_entity(name) {
            Some(resolved) => Cow::Borrowed(resolved),
            None => Cow::Owned(format!("&{name};")),
        },
        [TextPart::Node(_)] => Cow::Borrowed(""),
        _ => {
            let mut joined = String::new();
            for part in parts {
                match part {
                    TextPart::Text(text) | TextPart::CData(text) => joined.push_str(text),
                    TextPart::EntityRef(name) => match resolve_standard_entity(name) {
                        Some(resolved) => joined.push_str(resolved),
                        None => {
                            joined.push('&');
                            joined.push_str(name);
                            joined.push(';');
                        }
                    },
                    TextPart::Node(_) => {}
                }
            }
            Cow::Owned(joined)
        }
    }
}

pub(crate) fn resolve_standard_entity(name: &str) -> Option<&'static str> {
    match name {
        "amp" => Some("&"),
        "lt" => Some("<"),
        "gt" => Some(">"),
        "quot" => Some("\""),
        "apos" => Some("'"),
        _ => None,
    }
}
