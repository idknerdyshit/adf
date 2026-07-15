use crate::{Attribute, Span, XmlNode};
use std::borrow::Cow;

/// Root typed representation of an ADF document.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
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
#[non_exhaustive]
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
#[non_exhaustive]
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
#[non_exhaustive]
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
#[non_exhaustive]
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
#[non_exhaustive]
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
#[non_exhaustive]
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
#[non_exhaustive]
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
#[non_exhaustive]
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
#[non_exhaustive]
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
#[non_exhaustive]
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
#[non_exhaustive]
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
#[non_exhaustive]
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
#[non_exhaustive]
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
#[non_exhaustive]
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
#[non_exhaustive]
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

impl<'a> From<&'a str> for TextElement<'a> {
    fn from(value: &'a str) -> Self {
        Self::new(Cow::Borrowed(value), Vec::new())
    }
}

impl<'a> From<String> for TextElement<'a> {
    fn from(value: String) -> Self {
        Self::new(Cow::Owned(value), Vec::new())
    }
}

/// Required contact channel used when constructing a contact.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ContactMethod<'a> {
    Email(TextElement<'a>),
    Phone(TextElement<'a>),
}

/// Required color content used when constructing a color combination.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ColorSelection<'a> {
    Interior(TextElement<'a>),
    Exterior(TextElement<'a>),
    Both {
        interior: TextElement<'a>,
        exterior: TextElement<'a>,
    },
}

/// Required date content used when constructing a customer timeframe.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TimeframeWindow<'a> {
    Earliest(TextElement<'a>),
    Latest(TextElement<'a>),
    Range {
        earliest: TextElement<'a>,
        latest: TextElement<'a>,
    },
}

macro_rules! builder_type {
    ($builder:ident, $model:ident) => {
        #[doc = concat!("Builder for [`", stringify!($model), "`].")]
        #[derive(Debug, Clone)]
        pub struct $builder<'a> {
            value: $model<'a>,
        }

        impl<'a> $builder<'a> {
            /// Finish building the value.
            pub fn build(self) -> $model<'a> {
                self.value
            }
        }
    };
}

builder_type!(AdfBuilder, Adf);
builder_type!(ProspectBuilder, Prospect);
builder_type!(VehicleBuilder, Vehicle);
builder_type!(ColorCombinationBuilder, ColorCombination);
builder_type!(VehicleOptionBuilder, VehicleOption);
builder_type!(FinanceBuilder, Finance);
builder_type!(CustomerBuilder, Customer);
builder_type!(TimeframeBuilder, Timeframe);
builder_type!(VendorBuilder, Vendor);
builder_type!(ProviderBuilder, Provider);
builder_type!(ContactBuilder, Contact);
builder_type!(AddressBuilder, Address);
builder_type!(IdBuilder, Id);
builder_type!(PriceBuilder, Price);
builder_type!(NameBuilder, Name);
builder_type!(TextElementBuilder, TextElement);

impl<'a> Adf<'a> {
    pub fn builder(prospect: Prospect<'a>) -> AdfBuilder<'a> {
        AdfBuilder {
            value: Self {
                prospects: vec![prospect],
                ..Self::default()
            },
        }
    }
}

impl<'a> Prospect<'a> {
    pub fn builder(
        request_date: impl Into<TextElement<'a>>,
        vehicle: Vehicle<'a>,
        customer: Customer<'a>,
        vendor: Vendor<'a>,
    ) -> ProspectBuilder<'a> {
        ProspectBuilder {
            value: Self {
                request_date: Some(request_date.into()),
                vehicles: vec![vehicle],
                customer: Some(customer),
                vendor: Some(vendor),
                ..Self::default()
            },
        }
    }
}

impl<'a> Vehicle<'a> {
    pub fn builder(
        year: impl Into<TextElement<'a>>,
        make: impl Into<TextElement<'a>>,
        model: impl Into<TextElement<'a>>,
    ) -> VehicleBuilder<'a> {
        VehicleBuilder {
            value: Self {
                year: Some(year.into()),
                make: Some(make.into()),
                model: Some(model.into()),
                ..Self::default()
            },
        }
    }
}

impl<'a> ColorCombination<'a> {
    pub fn builder(
        selection: ColorSelection<'a>,
        preference: impl Into<TextElement<'a>>,
    ) -> ColorCombinationBuilder<'a> {
        let (interior_color, exterior_color) = match selection {
            ColorSelection::Interior(value) => (Some(value), None),
            ColorSelection::Exterior(value) => (None, Some(value)),
            ColorSelection::Both { interior, exterior } => (Some(interior), Some(exterior)),
        };
        ColorCombinationBuilder {
            value: Self {
                interior_color,
                exterior_color,
                preference: Some(preference.into()),
                ..Self::default()
            },
        }
    }
}

impl<'a> VehicleOption<'a> {
    pub fn builder(
        option_name: impl Into<TextElement<'a>>,
        weighting: impl Into<TextElement<'a>>,
    ) -> VehicleOptionBuilder<'a> {
        VehicleOptionBuilder {
            value: Self {
                option_name: Some(option_name.into()),
                weighting: Some(weighting.into()),
                ..Self::default()
            },
        }
    }
}

impl<'a> Finance<'a> {
    pub fn builder(
        method: impl Into<TextElement<'a>>,
        amount: impl Into<TextElement<'a>>,
    ) -> FinanceBuilder<'a> {
        FinanceBuilder {
            value: Self {
                method: Some(method.into()),
                amounts: vec![amount.into()],
                ..Self::default()
            },
        }
    }
}

impl<'a> Customer<'a> {
    pub fn builder(contact: Contact<'a>) -> CustomerBuilder<'a> {
        CustomerBuilder {
            value: Self {
                contacts: vec![contact],
                ..Self::default()
            },
        }
    }
}

impl<'a> Timeframe<'a> {
    pub fn builder(window: TimeframeWindow<'a>) -> TimeframeBuilder<'a> {
        let (earliest_date, latest_date) = match window {
            TimeframeWindow::Earliest(value) => (Some(value), None),
            TimeframeWindow::Latest(value) => (None, Some(value)),
            TimeframeWindow::Range { earliest, latest } => (Some(earliest), Some(latest)),
        };
        TimeframeBuilder {
            value: Self {
                earliest_date,
                latest_date,
                ..Self::default()
            },
        }
    }
}

impl<'a> Vendor<'a> {
    pub fn builder(
        vendor_name: impl Into<TextElement<'a>>,
        contact: Contact<'a>,
    ) -> VendorBuilder<'a> {
        VendorBuilder {
            value: Self {
                vendor_name: Some(vendor_name.into()),
                contacts: vec![contact],
                ..Self::default()
            },
        }
    }
}

impl<'a> Provider<'a> {
    pub fn builder(name: Name<'a>) -> ProviderBuilder<'a> {
        ProviderBuilder {
            value: Self {
                name: Some(name),
                ..Self::default()
            },
        }
    }
}

impl<'a> Contact<'a> {
    pub fn builder(name: Name<'a>, method: ContactMethod<'a>) -> ContactBuilder<'a> {
        let (emails, phones) = match method {
            ContactMethod::Email(value) => (vec![value], Vec::new()),
            ContactMethod::Phone(value) => (Vec::new(), vec![value]),
        };
        ContactBuilder {
            value: Self {
                names: vec![name],
                emails,
                phones,
                ..Self::default()
            },
        }
    }
}

impl<'a> Address<'a> {
    pub fn builder(street: impl Into<TextElement<'a>>) -> AddressBuilder<'a> {
        AddressBuilder {
            value: Self {
                streets: vec![street.into()],
                ..Self::default()
            },
        }
    }
}

impl<'a> Id<'a> {
    pub fn new(value: impl Into<Cow<'a, str>>, source: impl Into<Cow<'a, str>>) -> Self {
        Self {
            source: Some(source.into()),
            parts: vec![TextPart::Text(value.into())],
            ..Self::default()
        }
    }

    pub fn builder(
        value: impl Into<Cow<'a, str>>,
        source: impl Into<Cow<'a, str>>,
    ) -> IdBuilder<'a> {
        IdBuilder {
            value: Self::new(value, source),
        }
    }
}

impl<'a> Price<'a> {
    pub fn new(value: impl Into<Cow<'a, str>>) -> Self {
        Self {
            parts: vec![TextPart::Text(value.into())],
            ..Self::default()
        }
    }

    pub fn builder(value: impl Into<Cow<'a, str>>) -> PriceBuilder<'a> {
        PriceBuilder {
            value: Self::new(value),
        }
    }
}

impl<'a> Name<'a> {
    pub fn new(value: impl Into<Cow<'a, str>>) -> Self {
        Self {
            parts: vec![TextPart::Text(value.into())],
            ..Self::default()
        }
    }

    pub fn builder(value: impl Into<Cow<'a, str>>) -> NameBuilder<'a> {
        NameBuilder {
            value: Self::new(value),
        }
    }
}

impl<'a> TextElement<'a> {
    pub fn builder(value: impl Into<Cow<'a, str>>) -> TextElementBuilder<'a> {
        TextElementBuilder {
            value: Self::new(value.into(), Vec::new()),
        }
    }
}

macro_rules! add_attributes {
    ($($builder:ident),+ $(,)?) => {$(
        impl<'a> $builder<'a> {
            pub fn attribute(mut self, name: impl Into<Cow<'a, str>>, value: impl Into<Cow<'a, str>>) -> Self {
                self.value.attributes.push(Attribute::new(name, value));
                self
            }
        }
    )+};
}

add_attributes!(
    AdfBuilder,
    ProspectBuilder,
    VehicleBuilder,
    ColorCombinationBuilder,
    VehicleOptionBuilder,
    FinanceBuilder,
    CustomerBuilder,
    TimeframeBuilder,
    VendorBuilder,
    ProviderBuilder,
    ContactBuilder,
    AddressBuilder,
    IdBuilder,
    PriceBuilder,
    NameBuilder,
    TextElementBuilder
);

macro_rules! add_extensions {
    ($($builder:ident),+ $(,)?) => {$(
        impl<'a> $builder<'a> {
            pub fn extension(mut self, extension: XmlNode<'a>) -> Self {
                self.value.extensions.push(extension);
                self
            }
        }
    )+};
}

add_extensions!(
    AdfBuilder,
    ProspectBuilder,
    VehicleBuilder,
    ColorCombinationBuilder,
    VehicleOptionBuilder,
    FinanceBuilder,
    CustomerBuilder,
    TimeframeBuilder,
    VendorBuilder,
    ProviderBuilder,
    ContactBuilder,
    AddressBuilder
);

impl<'a> AdfBuilder<'a> {
    pub fn prospect(mut self, prospect: Prospect<'a>) -> Self {
        self.value.prospects.push(prospect);
        self
    }
}

impl<'a> ProspectBuilder<'a> {
    pub fn status(mut self, value: impl Into<Cow<'a, str>>) -> Self {
        self.value.status = Some(value.into());
        self
    }
    pub fn id(mut self, value: Id<'a>) -> Self {
        self.value.ids.push(value);
        self
    }
    pub fn vehicle(mut self, value: Vehicle<'a>) -> Self {
        self.value.vehicles.push(value);
        self
    }
    pub fn provider(mut self, value: Provider<'a>) -> Self {
        self.value.provider = Some(value);
        self
    }
}

macro_rules! vehicle_text_setters {
    ($($method:ident => $field:ident),+ $(,)?) => {$(
        pub fn $method(mut self, value: impl Into<TextElement<'a>>) -> Self {
            self.value.$field = Some(value.into()); self
        }
    )+};
}

impl<'a> VehicleBuilder<'a> {
    vehicle_text_setters!(vin => vin, stock => stock, trim => trim, doors => doors,
        body_style => body_style, transmission => transmission, odometer => odometer,
        condition => condition, price_comments => price_comments, comments => comments);
    pub fn id(mut self, value: Id<'a>) -> Self {
        self.value.ids.push(value);
        self
    }
    pub fn color_combination(mut self, value: ColorCombination<'a>) -> Self {
        self.value.color_combinations.push(value);
        self
    }
    pub fn image_tag(mut self, value: impl Into<TextElement<'a>>) -> Self {
        self.value.image_tags.push(value.into());
        self
    }
    pub fn price(mut self, value: Price<'a>) -> Self {
        self.value.prices.push(value);
        self
    }
    pub fn option(mut self, value: VehicleOption<'a>) -> Self {
        self.value.options.push(value);
        self
    }
    pub fn finance(mut self, value: Finance<'a>) -> Self {
        self.value.finance = Some(value);
        self
    }
    pub fn interest(mut self, value: impl Into<Cow<'a, str>>) -> Self {
        self.value.interest = Some(value.into());
        self
    }
    pub fn status(mut self, value: impl Into<Cow<'a, str>>) -> Self {
        self.value.status = Some(value.into());
        self
    }
}

impl<'a> VehicleOptionBuilder<'a> {
    pub fn manufacturer_code(mut self, value: impl Into<TextElement<'a>>) -> Self {
        self.value.manufacturer_code = Some(value.into());
        self
    }
    pub fn stock(mut self, value: impl Into<TextElement<'a>>) -> Self {
        self.value.stock = Some(value.into());
        self
    }
    pub fn price(mut self, value: Price<'a>) -> Self {
        self.value.prices.push(value);
        self
    }
}

impl<'a> FinanceBuilder<'a> {
    pub fn amount(mut self, value: impl Into<TextElement<'a>>) -> Self {
        self.value.amounts.push(value.into());
        self
    }
    pub fn balance(mut self, value: impl Into<TextElement<'a>>) -> Self {
        self.value.balances.push(value.into());
        self
    }
}

impl<'a> CustomerBuilder<'a> {
    pub fn id(mut self, value: Id<'a>) -> Self {
        self.value.ids.push(value);
        self
    }
    pub fn timeframe(mut self, value: Timeframe<'a>) -> Self {
        self.value.timeframe = Some(value);
        self
    }
    pub fn comments(mut self, value: impl Into<TextElement<'a>>) -> Self {
        self.value.comments = Some(value.into());
        self
    }
}

impl<'a> TimeframeBuilder<'a> {
    pub fn description(mut self, value: impl Into<TextElement<'a>>) -> Self {
        self.value.description = Some(value.into());
        self
    }
}

impl<'a> VendorBuilder<'a> {
    pub fn id(mut self, value: Id<'a>) -> Self {
        self.value.ids.push(value);
        self
    }
    pub fn url(mut self, value: impl Into<TextElement<'a>>) -> Self {
        self.value.url = Some(value.into());
        self
    }
}

impl<'a> ProviderBuilder<'a> {
    pub fn id(mut self, value: Id<'a>) -> Self {
        self.value.ids.push(value);
        self
    }
    pub fn service(mut self, value: impl Into<TextElement<'a>>) -> Self {
        self.value.service = Some(value.into());
        self
    }
    pub fn url(mut self, value: impl Into<TextElement<'a>>) -> Self {
        self.value.url = Some(value.into());
        self
    }
    pub fn email(mut self, value: impl Into<TextElement<'a>>) -> Self {
        self.value.email = Some(value.into());
        self
    }
    pub fn phone(mut self, value: impl Into<TextElement<'a>>) -> Self {
        self.value.phone = Some(value.into());
        self
    }
    pub fn contact(mut self, value: Contact<'a>) -> Self {
        self.value.contacts.push(value);
        self
    }
}

impl<'a> ContactBuilder<'a> {
    pub fn primary_contact(mut self, value: bool) -> Self {
        self.value.primary_contact = Some(Cow::Borrowed(if value { "1" } else { "0" }));
        self
    }
    pub fn name(mut self, value: Name<'a>) -> Self {
        self.value.names.push(value);
        self
    }
    pub fn email(mut self, value: impl Into<TextElement<'a>>) -> Self {
        self.value.emails.push(value.into());
        self
    }
    pub fn phone(mut self, value: impl Into<TextElement<'a>>) -> Self {
        self.value.phones.push(value.into());
        self
    }
    pub fn address(mut self, value: Address<'a>) -> Self {
        self.value.addresses.push(value);
        self
    }
}

impl<'a> AddressBuilder<'a> {
    pub fn address_type(mut self, value: impl Into<Cow<'a, str>>) -> Self {
        self.value.address_type = Some(value.into());
        self
    }
    pub fn street(mut self, value: impl Into<TextElement<'a>>) -> Self {
        self.value.streets.push(value.into());
        self
    }
    pub fn apartment(mut self, value: impl Into<TextElement<'a>>) -> Self {
        self.value.apartment = Some(value.into());
        self
    }
    pub fn city(mut self, value: impl Into<TextElement<'a>>) -> Self {
        self.value.city = Some(value.into());
        self
    }
    pub fn region_code(mut self, value: impl Into<TextElement<'a>>) -> Self {
        self.value.region_code = Some(value.into());
        self
    }
    pub fn postal_code(mut self, value: impl Into<TextElement<'a>>) -> Self {
        self.value.postal_code = Some(value.into());
        self
    }
    pub fn country(mut self, value: impl Into<TextElement<'a>>) -> Self {
        self.value.country = Some(value.into());
        self
    }
}

impl<'a> IdBuilder<'a> {
    pub fn sequence(mut self, value: impl Into<Cow<'a, str>>) -> Self {
        self.value.sequence = Some(value.into());
        self
    }
}

impl<'a> PriceBuilder<'a> {
    pub fn price_type(mut self, value: impl Into<Cow<'a, str>>) -> Self {
        self.value.price_type = Some(value.into());
        self
    }
    pub fn currency(mut self, value: impl Into<Cow<'a, str>>) -> Self {
        self.value.currency = Some(value.into());
        self
    }
    pub fn delta(mut self, value: impl Into<Cow<'a, str>>) -> Self {
        self.value.delta = Some(value.into());
        self
    }
    pub fn relative_to(mut self, value: impl Into<Cow<'a, str>>) -> Self {
        self.value.relative_to = Some(value.into());
        self
    }
    pub fn source(mut self, value: impl Into<Cow<'a, str>>) -> Self {
        self.value.source = Some(value.into());
        self
    }
}

impl<'a> NameBuilder<'a> {
    pub fn part(mut self, value: impl Into<Cow<'a, str>>) -> Self {
        self.value.part = Some(value.into());
        self
    }
    pub fn name_type(mut self, value: impl Into<Cow<'a, str>>) -> Self {
        self.value.name_type = Some(value.into());
        self
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

fn owned_cow(value: Cow<'_, str>) -> Cow<'static, str> {
    Cow::Owned(value.into_owned())
}

fn owned_attributes(values: Vec<Attribute<'_>>) -> Vec<Attribute<'static>> {
    values.into_iter().map(Attribute::into_owned).collect()
}

fn owned_extensions(values: Vec<XmlNode<'_>>) -> Vec<XmlNode<'static>> {
    values.into_iter().map(XmlNode::into_owned).collect()
}

impl TextPart<'_> {
    pub fn into_owned(self) -> TextPart<'static> {
        match self {
            TextPart::Text(value) => TextPart::Text(owned_cow(value)),
            TextPart::CData(value) => TextPart::CData(owned_cow(value)),
            TextPart::EntityRef(value) => TextPart::EntityRef(owned_cow(value)),
            TextPart::Node(value) => TextPart::Node(value.into_owned()),
        }
    }
}

macro_rules! own_text_model {
    ($type:ident { $($field:ident),* $(,)? }) => {
        impl $type<'_> {
            pub fn into_owned(self) -> $type<'static> {
                $type {
                    $($field: self.$field.map(owned_cow),)*
                    parts: self.parts.into_iter().map(TextPart::into_owned).collect(),
                    attributes: owned_attributes(self.attributes),
                    span: self.span,
                }
            }
        }
    };
}

own_text_model!(Id { sequence, source });
own_text_model!(Price {
    price_type,
    currency,
    delta,
    relative_to,
    source
});
own_text_model!(Name { part, name_type });

impl TextElement<'_> {
    pub fn into_owned(self) -> TextElement<'static> {
        TextElement {
            parts: self.parts.into_iter().map(TextPart::into_owned).collect(),
            attributes: owned_attributes(self.attributes),
            span: self.span,
        }
    }
}

impl Adf<'_> {
    pub fn into_owned(self) -> Adf<'static> {
        Adf {
            prospects: self
                .prospects
                .into_iter()
                .map(Prospect::into_owned)
                .collect(),
            extensions: owned_extensions(self.extensions),
            attributes: owned_attributes(self.attributes),
            span: self.span,
        }
    }
}

impl Prospect<'_> {
    pub fn into_owned(self) -> Prospect<'static> {
        Prospect {
            status: self.status.map(owned_cow),
            ids: self.ids.into_iter().map(Id::into_owned).collect(),
            request_date: self.request_date.map(TextElement::into_owned),
            vehicles: self.vehicles.into_iter().map(Vehicle::into_owned).collect(),
            customer: self.customer.map(Customer::into_owned),
            vendor: self.vendor.map(Vendor::into_owned),
            provider: self.provider.map(Provider::into_owned),
            extensions: owned_extensions(self.extensions),
            attributes: owned_attributes(self.attributes),
            span: self.span,
        }
    }
}

impl Vehicle<'_> {
    pub fn into_owned(self) -> Vehicle<'static> {
        Vehicle {
            interest: self.interest.map(owned_cow),
            status: self.status.map(owned_cow),
            ids: self.ids.into_iter().map(Id::into_owned).collect(),
            year: self.year.map(TextElement::into_owned),
            make: self.make.map(TextElement::into_owned),
            model: self.model.map(TextElement::into_owned),
            vin: self.vin.map(TextElement::into_owned),
            stock: self.stock.map(TextElement::into_owned),
            trim: self.trim.map(TextElement::into_owned),
            doors: self.doors.map(TextElement::into_owned),
            body_style: self.body_style.map(TextElement::into_owned),
            transmission: self.transmission.map(TextElement::into_owned),
            odometer: self.odometer.map(TextElement::into_owned),
            condition: self.condition.map(TextElement::into_owned),
            color_combinations: self
                .color_combinations
                .into_iter()
                .map(ColorCombination::into_owned)
                .collect(),
            image_tags: self
                .image_tags
                .into_iter()
                .map(TextElement::into_owned)
                .collect(),
            prices: self.prices.into_iter().map(Price::into_owned).collect(),
            price_comments: self.price_comments.map(TextElement::into_owned),
            options: self
                .options
                .into_iter()
                .map(VehicleOption::into_owned)
                .collect(),
            finance: self.finance.map(Finance::into_owned),
            comments: self.comments.map(TextElement::into_owned),
            extensions: owned_extensions(self.extensions),
            attributes: owned_attributes(self.attributes),
            span: self.span,
        }
    }
}

impl ColorCombination<'_> {
    pub fn into_owned(self) -> ColorCombination<'static> {
        ColorCombination {
            interior_color: self.interior_color.map(TextElement::into_owned),
            exterior_color: self.exterior_color.map(TextElement::into_owned),
            preference: self.preference.map(TextElement::into_owned),
            extensions: owned_extensions(self.extensions),
            attributes: owned_attributes(self.attributes),
            span: self.span,
        }
    }
}
impl VehicleOption<'_> {
    pub fn into_owned(self) -> VehicleOption<'static> {
        VehicleOption {
            option_name: self.option_name.map(TextElement::into_owned),
            manufacturer_code: self.manufacturer_code.map(TextElement::into_owned),
            stock: self.stock.map(TextElement::into_owned),
            weighting: self.weighting.map(TextElement::into_owned),
            prices: self.prices.into_iter().map(Price::into_owned).collect(),
            extensions: owned_extensions(self.extensions),
            attributes: owned_attributes(self.attributes),
            span: self.span,
        }
    }
}
impl Finance<'_> {
    pub fn into_owned(self) -> Finance<'static> {
        Finance {
            method: self.method.map(TextElement::into_owned),
            amounts: self
                .amounts
                .into_iter()
                .map(TextElement::into_owned)
                .collect(),
            balances: self
                .balances
                .into_iter()
                .map(TextElement::into_owned)
                .collect(),
            extensions: owned_extensions(self.extensions),
            attributes: owned_attributes(self.attributes),
            span: self.span,
        }
    }
}
impl Customer<'_> {
    pub fn into_owned(self) -> Customer<'static> {
        Customer {
            ids: self.ids.into_iter().map(Id::into_owned).collect(),
            contacts: self.contacts.into_iter().map(Contact::into_owned).collect(),
            timeframe: self.timeframe.map(Timeframe::into_owned),
            comments: self.comments.map(TextElement::into_owned),
            extensions: owned_extensions(self.extensions),
            attributes: owned_attributes(self.attributes),
            span: self.span,
        }
    }
}
impl Timeframe<'_> {
    pub fn into_owned(self) -> Timeframe<'static> {
        Timeframe {
            description: self.description.map(TextElement::into_owned),
            earliest_date: self.earliest_date.map(TextElement::into_owned),
            latest_date: self.latest_date.map(TextElement::into_owned),
            extensions: owned_extensions(self.extensions),
            attributes: owned_attributes(self.attributes),
            span: self.span,
        }
    }
}
impl Vendor<'_> {
    pub fn into_owned(self) -> Vendor<'static> {
        Vendor {
            ids: self.ids.into_iter().map(Id::into_owned).collect(),
            vendor_name: self.vendor_name.map(TextElement::into_owned),
            url: self.url.map(TextElement::into_owned),
            contacts: self.contacts.into_iter().map(Contact::into_owned).collect(),
            extensions: owned_extensions(self.extensions),
            attributes: owned_attributes(self.attributes),
            span: self.span,
        }
    }
}
impl Provider<'_> {
    pub fn into_owned(self) -> Provider<'static> {
        Provider {
            ids: self.ids.into_iter().map(Id::into_owned).collect(),
            name: self.name.map(Name::into_owned),
            service: self.service.map(TextElement::into_owned),
            url: self.url.map(TextElement::into_owned),
            email: self.email.map(TextElement::into_owned),
            phone: self.phone.map(TextElement::into_owned),
            contacts: self.contacts.into_iter().map(Contact::into_owned).collect(),
            extensions: owned_extensions(self.extensions),
            attributes: owned_attributes(self.attributes),
            span: self.span,
        }
    }
}
impl Contact<'_> {
    pub fn into_owned(self) -> Contact<'static> {
        Contact {
            primary_contact: self.primary_contact.map(owned_cow),
            names: self.names.into_iter().map(Name::into_owned).collect(),
            emails: self
                .emails
                .into_iter()
                .map(TextElement::into_owned)
                .collect(),
            phones: self
                .phones
                .into_iter()
                .map(TextElement::into_owned)
                .collect(),
            addresses: self
                .addresses
                .into_iter()
                .map(Address::into_owned)
                .collect(),
            extensions: owned_extensions(self.extensions),
            attributes: owned_attributes(self.attributes),
            span: self.span,
        }
    }
}
impl Address<'_> {
    pub fn into_owned(self) -> Address<'static> {
        Address {
            address_type: self.address_type.map(owned_cow),
            streets: self
                .streets
                .into_iter()
                .map(TextElement::into_owned)
                .collect(),
            apartment: self.apartment.map(TextElement::into_owned),
            city: self.city.map(TextElement::into_owned),
            region_code: self.region_code.map(TextElement::into_owned),
            postal_code: self.postal_code.map(TextElement::into_owned),
            country: self.country.map(TextElement::into_owned),
            extensions: owned_extensions(self.extensions),
            attributes: owned_attributes(self.attributes),
            span: self.span,
        }
    }
}

impl ContactMethod<'_> {
    pub fn into_owned(self) -> ContactMethod<'static> {
        match self {
            ContactMethod::Email(value) => ContactMethod::Email(value.into_owned()),
            ContactMethod::Phone(value) => ContactMethod::Phone(value.into_owned()),
        }
    }
}
impl ColorSelection<'_> {
    pub fn into_owned(self) -> ColorSelection<'static> {
        match self {
            ColorSelection::Interior(value) => ColorSelection::Interior(value.into_owned()),
            ColorSelection::Exterior(value) => ColorSelection::Exterior(value.into_owned()),
            ColorSelection::Both { interior, exterior } => ColorSelection::Both {
                interior: interior.into_owned(),
                exterior: exterior.into_owned(),
            },
        }
    }
}
impl TimeframeWindow<'_> {
    pub fn into_owned(self) -> TimeframeWindow<'static> {
        match self {
            TimeframeWindow::Earliest(value) => TimeframeWindow::Earliest(value.into_owned()),
            TimeframeWindow::Latest(value) => TimeframeWindow::Latest(value.into_owned()),
            TimeframeWindow::Range { earliest, latest } => TimeframeWindow::Range {
                earliest: earliest.into_owned(),
                latest: latest.into_owned(),
            },
        }
    }
}
