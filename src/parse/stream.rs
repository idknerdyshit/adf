use super::{EventConsumer, attr, is_document_misc, is_xml_whitespace};
use crate::document::{Span, XmlElement, XmlNode};
use crate::error::{Error, Result};
use crate::model::{
    Address, Adf, ColorCombination, Contact, Customer, Finance, Id, Name, Price, Prospect,
    Provider, TextElement, TextPart, Timeframe, Vehicle, VehicleOption, Vendor,
};
use std::ops::Range;

pub(super) struct ParsedTypedDocument<'a> {
    pub(super) adf: Adf<'a>,
    pub(super) prospect_spans: Vec<Range<usize>>,
    pub(super) prolog: Vec<XmlNode<'a>>,
    pub(super) epilog: Vec<XmlNode<'a>>,
}

#[derive(Default)]
pub(super) struct TypedAdfBuilder<'a> {
    stack: Vec<Frame<'a>>,
    adf: Option<Adf<'a>>,
    root_complete: bool,
    unexpected_root: Option<(String, u64)>,
    prospect_spans: Vec<Range<usize>>,
    prolog: Vec<XmlNode<'a>>,
    epilog: Vec<XmlNode<'a>>,
}

impl<'a> EventConsumer<'a> for TypedAdfBuilder<'a> {
    type Output = ParsedTypedDocument<'a>;

    fn start(&mut self, element: XmlElement<'a>, _position: u64) -> Result<()> {
        self.open(element)
    }

    fn empty(&mut self, element: XmlElement<'a>, _position: u64) -> Result<()> {
        let tag = Tag::from_name(element.name.as_ref());
        if let Some(parent) = self.stack.last_mut() {
            let frame_type = parent.child_type(tag);
            if let Some(span) = parent.attach(Frame::new(element, frame_type, tag)) {
                self.prospect_spans.push(span);
            }
            return Ok(());
        }

        if self.root_complete {
            return Err(Error::MultipleRoots);
        }
        if tag != Tag::Adf {
            self.unexpected_root = Some((element.name.to_string(), element.span.start as u64));
            self.root_complete = true;
            return Ok(());
        }
        let frame = Frame::new(element, FrameType::Adf, tag);
        let FrameKind::Adf(adf) = frame.kind else {
            unreachable!("the document root frame is always ADF");
        };
        self.adf = Some(adf);
        self.root_complete = true;
        Ok(())
    }

    fn end(&mut self, span_end: usize, position: u64) -> Result<()> {
        self.close(span_end, position)
    }

    fn node(&mut self, node: XmlNode<'a>, position: u64) -> Result<()> {
        if let Some(frame) = self.stack.last_mut() {
            frame.push_node(node);
            return Ok(());
        }

        let has_root = self.root_complete;
        if !is_document_misc(&node, has_root) {
            return Err(Error::ContentOutsideRoot { position });
        }
        if has_root {
            self.epilog.push(node);
        } else {
            self.prolog.push(node);
        }
        Ok(())
    }

    fn finish(mut self, position: u64) -> Result<Self::Output> {
        if let Some(unclosed) = self.stack.pop() {
            return Err(Error::UnexpectedEnd {
                name: unclosed.diagnostic_name().to_owned(),
                position,
            });
        }
        if let Some((found, position)) = self.unexpected_root {
            return Err(Error::UnexpectedRoot { found, position });
        }
        Ok(ParsedTypedDocument {
            adf: self.adf.ok_or(Error::MissingRoot)?,
            prospect_spans: self.prospect_spans,
            prolog: self.prolog,
            epilog: self.epilog,
        })
    }
}

impl<'a> TypedAdfBuilder<'a> {
    fn open(&mut self, element: XmlElement<'a>) -> Result<()> {
        let tag = Tag::from_name(element.name.as_ref());
        let frame_type = if let Some(parent) = self.stack.last() {
            parent.child_type(tag)
        } else {
            if self.root_complete {
                return Err(Error::MultipleRoots);
            }
            if tag != Tag::Adf {
                self.unexpected_root = Some((element.name.to_string(), element.span.start as u64));
                FrameType::Raw
            } else {
                FrameType::Adf
            }
        };
        self.stack.push(Frame::new(element, frame_type, tag));
        Ok(())
    }

    fn close(&mut self, span_end: usize, position: u64) -> Result<()> {
        let mut frame = self.stack.pop().ok_or_else(|| Error::UnexpectedEnd {
            name: String::new(),
            position,
        })?;
        frame.set_span_end(span_end);

        if let Some(parent) = self.stack.last_mut() {
            if let Some(span) = parent.attach(frame) {
                self.prospect_spans.push(span);
            }
            return Ok(());
        }

        match frame.kind {
            FrameKind::Adf(adf) => self.adf = Some(adf),
            FrameKind::Raw(_) if self.unexpected_root.is_some() => {}
            _ => unreachable!("the document root frame is always ADF or deferred invalid XML"),
        }
        self.root_complete = true;
        Ok(())
    }
}

struct Frame<'a> {
    tag: Tag,
    kind: FrameKind<'a>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tag {
    Adf,
    Prospect,
    Id,
    RequestDate,
    Vehicle,
    Customer,
    Vendor,
    Provider,
    Year,
    Make,
    Model,
    Vin,
    Stock,
    Trim,
    Doors,
    BodyStyle,
    Transmission,
    Odometer,
    Condition,
    ColorCombination,
    Image,
    Price,
    PriceComments,
    Option,
    Finance,
    Comments,
    InteriorColor,
    ExteriorColor,
    Preference,
    OptionName,
    ManufacturerCode,
    Weighting,
    Method,
    Amount,
    Balance,
    Contact,
    Timeframe,
    Description,
    EarliestDate,
    LatestDate,
    VendorName,
    Url,
    Name,
    Service,
    Email,
    Phone,
    Address,
    Street,
    Apartment,
    City,
    RegionCode,
    PostalCode,
    Country,
    Other,
}

impl Tag {
    fn from_name(name: &str) -> Self {
        match name {
            "adf" => Self::Adf,
            "prospect" => Self::Prospect,
            "id" => Self::Id,
            "requestdate" => Self::RequestDate,
            "vehicle" => Self::Vehicle,
            "customer" => Self::Customer,
            "vendor" => Self::Vendor,
            "provider" => Self::Provider,
            "year" => Self::Year,
            "make" => Self::Make,
            "model" => Self::Model,
            "vin" => Self::Vin,
            "stock" => Self::Stock,
            "trim" => Self::Trim,
            "doors" => Self::Doors,
            "bodystyle" => Self::BodyStyle,
            "transmission" => Self::Transmission,
            "odometer" => Self::Odometer,
            "condition" => Self::Condition,
            "colorcombination" => Self::ColorCombination,
            "imagetag" => Self::Image,
            "price" => Self::Price,
            "pricecomments" => Self::PriceComments,
            "option" => Self::Option,
            "finance" => Self::Finance,
            "comments" => Self::Comments,
            "interiorcolor" => Self::InteriorColor,
            "exteriorcolor" => Self::ExteriorColor,
            "preference" => Self::Preference,
            "optionname" => Self::OptionName,
            "manufacturercode" => Self::ManufacturerCode,
            "weighting" => Self::Weighting,
            "method" => Self::Method,
            "amount" => Self::Amount,
            "balance" => Self::Balance,
            "contact" => Self::Contact,
            "timeframe" => Self::Timeframe,
            "description" => Self::Description,
            "earliestdate" => Self::EarliestDate,
            "latestdate" => Self::LatestDate,
            "vendorname" => Self::VendorName,
            "url" => Self::Url,
            "name" => Self::Name,
            "service" => Self::Service,
            "email" => Self::Email,
            "phone" => Self::Phone,
            "address" => Self::Address,
            "street" => Self::Street,
            "apartment" => Self::Apartment,
            "city" => Self::City,
            "regioncode" => Self::RegionCode,
            "postalcode" => Self::PostalCode,
            "country" => Self::Country,
            _ => Self::Other,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Adf => "adf",
            Self::Prospect => "prospect",
            Self::Id => "id",
            Self::RequestDate => "requestdate",
            Self::Vehicle => "vehicle",
            Self::Customer => "customer",
            Self::Vendor => "vendor",
            Self::Provider => "provider",
            Self::Year => "year",
            Self::Make => "make",
            Self::Model => "model",
            Self::Vin => "vin",
            Self::Stock => "stock",
            Self::Trim => "trim",
            Self::Doors => "doors",
            Self::BodyStyle => "bodystyle",
            Self::Transmission => "transmission",
            Self::Odometer => "odometer",
            Self::Condition => "condition",
            Self::ColorCombination => "colorcombination",
            Self::Image => "imagetag",
            Self::Price => "price",
            Self::PriceComments => "pricecomments",
            Self::Option => "option",
            Self::Finance => "finance",
            Self::Comments => "comments",
            Self::InteriorColor => "interiorcolor",
            Self::ExteriorColor => "exteriorcolor",
            Self::Preference => "preference",
            Self::OptionName => "optionname",
            Self::ManufacturerCode => "manufacturercode",
            Self::Weighting => "weighting",
            Self::Method => "method",
            Self::Amount => "amount",
            Self::Balance => "balance",
            Self::Contact => "contact",
            Self::Timeframe => "timeframe",
            Self::Description => "description",
            Self::EarliestDate => "earliestdate",
            Self::LatestDate => "latestdate",
            Self::VendorName => "vendorname",
            Self::Url => "url",
            Self::Name => "name",
            Self::Service => "service",
            Self::Email => "email",
            Self::Phone => "phone",
            Self::Address => "address",
            Self::Street => "street",
            Self::Apartment => "apartment",
            Self::City => "city",
            Self::RegionCode => "regioncode",
            Self::PostalCode => "postalcode",
            Self::Country => "country",
            Self::Other => unreachable!("raw frames retain their source name"),
        }
    }
}

#[derive(Clone, Copy)]
enum FrameType {
    Adf,
    Prospect,
    Vehicle,
    ColorCombination,
    VehicleOption,
    Finance,
    Customer,
    Timeframe,
    Vendor,
    Provider,
    Contact,
    Address,
    Id,
    Price,
    Name,
    Text,
    Raw,
}

enum FrameKind<'a> {
    Adf(Adf<'a>),
    Prospect(Box<Prospect<'a>>),
    Vehicle(Box<Vehicle<'a>>),
    ColorCombination(Box<ColorCombination<'a>>),
    VehicleOption(Box<VehicleOption<'a>>),
    Finance(Box<Finance<'a>>),
    Customer(Box<Customer<'a>>),
    Timeframe(Box<Timeframe<'a>>),
    Vendor(Box<Vendor<'a>>),
    Provider(Box<Provider<'a>>),
    Contact(Box<Contact<'a>>),
    Address(Box<Address<'a>>),
    Id(Id<'a>),
    Price(Price<'a>),
    Name(Name<'a>),
    Text(TextElement<'a>),
    Raw(XmlElement<'a>),
}

impl<'a> Frame<'a> {
    fn new(element: XmlElement<'a>, frame_type: FrameType, tag: Tag) -> Self {
        let XmlElement {
            name,
            attributes,
            children,
            span,
        } = element;
        debug_assert!(children.is_empty());
        let kind = match frame_type {
            FrameType::Adf => FrameKind::Adf(Adf {
                attributes,
                span,
                ..Default::default()
            }),
            FrameType::Prospect => FrameKind::Prospect(Box::new(Prospect {
                status: attr(&attributes, "status"),
                attributes,
                span,
                ..Default::default()
            })),
            FrameType::Vehicle => FrameKind::Vehicle(Box::new(Vehicle {
                interest: attr(&attributes, "interest"),
                status: attr(&attributes, "status"),
                attributes,
                span,
                ..Default::default()
            })),
            FrameType::ColorCombination => {
                FrameKind::ColorCombination(Box::new(ColorCombination {
                    attributes,
                    span,
                    ..Default::default()
                }))
            }
            FrameType::VehicleOption => FrameKind::VehicleOption(Box::new(VehicleOption {
                attributes,
                span,
                ..Default::default()
            })),
            FrameType::Finance => FrameKind::Finance(Box::new(Finance {
                attributes,
                span,
                ..Default::default()
            })),
            FrameType::Customer => FrameKind::Customer(Box::new(Customer {
                attributes,
                span,
                ..Default::default()
            })),
            FrameType::Timeframe => FrameKind::Timeframe(Box::new(Timeframe {
                attributes,
                span,
                ..Default::default()
            })),
            FrameType::Vendor => FrameKind::Vendor(Box::new(Vendor {
                attributes,
                span,
                ..Default::default()
            })),
            FrameType::Provider => FrameKind::Provider(Box::new(Provider {
                attributes,
                span,
                ..Default::default()
            })),
            FrameType::Contact => FrameKind::Contact(Box::new(Contact {
                primary_contact: attr(&attributes, "primarycontact"),
                attributes,
                span,
                ..Default::default()
            })),
            FrameType::Address => FrameKind::Address(Box::new(Address {
                address_type: attr(&attributes, "type"),
                attributes,
                span,
                ..Default::default()
            })),
            FrameType::Id => FrameKind::Id(Id {
                sequence: attr(&attributes, "sequence"),
                source: attr(&attributes, "source"),
                attributes,
                span,
                ..Default::default()
            }),
            FrameType::Price => FrameKind::Price(Price {
                price_type: attr(&attributes, "type"),
                currency: attr(&attributes, "currency"),
                delta: attr(&attributes, "delta"),
                relative_to: attr(&attributes, "relativeto"),
                source: attr(&attributes, "source"),
                attributes,
                span,
                ..Default::default()
            }),
            FrameType::Name => FrameKind::Name(Name {
                part: attr(&attributes, "part"),
                name_type: attr(&attributes, "type"),
                attributes,
                span,
                ..Default::default()
            }),
            FrameType::Text => FrameKind::Text(TextElement {
                attributes,
                span,
                ..Default::default()
            }),
            FrameType::Raw => FrameKind::Raw(XmlElement {
                name: name.clone(),
                attributes,
                children: Vec::new(),
                span,
            }),
        };
        Self { tag, kind }
    }

    fn diagnostic_name(&self) -> &str {
        match &self.kind {
            FrameKind::Raw(value) => value.name.as_ref(),
            _ => self.tag.as_str(),
        }
    }

    fn child_type(&self, tag: Tag) -> FrameType {
        match &self.kind {
            FrameKind::Adf(_) if tag == Tag::Prospect => FrameType::Prospect,
            FrameKind::Prospect(value) => match tag {
                Tag::Id => FrameType::Id,
                Tag::RequestDate if value.request_date.is_none() => FrameType::Text,
                Tag::Vehicle => FrameType::Vehicle,
                Tag::Customer if value.customer.is_none() => FrameType::Customer,
                Tag::Vendor if value.vendor.is_none() => FrameType::Vendor,
                Tag::Provider if value.provider.is_none() => FrameType::Provider,
                _ => FrameType::Raw,
            },
            FrameKind::Vehicle(value) => match tag {
                Tag::Id => FrameType::Id,
                Tag::Year if value.year.is_none() => FrameType::Text,
                Tag::Make if value.make.is_none() => FrameType::Text,
                Tag::Model if value.model.is_none() => FrameType::Text,
                Tag::Vin if value.vin.is_none() => FrameType::Text,
                Tag::Stock if value.stock.is_none() => FrameType::Text,
                Tag::Trim if value.trim.is_none() => FrameType::Text,
                Tag::Doors if value.doors.is_none() => FrameType::Text,
                Tag::BodyStyle if value.body_style.is_none() => FrameType::Text,
                Tag::Transmission if value.transmission.is_none() => FrameType::Text,
                Tag::Odometer if value.odometer.is_none() => FrameType::Text,
                Tag::Condition if value.condition.is_none() => FrameType::Text,
                Tag::ColorCombination => FrameType::ColorCombination,
                Tag::Image => FrameType::Text,
                Tag::Price => FrameType::Price,
                Tag::PriceComments if value.price_comments.is_none() => FrameType::Text,
                Tag::Option => FrameType::VehicleOption,
                Tag::Finance if value.finance.is_none() => FrameType::Finance,
                Tag::Comments if value.comments.is_none() => FrameType::Text,
                _ => FrameType::Raw,
            },
            FrameKind::ColorCombination(value) => match tag {
                Tag::InteriorColor if value.interior_color.is_none() => FrameType::Text,
                Tag::ExteriorColor if value.exterior_color.is_none() => FrameType::Text,
                Tag::Preference if value.preference.is_none() => FrameType::Text,
                _ => FrameType::Raw,
            },
            FrameKind::VehicleOption(value) => match tag {
                Tag::OptionName if value.option_name.is_none() => FrameType::Text,
                Tag::ManufacturerCode if value.manufacturer_code.is_none() => FrameType::Text,
                Tag::Stock if value.stock.is_none() => FrameType::Text,
                Tag::Weighting if value.weighting.is_none() => FrameType::Text,
                Tag::Price => FrameType::Price,
                _ => FrameType::Raw,
            },
            FrameKind::Finance(value) => match tag {
                Tag::Method if value.method.is_none() => FrameType::Text,
                Tag::Amount | Tag::Balance => FrameType::Text,
                _ => FrameType::Raw,
            },
            FrameKind::Customer(value) => match tag {
                Tag::Id => FrameType::Id,
                Tag::Contact => FrameType::Contact,
                Tag::Timeframe if value.timeframe.is_none() => FrameType::Timeframe,
                Tag::Comments if value.comments.is_none() => FrameType::Text,
                _ => FrameType::Raw,
            },
            FrameKind::Timeframe(value) => match tag {
                Tag::Description if value.description.is_none() => FrameType::Text,
                Tag::EarliestDate if value.earliest_date.is_none() => FrameType::Text,
                Tag::LatestDate if value.latest_date.is_none() => FrameType::Text,
                _ => FrameType::Raw,
            },
            FrameKind::Vendor(value) => match tag {
                Tag::Id => FrameType::Id,
                Tag::VendorName if value.vendor_name.is_none() => FrameType::Text,
                Tag::Url if value.url.is_none() => FrameType::Text,
                Tag::Contact => FrameType::Contact,
                _ => FrameType::Raw,
            },
            FrameKind::Provider(value) => match tag {
                Tag::Id => FrameType::Id,
                Tag::Name if value.name.is_none() => FrameType::Name,
                Tag::Service if value.service.is_none() => FrameType::Text,
                Tag::Url if value.url.is_none() => FrameType::Text,
                Tag::Email if value.email.is_none() => FrameType::Text,
                Tag::Phone if value.phone.is_none() => FrameType::Text,
                Tag::Contact => FrameType::Contact,
                _ => FrameType::Raw,
            },
            FrameKind::Contact(_) => match tag {
                Tag::Name => FrameType::Name,
                Tag::Email | Tag::Phone => FrameType::Text,
                Tag::Address => FrameType::Address,
                _ => FrameType::Raw,
            },
            FrameKind::Address(value) => match tag {
                Tag::Street => FrameType::Text,
                Tag::Apartment if value.apartment.is_none() => FrameType::Text,
                Tag::City if value.city.is_none() => FrameType::Text,
                Tag::RegionCode if value.region_code.is_none() => FrameType::Text,
                Tag::PostalCode if value.postal_code.is_none() => FrameType::Text,
                Tag::Country if value.country.is_none() => FrameType::Text,
                _ => FrameType::Raw,
            },
            FrameKind::Raw(_)
            | FrameKind::Id(_)
            | FrameKind::Price(_)
            | FrameKind::Name(_)
            | FrameKind::Text(_)
            | FrameKind::Adf(_) => FrameType::Raw,
        }
    }

    fn set_span_end(&mut self, span_end: usize) {
        self.span_mut().end = span_end;
    }

    fn span_mut(&mut self) -> &mut Span {
        match &mut self.kind {
            FrameKind::Adf(value) => &mut value.span,
            FrameKind::Prospect(value) => &mut value.span,
            FrameKind::Vehicle(value) => &mut value.span,
            FrameKind::ColorCombination(value) => &mut value.span,
            FrameKind::VehicleOption(value) => &mut value.span,
            FrameKind::Finance(value) => &mut value.span,
            FrameKind::Customer(value) => &mut value.span,
            FrameKind::Timeframe(value) => &mut value.span,
            FrameKind::Vendor(value) => &mut value.span,
            FrameKind::Provider(value) => &mut value.span,
            FrameKind::Contact(value) => &mut value.span,
            FrameKind::Address(value) => &mut value.span,
            FrameKind::Id(value) => &mut value.span,
            FrameKind::Price(value) => &mut value.span,
            FrameKind::Name(value) => &mut value.span,
            FrameKind::Text(value) => &mut value.span,
            FrameKind::Raw(value) => &mut value.span,
        }
    }

    fn push_node(&mut self, node: XmlNode<'a>) {
        match &mut self.kind {
            FrameKind::Raw(value) => value.children.push(node),
            FrameKind::Id(value) => value.parts.push(text_part(node)),
            FrameKind::Price(value) => value.parts.push(text_part(node)),
            FrameKind::Name(value) => value.parts.push(text_part(node)),
            FrameKind::Text(value) => value.parts.push(text_part(node)),
            _ if matches!(&node, XmlNode::Text(text) if text.bytes().all(is_xml_whitespace)) => {}
            _ => self.extensions_mut().push(node),
        }
    }

    fn extensions_mut(&mut self) -> &mut Vec<XmlNode<'a>> {
        match &mut self.kind {
            FrameKind::Adf(value) => &mut value.extensions,
            FrameKind::Prospect(value) => &mut value.extensions,
            FrameKind::Vehicle(value) => &mut value.extensions,
            FrameKind::ColorCombination(value) => &mut value.extensions,
            FrameKind::VehicleOption(value) => &mut value.extensions,
            FrameKind::Finance(value) => &mut value.extensions,
            FrameKind::Customer(value) => &mut value.extensions,
            FrameKind::Timeframe(value) => &mut value.extensions,
            FrameKind::Vendor(value) => &mut value.extensions,
            FrameKind::Provider(value) => &mut value.extensions,
            FrameKind::Contact(value) => &mut value.extensions,
            FrameKind::Address(value) => &mut value.extensions,
            FrameKind::Raw(_)
            | FrameKind::Id(_)
            | FrameKind::Price(_)
            | FrameKind::Name(_)
            | FrameKind::Text(_) => unreachable!("leaf frames do not have extension vectors"),
        }
    }

    fn attach(&mut self, child: Frame<'a>) -> Option<Range<usize>> {
        let Frame { tag, kind } = child;
        if let FrameKind::Raw(value) = kind {
            self.push_node(XmlNode::Element(value));
            return None;
        }

        match (&mut self.kind, kind) {
            (FrameKind::Adf(parent), FrameKind::Prospect(value)) => {
                let span = value.span.start..value.span.end;
                parent.prospects.push(*value);
                return Some(span);
            }
            (FrameKind::Prospect(parent), FrameKind::Id(value)) => parent.ids.push(value),
            (FrameKind::Prospect(parent), FrameKind::Text(value)) if tag == Tag::RequestDate => {
                parent.request_date = Some(value);
            }
            (FrameKind::Prospect(parent), FrameKind::Vehicle(value)) => {
                parent.vehicles.push(*value);
            }
            (FrameKind::Prospect(parent), FrameKind::Customer(value)) => {
                parent.customer = Some(*value);
            }
            (FrameKind::Prospect(parent), FrameKind::Vendor(value)) => {
                parent.vendor = Some(*value);
            }
            (FrameKind::Prospect(parent), FrameKind::Provider(value)) => {
                parent.provider = Some(*value);
            }
            (FrameKind::Vehicle(parent), FrameKind::Id(value)) => parent.ids.push(value),
            (FrameKind::Vehicle(parent), FrameKind::Text(value)) => match tag {
                Tag::Year => parent.year = Some(value),
                Tag::Make => parent.make = Some(value),
                Tag::Model => parent.model = Some(value),
                Tag::Vin => parent.vin = Some(value),
                Tag::Stock => parent.stock = Some(value),
                Tag::Trim => parent.trim = Some(value),
                Tag::Doors => parent.doors = Some(value),
                Tag::BodyStyle => parent.body_style = Some(value),
                Tag::Transmission => parent.transmission = Some(value),
                Tag::Odometer => parent.odometer = Some(value),
                Tag::Condition => parent.condition = Some(value),
                Tag::Image => parent.image_tags.push(value),
                Tag::PriceComments => parent.price_comments = Some(value),
                Tag::Comments => parent.comments = Some(value),
                _ => unreachable!("unexpected vehicle text child"),
            },
            (FrameKind::Vehicle(parent), FrameKind::ColorCombination(value)) => {
                parent.color_combinations.push(*value);
            }
            (FrameKind::Vehicle(parent), FrameKind::Price(value)) => parent.prices.push(value),
            (FrameKind::Vehicle(parent), FrameKind::VehicleOption(value)) => {
                parent.options.push(*value);
            }
            (FrameKind::Vehicle(parent), FrameKind::Finance(value)) => {
                parent.finance = Some(*value);
            }
            (FrameKind::ColorCombination(parent), FrameKind::Text(value)) => match tag {
                Tag::InteriorColor => parent.interior_color = Some(value),
                Tag::ExteriorColor => parent.exterior_color = Some(value),
                Tag::Preference => parent.preference = Some(value),
                _ => unreachable!("unexpected color text child"),
            },
            (FrameKind::VehicleOption(parent), FrameKind::Text(value)) => match tag {
                Tag::OptionName => parent.option_name = Some(value),
                Tag::ManufacturerCode => parent.manufacturer_code = Some(value),
                Tag::Stock => parent.stock = Some(value),
                Tag::Weighting => parent.weighting = Some(value),
                _ => unreachable!("unexpected option text child"),
            },
            (FrameKind::VehicleOption(parent), FrameKind::Price(value)) => {
                parent.prices.push(value);
            }
            (FrameKind::Finance(parent), FrameKind::Text(value)) => match tag {
                Tag::Method => parent.method = Some(value),
                Tag::Amount => parent.amounts.push(value),
                Tag::Balance => parent.balances.push(value),
                _ => unreachable!("unexpected finance text child"),
            },
            (FrameKind::Customer(parent), FrameKind::Id(value)) => parent.ids.push(value),
            (FrameKind::Customer(parent), FrameKind::Contact(value)) => {
                parent.contacts.push(*value);
            }
            (FrameKind::Customer(parent), FrameKind::Timeframe(value)) => {
                parent.timeframe = Some(*value);
            }
            (FrameKind::Customer(parent), FrameKind::Text(value)) if tag == Tag::Comments => {
                parent.comments = Some(value);
            }
            (FrameKind::Timeframe(parent), FrameKind::Text(value)) => match tag {
                Tag::Description => parent.description = Some(value),
                Tag::EarliestDate => parent.earliest_date = Some(value),
                Tag::LatestDate => parent.latest_date = Some(value),
                _ => unreachable!("unexpected timeframe text child"),
            },
            (FrameKind::Vendor(parent), FrameKind::Id(value)) => parent.ids.push(value),
            (FrameKind::Vendor(parent), FrameKind::Text(value)) => match tag {
                Tag::VendorName => parent.vendor_name = Some(value),
                Tag::Url => parent.url = Some(value),
                _ => unreachable!("unexpected vendor text child"),
            },
            (FrameKind::Vendor(parent), FrameKind::Contact(value)) => {
                parent.contacts.push(*value);
            }
            (FrameKind::Provider(parent), FrameKind::Id(value)) => parent.ids.push(value),
            (FrameKind::Provider(parent), FrameKind::Name(value)) if tag == Tag::Name => {
                parent.name = Some(value);
            }
            (FrameKind::Provider(parent), FrameKind::Text(value)) => match tag {
                Tag::Service => parent.service = Some(value),
                Tag::Url => parent.url = Some(value),
                Tag::Email => parent.email = Some(value),
                Tag::Phone => parent.phone = Some(value),
                _ => unreachable!("unexpected provider text child"),
            },
            (FrameKind::Provider(parent), FrameKind::Contact(value)) => {
                parent.contacts.push(*value);
            }
            (FrameKind::Contact(parent), FrameKind::Name(value)) => parent.names.push(value),
            (FrameKind::Contact(parent), FrameKind::Text(value)) => match tag {
                Tag::Email => parent.emails.push(value),
                Tag::Phone => parent.phones.push(value),
                _ => unreachable!("unexpected contact text child"),
            },
            (FrameKind::Contact(parent), FrameKind::Address(value)) => {
                parent.addresses.push(*value);
            }
            (FrameKind::Address(parent), FrameKind::Text(value)) => match tag {
                Tag::Street => parent.streets.push(value),
                Tag::Apartment => parent.apartment = Some(value),
                Tag::City => parent.city = Some(value),
                Tag::RegionCode => parent.region_code = Some(value),
                Tag::PostalCode => parent.postal_code = Some(value),
                Tag::Country => parent.country = Some(value),
                _ => unreachable!("unexpected address text child"),
            },
            _ => unreachable!("typed child was classified for a different parent"),
        }
        None
    }
}

fn text_part(node: XmlNode<'_>) -> TextPart<'_> {
    match node {
        XmlNode::Text(value) => TextPart::Text(value),
        XmlNode::CData(value) => TextPart::CData(value),
        XmlNode::EntityRef(value) => TextPart::EntityRef(value),
        node => TextPart::Node(node),
    }
}
