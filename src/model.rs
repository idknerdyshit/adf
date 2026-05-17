use crate::{Attribute, XmlNode};
use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Adf<'a> {
    pub prospects: Vec<Prospect<'a>>,
    pub extensions: Vec<XmlNode<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Prospect<'a> {
    pub status: Option<Cow<'a, str>>,
    pub ids: Vec<Id<'a>>,
    pub request_date: Option<TextElement<'a>>,
    pub vehicles: Vec<Vehicle<'a>>,
    pub customer: Option<Customer<'a>>,
    pub vendor: Option<Vendor<'a>>,
    pub provider: Option<Provider<'a>>,
    pub extensions: Vec<XmlNode<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Vehicle<'a> {
    pub interest: Option<Cow<'a, str>>,
    pub status: Option<Cow<'a, str>>,
    pub ids: Vec<Id<'a>>,
    pub year: Option<TextElement<'a>>,
    pub make: Option<TextElement<'a>>,
    pub model: Option<TextElement<'a>>,
    pub vin: Option<TextElement<'a>>,
    pub stock: Option<TextElement<'a>>,
    pub trim: Option<TextElement<'a>>,
    pub doors: Option<TextElement<'a>>,
    pub body_style: Option<TextElement<'a>>,
    pub transmission: Option<TextElement<'a>>,
    pub odometer: Option<TextElement<'a>>,
    pub condition: Option<TextElement<'a>>,
    pub color_combinations: Vec<ColorCombination<'a>>,
    pub image_tags: Vec<TextElement<'a>>,
    pub prices: Vec<Price<'a>>,
    pub price_comments: Option<TextElement<'a>>,
    pub options: Vec<VehicleOption<'a>>,
    pub finance: Option<Finance<'a>>,
    pub comments: Option<TextElement<'a>>,
    pub extensions: Vec<XmlNode<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ColorCombination<'a> {
    pub interior_color: Option<TextElement<'a>>,
    pub exterior_color: Option<TextElement<'a>>,
    pub preference: Option<TextElement<'a>>,
    pub extensions: Vec<XmlNode<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VehicleOption<'a> {
    pub option_name: Option<TextElement<'a>>,
    pub manufacturer_code: Option<TextElement<'a>>,
    pub stock: Option<TextElement<'a>>,
    pub weighting: Option<TextElement<'a>>,
    pub prices: Vec<Price<'a>>,
    pub extensions: Vec<XmlNode<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Finance<'a> {
    pub method: Option<TextElement<'a>>,
    pub amounts: Vec<TextElement<'a>>,
    pub balances: Vec<TextElement<'a>>,
    pub extensions: Vec<XmlNode<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Customer<'a> {
    pub ids: Vec<Id<'a>>,
    pub contacts: Vec<Contact<'a>>,
    pub timeframe: Option<Timeframe<'a>>,
    pub comments: Option<TextElement<'a>>,
    pub extensions: Vec<XmlNode<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Timeframe<'a> {
    pub description: Option<TextElement<'a>>,
    pub earliest_date: Option<TextElement<'a>>,
    pub latest_date: Option<TextElement<'a>>,
    pub extensions: Vec<XmlNode<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Vendor<'a> {
    pub ids: Vec<Id<'a>>,
    pub vendor_name: Option<TextElement<'a>>,
    pub url: Option<TextElement<'a>>,
    pub contacts: Vec<Contact<'a>>,
    pub extensions: Vec<XmlNode<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Provider<'a> {
    pub ids: Vec<Id<'a>>,
    pub name: Option<Name<'a>>,
    pub service: Option<TextElement<'a>>,
    pub url: Option<TextElement<'a>>,
    pub email: Option<TextElement<'a>>,
    pub phone: Option<TextElement<'a>>,
    pub contacts: Vec<Contact<'a>>,
    pub extensions: Vec<XmlNode<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Contact<'a> {
    pub primary_contact: Option<Cow<'a, str>>,
    pub names: Vec<Name<'a>>,
    pub emails: Vec<TextElement<'a>>,
    pub phones: Vec<TextElement<'a>>,
    pub addresses: Vec<Address<'a>>,
    pub extensions: Vec<XmlNode<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Address<'a> {
    pub address_type: Option<Cow<'a, str>>,
    pub streets: Vec<TextElement<'a>>,
    pub apartment: Option<TextElement<'a>>,
    pub city: Option<TextElement<'a>>,
    pub region_code: Option<TextElement<'a>>,
    pub postal_code: Option<TextElement<'a>>,
    pub country: Option<TextElement<'a>>,
    pub extensions: Vec<XmlNode<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Id<'a> {
    pub sequence: Option<Cow<'a, str>>,
    pub source: Option<Cow<'a, str>>,
    pub value: Cow<'a, str>,
    pub attributes: Vec<Attribute<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Price<'a> {
    pub price_type: Option<Cow<'a, str>>,
    pub currency: Option<Cow<'a, str>>,
    pub delta: Option<Cow<'a, str>>,
    pub relative_to: Option<Cow<'a, str>>,
    pub source: Option<Cow<'a, str>>,
    pub value: Cow<'a, str>,
    pub attributes: Vec<Attribute<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Name<'a> {
    pub part: Option<Cow<'a, str>>,
    pub name_type: Option<Cow<'a, str>>,
    pub value: Cow<'a, str>,
    pub attributes: Vec<Attribute<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TextElement<'a> {
    pub value: Cow<'a, str>,
    pub attributes: Vec<Attribute<'a>>,
}

impl<'a> TextElement<'a> {
    pub fn new(value: Cow<'a, str>, attributes: Vec<Attribute<'a>>) -> Self {
        Self { value, attributes }
    }
}
