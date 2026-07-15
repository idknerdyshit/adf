use crate::document::Span;
use crate::model::{
    Address, Adf, ColorCombination, Contact, Customer, Finance, Id, Price, Prospect, Provider,
    Timeframe, Vehicle, VehicleOption, Vendor,
};
use crate::{Attribute, TextElement, TextPart, XmlNode};
use std::borrow::Cow;

/// Validation severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Advisory issue; the parsed document may still be usable.
    Warning,
    /// Structural issue severe enough for [`ValidationReport::is_valid`] to fail.
    Error,
}

/// Machine-readable validation finding category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValidationCode {
    Advisory,
    MissingRequired,
    Duplicate,
    Excessive,
    OutOfOrder,
    UnexpectedElement,
    UnexpectedAttribute,
    InvalidEnum,
    InvalidFormat,
    InvalidRange,
}

/// Validation behavior to apply to a typed ADF model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum ValidationProfile {
    #[default]
    Lenient,
    Structural,
    Adf10,
    Adf10Extended,
}

impl ValidationProfile {
    fn is_conformance(self) -> bool {
        matches!(self, Self::Adf10 | Self::Adf10Extended)
    }

    fn rejects_extensions(self) -> bool {
        matches!(self, Self::Adf10)
    }
}

/// One validation finding with a model path and optional source span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue<'a> {
    /// Machine-readable category for this finding.
    pub code: ValidationCode,
    /// Whether this issue is a warning or error.
    pub severity: Severity,
    /// Dot-style path into the typed ADF model.
    pub path: Cow<'a, str>,
    /// Human-readable issue text. This never includes raw lead payload values
    /// beyond short invalid enum/format samples.
    pub message: Cow<'a, str>,
    /// Byte span in the original input when the issue maps to parsed XML.
    pub span: Option<Span>,
}

/// Collection of validation findings for an ADF model.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ValidationReport<'a> {
    /// Findings emitted during validation.
    pub issues: Vec<ValidationIssue<'a>>,
    profile: ValidationProfile,
}

impl ValidationReport<'_> {
    /// Return `true` when the report contains no [`Severity::Error`] issues.
    pub fn is_valid(&self) -> bool {
        !self
            .issues
            .iter()
            .any(|issue| issue.severity == Severity::Error)
    }
}

/// Options selecting structural or ADF 1.0 conformance validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct ValidationOptions {
    /// Selected validation profile.
    pub profile: ValidationProfile,
}

impl ValidationOptions {
    /// Promote structural requirement checks from warnings to errors.
    ///
    /// This does not promote enum, date, country, or currency shape warnings.
    #[must_use]
    pub fn strict(mut self, strict: bool) -> Self {
        self.profile = if strict {
            ValidationProfile::Structural
        } else {
            ValidationProfile::Lenient
        };
        self
    }

    /// Select an explicit validation profile.
    #[must_use]
    pub fn profile(mut self, profile: ValidationProfile) -> Self {
        self.profile = profile;
        self
    }
}

#[derive(Debug, Clone, Copy)]
struct AllowedValues {
    values: &'static [&'static str],
    display: &'static str,
}

const PROSPECT_STATUS: AllowedValues = AllowedValues {
    values: &["new", "resend"],
    display: "new, resend",
};
const VEHICLE_INTEREST: AllowedValues = AllowedValues {
    values: &["buy", "lease", "sell", "trade-in", "test-drive"],
    display: "buy, lease, sell, trade-in, test-drive",
};
const VEHICLE_STATUS: AllowedValues = AllowedValues {
    values: &["new", "used"],
    display: "new, used",
};
const PRICE_TYPE: AllowedValues = AllowedValues {
    values: &[
        "quote",
        "offer",
        "msrp",
        "invoice",
        "call",
        "appraisal",
        "asking",
    ],
    display: "quote, offer, msrp, invoice, call, appraisal, asking",
};
const PRICE_DELTA: AllowedValues = AllowedValues {
    values: &["absolute", "relative", "percentage"],
    display: "absolute, relative, percentage",
};
const PRICE_RELATIVE_TO: AllowedValues = AllowedValues {
    values: &["msrp", "invoice"],
    display: "msrp, invoice",
};
const NAME_PART: AllowedValues = AllowedValues {
    values: &["first", "middle", "suffix", "last", "full"],
    display: "first, middle, suffix, last, full",
};
const NAME_TYPE: AllowedValues = AllowedValues {
    values: &["business", "individual"],
    display: "business, individual",
};
const BOOL_FLAG: AllowedValues = AllowedValues {
    values: &["0", "1"],
    display: "0, 1",
};
const PHONE_TYPE: AllowedValues = AllowedValues {
    values: &["voice", "fax", "cellphone", "pager"],
    display: "voice, fax, cellphone, pager",
};
const PHONE_TIME: AllowedValues = AllowedValues {
    values: &["morning", "afternoon", "evening", "nopreference", "day"],
    display: "morning, afternoon, evening, nopreference, day",
};
const ADDRESS_TYPE: AllowedValues = AllowedValues {
    values: &["work", "home", "delivery"],
    display: "work, home, delivery",
};
const ODOMETER_STATUS: AllowedValues = AllowedValues {
    values: &["unknown", "rolledover", "replaced", "original"],
    display: "unknown, rolledover, replaced, original",
};
const ODOMETER_UNITS: AllowedValues = AllowedValues {
    values: &["km", "mi"],
    display: "km, mi",
};
const CONDITION_VALUES: &[&str] = &["excellent", "good", "fair", "poor", "unknown"];
const FINANCE_METHOD: &[&str] = &["cash", "finance", "lease"];
const AMOUNT_TYPE: AllowedValues = AllowedValues {
    values: &[
        "downpayment",
        "tradein",
        "rebate",
        "total",
        "monthly",
        "fee",
        "tax",
        "other",
    ],
    display: "downpayment, tradein, rebate, total, monthly, fee, tax, other",
};
const AMOUNT_LIMIT: AllowedValues = AllowedValues {
    values: &["minimum", "maximum", "exact"],
    display: "minimum, maximum, exact",
};
const BALANCE_TYPE: AllowedValues = AllowedValues {
    values: &["finance", "residual", "payoff", "other"],
    display: "finance, residual, payoff, other",
};

/// Validate a typed ADF model with lenient default options.
pub fn validate<'a>(adf: &Adf<'a>) -> ValidationReport<'a> {
    validate_with(adf, ValidationOptions::default())
}

/// Validate against the exact ADF 1.0 profile.
pub fn validate_adf_1_0<'a>(adf: &Adf<'a>) -> ValidationReport<'a> {
    validate_with(
        adf,
        ValidationOptions::default().profile(ValidationProfile::Adf10),
    )
}

/// Validate ADF 1.0 while permitting partner extensions.
pub fn validate_adf_1_0_extended<'a>(adf: &Adf<'a>) -> ValidationReport<'a> {
    validate_with(
        adf,
        ValidationOptions::default().profile(ValidationProfile::Adf10Extended),
    )
}

/// Validate a typed ADF model with explicit options.
///
/// Lenient and structural profiles retain the crate's compatibility-oriented
/// checks. The two ADF 1.0 profiles enforce the standard's complete content
/// model, values, formats, ranges, and standard-name placement.
pub fn validate_with<'a>(adf: &Adf<'a>, options: ValidationOptions) -> ValidationReport<'a> {
    let span = tracing::debug_span!("adf.validate", profile = ?options.profile);
    let _span_guard = span.enter();

    let mut report = ValidationReport {
        issues: Vec::new(),
        profile: options.profile,
    };

    report.required(
        options,
        "adf",
        adf.span,
        !adf.prospects.is_empty(),
        "ADF document should contain at least one prospect",
    );

    for (index, prospect) in adf.prospects.iter().enumerate() {
        let path = format!("adf.prospect[{index}]");
        validate_prospect(&mut report, &path, prospect, options);
    }

    if options.profile.is_conformance() {
        validate_conformance(&mut report, adf);
    }

    if tracing::enabled!(tracing::Level::DEBUG) {
        let stats = crate::trace::DocumentStats::from_adf(adf);
        let (warnings, errors) = crate::trace::validation_issue_counts(&report);
        tracing::debug!(
            prospects = stats.prospects,
            vehicles = stats.vehicles,
            contacts = stats.contacts,
            addresses = stats.addresses,
            extensions = stats.extensions,
            warnings,
            errors,
            "ADF validation complete"
        );
    }
    report
}

fn validate_prospect(
    report: &mut ValidationReport<'_>,
    path: &str,
    prospect: &Prospect<'_>,
    options: ValidationOptions,
) {
    report.required(
        options,
        path,
        prospect.span,
        prospect.request_date.is_some(),
        "prospect is missing requestdate",
    );
    report.required(
        options,
        path,
        prospect.span,
        !prospect.vehicles.is_empty(),
        "prospect is missing vehicle",
    );
    report.required(
        options,
        path,
        prospect.span,
        prospect.customer.is_some(),
        "prospect is missing customer",
    );
    report.required(
        options,
        path,
        prospect.span,
        prospect.vendor.is_some(),
        "prospect is missing vendor",
    );

    check_enum(
        report,
        || format!("{path}@status"),
        prospect.span,
        prospect.status.as_deref(),
        PROSPECT_STATUS,
    );

    if let Some(date) = &prospect.request_date {
        check_iso_datetime(
            report,
            || format!("{path}.requestdate"),
            date.span,
            &date.value(),
        );
    }

    if let Some(customer) = &prospect.customer {
        validate_customer(report, path, customer, options);
    }
    if let Some(vendor) = &prospect.vendor {
        validate_vendor(report, path, vendor, options);
    }
    if let Some(provider) = &prospect.provider {
        validate_provider(report, path, provider, options);
    }
    for (vehicle_index, vehicle) in prospect.vehicles.iter().enumerate() {
        validate_vehicle(
            report,
            &format!("{path}.vehicle[{vehicle_index}]"),
            vehicle,
            options,
        );
    }
}

fn validate_customer(
    report: &mut ValidationReport<'_>,
    prospect_path: &str,
    customer: &Customer<'_>,
    options: ValidationOptions,
) {
    let customer_path = format!("{prospect_path}.customer");
    report.required(
        options,
        &customer_path,
        customer.span,
        !customer.contacts.is_empty(),
        "customer is missing contact",
    );

    for (index, contact) in customer.contacts.iter().enumerate() {
        let path = format!("{customer_path}.contact[{index}]");
        validate_contact(report, &path, contact, options, true);
    }

    if let Some(timeframe) = &customer.timeframe {
        validate_timeframe(report, &customer_path, timeframe);
    }
}

fn validate_timeframe(
    report: &mut ValidationReport<'_>,
    customer_path: &str,
    timeframe: &Timeframe<'_>,
) {
    if let Some(date) = &timeframe.earliest_date {
        check_iso_datetime(
            report,
            || format!("{customer_path}.timeframe.earliestdate"),
            date.span,
            &date.value(),
        );
    }
    if let Some(date) = &timeframe.latest_date {
        check_iso_datetime(
            report,
            || format!("{customer_path}.timeframe.latestdate"),
            date.span,
            &date.value(),
        );
    }
}

fn validate_vendor(
    report: &mut ValidationReport<'_>,
    prospect_path: &str,
    vendor: &Vendor<'_>,
    options: ValidationOptions,
) {
    let vendor_path = format!("{prospect_path}.vendor");
    report.required(
        options,
        &vendor_path,
        vendor.span,
        vendor.vendor_name.is_some(),
        "vendor is missing vendorname",
    );
    report.required(
        options,
        &vendor_path,
        vendor.span,
        !vendor.contacts.is_empty(),
        "vendor is missing contact",
    );
    for (index, contact) in vendor.contacts.iter().enumerate() {
        let path = format!("{vendor_path}.contact[{index}]");
        validate_contact(report, &path, contact, options, false);
    }
}

fn validate_provider(
    report: &mut ValidationReport<'_>,
    prospect_path: &str,
    provider: &Provider<'_>,
    options: ValidationOptions,
) {
    let provider_path = format!("{prospect_path}.provider");
    if let Some(name) = &provider.name {
        check_enum(
            report,
            || format!("{provider_path}.name@part"),
            name.span,
            name.part.as_deref(),
            NAME_PART,
        );
        check_enum(
            report,
            || format!("{provider_path}.name@type"),
            name.span,
            name.name_type.as_deref(),
            NAME_TYPE,
        );
    }
    if let Some(email) = &provider.email {
        check_enum(
            report,
            || format!("{provider_path}.email@preferredcontact"),
            email.span,
            attr_value(&email.attributes, "preferredcontact"),
            BOOL_FLAG,
        );
    }
    if let Some(phone) = &provider.phone {
        check_phone_attributes(report, || format!("{provider_path}.phone"), phone);
    }
    for (index, contact) in provider.contacts.iter().enumerate() {
        let path = format!("{provider_path}.contact[{index}]");
        validate_contact(report, &path, contact, options, false);
    }
}

fn validate_contact(
    report: &mut ValidationReport<'_>,
    path: &str,
    contact: &Contact<'_>,
    options: ValidationOptions,
    require_email_or_phone: bool,
) {
    report.required(
        options,
        path,
        contact.span,
        !contact.names.is_empty(),
        "contact is missing name",
    );
    if require_email_or_phone && contact.emails.is_empty() && contact.phones.is_empty() {
        report.rule(
            ValidationCode::MissingRequired,
            path.to_owned(),
            contact.span,
            "contact should contain email or phone",
        );
    }

    check_enum(
        report,
        || format!("{path}@primarycontact"),
        contact.span,
        contact.primary_contact.as_deref(),
        BOOL_FLAG,
    );

    for (index, name) in contact.names.iter().enumerate() {
        check_enum(
            report,
            || format!("{path}.name[{index}]@part"),
            name.span,
            name.part.as_deref(),
            NAME_PART,
        );
        check_enum(
            report,
            || format!("{path}.name[{index}]@type"),
            name.span,
            name.name_type.as_deref(),
            NAME_TYPE,
        );
    }

    for (index, email) in contact.emails.iter().enumerate() {
        let preferred = attr_value(&email.attributes, "preferredcontact");
        check_enum(
            report,
            || format!("{path}.email[{index}]@preferredcontact"),
            email.span,
            preferred,
            BOOL_FLAG,
        );
    }

    for (index, phone) in contact.phones.iter().enumerate() {
        let phone_path = format!("{path}.phone[{index}]");
        check_phone_attributes(report, || phone_path, phone);
    }

    for (index, address) in contact.addresses.iter().enumerate() {
        let address_path = format!("{path}.address[{index}]");
        check_enum(
            report,
            || format!("{address_path}@type"),
            address.span,
            address.address_type.as_deref(),
            ADDRESS_TYPE,
        );
        if let Some(country) = &address.country {
            check_iso_country(
                report,
                || format!("{address_path}.country"),
                country.span,
                &country.value(),
            );
        }
    }
}

fn validate_vehicle(
    report: &mut ValidationReport<'_>,
    path: &str,
    vehicle: &Vehicle<'_>,
    options: ValidationOptions,
) {
    report.required(
        options,
        path,
        vehicle.span,
        vehicle.year.is_some(),
        "vehicle is missing year",
    );
    report.required(
        options,
        path,
        vehicle.span,
        vehicle.make.is_some(),
        "vehicle is missing make",
    );
    report.required(
        options,
        path,
        vehicle.span,
        vehicle.model.is_some(),
        "vehicle is missing model",
    );

    check_enum(
        report,
        || format!("{path}@interest"),
        vehicle.span,
        vehicle.interest.as_deref(),
        VEHICLE_INTEREST,
    );
    check_enum(
        report,
        || format!("{path}@status"),
        vehicle.span,
        vehicle.status.as_deref(),
        VEHICLE_STATUS,
    );

    if let Some(odometer) = &vehicle.odometer {
        check_enum(
            report,
            || format!("{path}.odometer@status"),
            odometer.span,
            attr_value(&odometer.attributes, "status"),
            ODOMETER_STATUS,
        );
        check_enum(
            report,
            || format!("{path}.odometer@units"),
            odometer.span,
            attr_value(&odometer.attributes, "units"),
            ODOMETER_UNITS,
        );
    }

    if let Some(condition) = &vehicle.condition {
        let value = condition.value();
        let trimmed = value.trim();
        if !trimmed.is_empty() && !CONDITION_VALUES.contains(&trimmed) {
            report.rule(
                ValidationCode::InvalidEnum,
                format!("{path}.condition"),
                condition.span,
                format!("invalid condition value {trimmed:?}"),
            );
        }
    }

    for (index, price) in vehicle.prices.iter().enumerate() {
        validate_price(report, &format!("{path}.price[{index}]"), price);
    }

    for (index, option) in vehicle.options.iter().enumerate() {
        for (price_index, price) in option.prices.iter().enumerate() {
            validate_price(
                report,
                &format!("{path}.option[{index}].price[{price_index}]"),
                price,
            );
        }
    }

    if let Some(finance) = &vehicle.finance {
        validate_finance(report, &format!("{path}.finance"), finance);
    }
}

fn validate_price(report: &mut ValidationReport<'_>, path: &str, price: &Price<'_>) {
    check_enum(
        report,
        || format!("{path}@type"),
        price.span,
        price.price_type.as_deref(),
        PRICE_TYPE,
    );
    check_enum(
        report,
        || format!("{path}@delta"),
        price.span,
        price.delta.as_deref(),
        PRICE_DELTA,
    );
    check_enum(
        report,
        || format!("{path}@relativeto"),
        price.span,
        price.relative_to.as_deref(),
        PRICE_RELATIVE_TO,
    );
    if let Some(currency) = price.currency.as_deref() {
        check_iso_currency(report, || format!("{path}@currency"), price.span, currency);
    }
}

fn validate_finance(report: &mut ValidationReport<'_>, path: &str, finance: &Finance<'_>) {
    if let Some(method) = &finance.method {
        let value = method.value();
        let trimmed = value.trim();
        if !trimmed.is_empty() && !FINANCE_METHOD.contains(&trimmed) {
            report.rule(
                ValidationCode::InvalidEnum,
                format!("{path}.method"),
                method.span,
                format!("invalid finance method {trimmed:?}"),
            );
        }
    }

    for (index, amount) in finance.amounts.iter().enumerate() {
        let amount_path = format!("{path}.amount[{index}]");
        validate_amount(report, &amount_path, amount, AMOUNT_TYPE);
        check_enum(
            report,
            || format!("{amount_path}@limit"),
            amount.span,
            attr_value(&amount.attributes, "limit"),
            AMOUNT_LIMIT,
        );
    }

    for (index, balance) in finance.balances.iter().enumerate() {
        let balance_path = format!("{path}.balance[{index}]");
        validate_amount(report, &balance_path, balance, BALANCE_TYPE);
    }
}

fn validate_amount(
    report: &mut ValidationReport<'_>,
    path: &str,
    amount: &TextElement<'_>,
    type_values: AllowedValues,
) {
    check_enum(
        report,
        || format!("{path}@type"),
        amount.span,
        attr_value(&amount.attributes, "type"),
        type_values,
    );
    if let Some(currency) = attr_value(&amount.attributes, "currency") {
        check_iso_currency(report, || format!("{path}@currency"), amount.span, currency);
    }
}

fn validate_conformance(report: &mut ValidationReport<'_>, adf: &Adf<'_>) {
    check_attributes(report, "adf", adf.span, &adf.attributes, &[]);
    check_extensions(report, "adf", adf.span, &adf.extensions, &["prospect"]);
    let mut order = Vec::new();
    for (index, prospect) in adf.prospects.iter().enumerate() {
        order.push((prospect.span, 0, format!("adf.prospect[{index}]")));
        conform_prospect(report, &format!("adf.prospect[{index}]"), prospect);
    }
    check_order(report, order);
}

fn conform_prospect(report: &mut ValidationReport<'_>, path: &str, value: &Prospect<'_>) {
    check_attributes(report, path, value.span, &value.attributes, &["status"]);
    check_extensions(
        report,
        path,
        value.span,
        &value.extensions,
        &[
            "id",
            "requestdate",
            "vehicle",
            "customer",
            "vendor",
            "provider",
        ],
    );
    for (index, id) in value.ids.iter().enumerate() {
        conform_id(report, &format!("{path}.id[{index}]"), id);
    }
    if let Some(date) = &value.request_date {
        check_attributes(
            report,
            &format!("{path}.requestdate"),
            date.span,
            &date.attributes,
            &[],
        );
        check_text_parts(report, &format!("{path}.requestdate"), date);
        check_adf_datetime(
            report,
            &format!("{path}.requestdate"),
            date.span,
            &date.value(),
        );
    }
    let mut order = Vec::new();
    for (index, id) in value.ids.iter().enumerate() {
        order.push((id.span, 0, format!("{path}.id[{index}]")));
    }
    push_text_order(
        &mut order,
        &value.request_date,
        1,
        format!("{path}.requestdate"),
    );
    for (index, vehicle) in value.vehicles.iter().enumerate() {
        order.push((vehicle.span, 2, format!("{path}.vehicle[{index}]")));
        conform_vehicle(report, &format!("{path}.vehicle[{index}]"), vehicle);
    }
    if let Some(customer) = &value.customer {
        order.push((customer.span, 3, format!("{path}.customer")));
        conform_customer(report, &format!("{path}.customer"), customer);
    }
    if let Some(vendor) = &value.vendor {
        order.push((vendor.span, 4, format!("{path}.vendor")));
        conform_vendor(report, &format!("{path}.vendor"), vendor);
    }
    if let Some(provider) = &value.provider {
        order.push((provider.span, 5, format!("{path}.provider")));
        conform_provider(report, &format!("{path}.provider"), provider);
    }
    check_order(report, order);
}

fn conform_vehicle(report: &mut ValidationReport<'_>, path: &str, value: &Vehicle<'_>) {
    check_attributes(
        report,
        path,
        value.span,
        &value.attributes,
        &["interest", "status"],
    );
    check_extensions(
        report,
        path,
        value.span,
        &value.extensions,
        &[
            "id",
            "year",
            "make",
            "model",
            "vin",
            "stock",
            "trim",
            "doors",
            "bodystyle",
            "transmission",
            "odometer",
            "condition",
            "colorcombination",
            "imagetag",
            "price",
            "pricecomments",
            "option",
            "finance",
            "comments",
        ],
    );
    check_max(
        report,
        &format!("{path}.imagetag"),
        value.span,
        value.image_tags.len(),
        1,
    );
    for (name, field) in [
        ("year", &value.year),
        ("make", &value.make),
        ("model", &value.model),
        ("vin", &value.vin),
        ("stock", &value.stock),
        ("trim", &value.trim),
        ("doors", &value.doors),
        ("bodystyle", &value.body_style),
        ("transmission", &value.transmission),
        ("condition", &value.condition),
        ("pricecomments", &value.price_comments),
        ("comments", &value.comments),
    ] {
        check_plain_text(report, &format!("{path}.{name}"), field);
    }
    check_max(
        report,
        &format!("{path}.price"),
        value.span,
        value.prices.len(),
        1,
    );
    for (index, id) in value.ids.iter().enumerate() {
        conform_id(report, &format!("{path}.id[{index}]"), id);
    }
    if let Some(odometer) = &value.odometer {
        check_attributes(
            report,
            &format!("{path}.odometer"),
            odometer.span,
            &odometer.attributes,
            &["status", "units"],
        );
        check_text_parts(report, &format!("{path}.odometer"), odometer);
    }
    for (index, colors) in value.color_combinations.iter().enumerate() {
        conform_colors(report, &format!("{path}.colorcombination[{index}]"), colors);
    }
    for (index, image) in value.image_tags.iter().enumerate() {
        check_attributes(
            report,
            &format!("{path}.imagetag[{index}]"),
            image.span,
            &image.attributes,
            &["width", "height", "alttext"],
        );
        check_text_parts(report, &format!("{path}.imagetag[{index}]"), image);
    }
    for (index, price) in value.prices.iter().enumerate() {
        conform_price(report, &format!("{path}.price[{index}]"), price);
    }
    for (index, option) in value.options.iter().enumerate() {
        conform_option(report, &format!("{path}.option[{index}]"), option);
    }
    if let Some(finance) = &value.finance {
        conform_finance(report, &format!("{path}.finance"), finance);
    }
    let mut order = Vec::new();
    for (index, id) in value.ids.iter().enumerate() {
        order.push((id.span, 0, format!("{path}.id[{index}]")));
    }
    let fields = [
        (&value.year, 1, "year"),
        (&value.make, 2, "make"),
        (&value.model, 3, "model"),
        (&value.vin, 4, "vin"),
        (&value.stock, 5, "stock"),
        (&value.trim, 6, "trim"),
        (&value.doors, 7, "doors"),
        (&value.body_style, 8, "bodystyle"),
        (&value.transmission, 9, "transmission"),
        (&value.odometer, 10, "odometer"),
        (&value.condition, 11, "condition"),
    ];
    for (field, rank, name) in fields {
        push_text_order(&mut order, field, rank, format!("{path}.{name}"));
    }
    for (index, item) in value.color_combinations.iter().enumerate() {
        order.push((item.span, 12, format!("{path}.colorcombination[{index}]")));
    }
    for (index, item) in value.image_tags.iter().enumerate() {
        order.push((item.span, 13, format!("{path}.imagetag[{index}]")));
    }
    for (index, item) in value.prices.iter().enumerate() {
        order.push((item.span, 14, format!("{path}.price[{index}]")));
    }
    push_text_order(
        &mut order,
        &value.price_comments,
        15,
        format!("{path}.pricecomments"),
    );
    for (index, item) in value.options.iter().enumerate() {
        order.push((item.span, 16, format!("{path}.option[{index}]")));
    }
    if let Some(item) = &value.finance {
        order.push((item.span, 17, format!("{path}.finance")));
    }
    push_text_order(&mut order, &value.comments, 18, format!("{path}.comments"));
    check_order(report, order);
}

fn conform_colors(report: &mut ValidationReport<'_>, path: &str, value: &ColorCombination<'_>) {
    check_attributes(report, path, value.span, &value.attributes, &[]);
    check_extensions(
        report,
        path,
        value.span,
        &value.extensions,
        &["interiorcolor", "exteriorcolor", "preference"],
    );
    required_rule(
        report,
        path,
        value.span,
        value.interior_color.is_some() || value.exterior_color.is_some(),
        "colorcombination requires an interiorcolor or exteriorcolor",
    );
    required_rule(
        report,
        path,
        value.span,
        value.preference.is_some(),
        "colorcombination requires preference",
    );
    if let Some(preference) = &value.preference {
        check_positive_integer(
            report,
            &format!("{path}.preference"),
            preference.span,
            &preference.value(),
        );
    }
    let mut order = Vec::new();
    for (field, rank, name) in [
        (&value.interior_color, 0, "interiorcolor"),
        (&value.exterior_color, 1, "exteriorcolor"),
        (&value.preference, 2, "preference"),
    ] {
        check_plain_text(report, &format!("{path}.{name}"), field);
        push_text_order(&mut order, field, rank, format!("{path}.{name}"));
    }
    check_order(report, order);
}

fn conform_option(report: &mut ValidationReport<'_>, path: &str, value: &VehicleOption<'_>) {
    check_attributes(report, path, value.span, &value.attributes, &[]);
    check_extensions(
        report,
        path,
        value.span,
        &value.extensions,
        &[
            "optionname",
            "manufacturercode",
            "stock",
            "weighting",
            "price",
        ],
    );
    required_rule(
        report,
        path,
        value.span,
        value.option_name.is_some(),
        "option requires optionname",
    );
    required_rule(
        report,
        path,
        value.span,
        value.weighting.is_some(),
        "option requires weighting",
    );
    check_max(
        report,
        &format!("{path}.price"),
        value.span,
        value.prices.len(),
        1,
    );
    if let Some(weighting) = &value.weighting {
        check_integer_range(
            report,
            &format!("{path}.weighting"),
            weighting.span,
            &weighting.value(),
            -100,
            100,
        );
    }
    for (index, price) in value.prices.iter().enumerate() {
        conform_price(report, &format!("{path}.price[{index}]"), price);
    }
    let mut order = Vec::new();
    for (field, rank, name) in [
        (&value.option_name, 0, "optionname"),
        (&value.manufacturer_code, 1, "manufacturercode"),
        (&value.stock, 2, "stock"),
        (&value.weighting, 3, "weighting"),
    ] {
        check_plain_text(report, &format!("{path}.{name}"), field);
        push_text_order(&mut order, field, rank, format!("{path}.{name}"));
    }
    for (index, price) in value.prices.iter().enumerate() {
        order.push((price.span, 4, format!("{path}.price[{index}]")));
    }
    check_order(report, order);
}

fn conform_finance(report: &mut ValidationReport<'_>, path: &str, value: &Finance<'_>) {
    check_attributes(report, path, value.span, &value.attributes, &[]);
    check_extensions(
        report,
        path,
        value.span,
        &value.extensions,
        &["method", "amount", "balance"],
    );
    required_rule(
        report,
        path,
        value.span,
        value.method.is_some(),
        "finance requires method",
    );
    required_rule(
        report,
        path,
        value.span,
        !value.amounts.is_empty(),
        "finance requires at least one amount",
    );
    check_max(
        report,
        &format!("{path}.balance"),
        value.span,
        value.balances.len(),
        1,
    );
    check_plain_text(report, &format!("{path}.method"), &value.method);
    for (index, amount) in value.amounts.iter().enumerate() {
        check_attributes(
            report,
            &format!("{path}.amount[{index}]"),
            amount.span,
            &amount.attributes,
            &["type", "limit", "currency"],
        );
        check_text_parts(report, &format!("{path}.amount[{index}]"), amount);
        check_enum(
            report,
            || format!("{path}.amount[{index}]@type"),
            amount.span,
            attr_value(&amount.attributes, "type"),
            AllowedValues {
                values: &["downpayment", "monthly", "total"],
                display: "downpayment, monthly, total",
            },
        );
        if let Some(currency) = attr_value(&amount.attributes, "currency") {
            check_iso_currency_membership(
                report,
                &format!("{path}.amount[{index}]@currency"),
                amount.span,
                currency,
            );
        }
    }
    for (index, balance) in value.balances.iter().enumerate() {
        check_attributes(
            report,
            &format!("{path}.balance[{index}]"),
            balance.span,
            &balance.attributes,
            &["type", "currency"],
        );
        check_text_parts(report, &format!("{path}.balance[{index}]"), balance);
        check_enum(
            report,
            || format!("{path}.balance[{index}]@type"),
            balance.span,
            attr_value(&balance.attributes, "type"),
            AllowedValues {
                values: &["finance", "residual"],
                display: "finance, residual",
            },
        );
        if let Some(currency) = attr_value(&balance.attributes, "currency") {
            check_iso_currency_membership(
                report,
                &format!("{path}.balance[{index}]@currency"),
                balance.span,
                currency,
            );
        }
    }
    let mut order = Vec::new();
    push_text_order(&mut order, &value.method, 0, format!("{path}.method"));
    for (index, amount) in value.amounts.iter().enumerate() {
        order.push((amount.span, 1, format!("{path}.amount[{index}]")));
    }
    for (index, balance) in value.balances.iter().enumerate() {
        order.push((balance.span, 2, format!("{path}.balance[{index}]")));
    }
    check_order(report, order);
}

fn conform_customer(report: &mut ValidationReport<'_>, path: &str, value: &Customer<'_>) {
    check_attributes(report, path, value.span, &value.attributes, &[]);
    check_extensions(
        report,
        path,
        value.span,
        &value.extensions,
        &["contact", "id", "timeframe", "comments"],
    );
    check_exact(
        report,
        &format!("{path}.contact"),
        value.span,
        value.contacts.len(),
        1,
    );
    for (index, contact) in value.contacts.iter().enumerate() {
        conform_contact(report, &format!("{path}.contact[{index}]"), contact);
    }
    for (index, id) in value.ids.iter().enumerate() {
        conform_id(report, &format!("{path}.id[{index}]"), id);
    }
    if let Some(timeframe) = &value.timeframe {
        check_attributes(
            report,
            &format!("{path}.timeframe"),
            timeframe.span,
            &timeframe.attributes,
            &[],
        );
        check_extensions(
            report,
            &format!("{path}.timeframe"),
            timeframe.span,
            &timeframe.extensions,
            &["description", "earliestdate", "latestdate"],
        );
        required_rule(
            report,
            &format!("{path}.timeframe"),
            timeframe.span,
            timeframe.earliest_date.is_some() || timeframe.latest_date.is_some(),
            "timeframe requires earliestdate or latestdate",
        );
        if let Some(date) = &timeframe.earliest_date {
            check_adf_datetime(
                report,
                &format!("{path}.timeframe.earliestdate"),
                date.span,
                &date.value(),
            );
        }
        if let Some(date) = &timeframe.latest_date {
            check_adf_datetime(
                report,
                &format!("{path}.timeframe.latestdate"),
                date.span,
                &date.value(),
            );
        }
        let mut timeframe_order = Vec::new();
        for (field, rank, name) in [
            (&timeframe.description, 0, "description"),
            (&timeframe.earliest_date, 1, "earliestdate"),
            (&timeframe.latest_date, 2, "latestdate"),
        ] {
            check_plain_text(report, &format!("{path}.timeframe.{name}"), field);
            push_text_order(
                &mut timeframe_order,
                field,
                rank,
                format!("{path}.timeframe.{name}"),
            );
        }
        check_order(report, timeframe_order);
    }
    check_plain_text(report, &format!("{path}.comments"), &value.comments);
    let mut order = Vec::new();
    for (index, contact) in value.contacts.iter().enumerate() {
        order.push((contact.span, 0, format!("{path}.contact[{index}]")));
    }
    for (index, id) in value.ids.iter().enumerate() {
        order.push((id.span, 1, format!("{path}.id[{index}]")));
    }
    if let Some(item) = &value.timeframe {
        order.push((item.span, 2, format!("{path}.timeframe")));
    }
    push_text_order(&mut order, &value.comments, 3, format!("{path}.comments"));
    check_order(report, order);
}

fn conform_vendor(report: &mut ValidationReport<'_>, path: &str, value: &Vendor<'_>) {
    check_attributes(report, path, value.span, &value.attributes, &[]);
    check_extensions(
        report,
        path,
        value.span,
        &value.extensions,
        &["id", "vendorname", "url", "contact"],
    );
    check_exact(
        report,
        &format!("{path}.contact"),
        value.span,
        value.contacts.len(),
        1,
    );
    for (index, id) in value.ids.iter().enumerate() {
        conform_id(report, &format!("{path}.id[{index}]"), id);
    }
    for (index, contact) in value.contacts.iter().enumerate() {
        conform_contact(report, &format!("{path}.contact[{index}]"), contact);
    }
    check_plain_text(report, &format!("{path}.vendorname"), &value.vendor_name);
    check_plain_text(report, &format!("{path}.url"), &value.url);
    let mut order = Vec::new();
    for (index, id) in value.ids.iter().enumerate() {
        order.push((id.span, 0, format!("{path}.id[{index}]")));
    }
    push_text_order(
        &mut order,
        &value.vendor_name,
        1,
        format!("{path}.vendorname"),
    );
    push_text_order(&mut order, &value.url, 2, format!("{path}.url"));
    for (index, item) in value.contacts.iter().enumerate() {
        order.push((item.span, 3, format!("{path}.contact[{index}]")));
    }
    check_order(report, order);
}

fn conform_provider(report: &mut ValidationReport<'_>, path: &str, value: &Provider<'_>) {
    check_attributes(report, path, value.span, &value.attributes, &[]);
    check_extensions(
        report,
        path,
        value.span,
        &value.extensions,
        &["id", "name", "service", "url", "email", "phone", "contact"],
    );
    required_rule(
        report,
        path,
        value.span,
        value.name.is_some(),
        "provider requires name",
    );
    check_max(
        report,
        &format!("{path}.contact"),
        value.span,
        value.contacts.len(),
        1,
    );
    for (index, id) in value.ids.iter().enumerate() {
        conform_id(report, &format!("{path}.id[{index}]"), id);
    }
    if let Some(name) = &value.name {
        check_attributes(
            report,
            &format!("{path}.name"),
            name.span,
            &name.attributes,
            &["part", "type"],
        );
        check_text_parts_raw(report, &format!("{path}.name"), name.span, &name.parts);
    }
    if let Some(email) = &value.email {
        check_attributes(
            report,
            &format!("{path}.email"),
            email.span,
            &email.attributes,
            &["preferredcontact"],
        );
        check_text_parts(report, &format!("{path}.email"), email);
    }
    if let Some(phone) = &value.phone {
        check_attributes(
            report,
            &format!("{path}.phone"),
            phone.span,
            &phone.attributes,
            &["type", "time", "preferredcontact"],
        );
        check_text_parts(report, &format!("{path}.phone"), phone);
    }
    for (index, contact) in value.contacts.iter().enumerate() {
        conform_contact(report, &format!("{path}.contact[{index}]"), contact);
    }
    for (name, field) in [("service", &value.service), ("url", &value.url)] {
        check_plain_text(report, &format!("{path}.{name}"), field);
    }
    let mut order = Vec::new();
    for (index, id) in value.ids.iter().enumerate() {
        order.push((id.span, 0, format!("{path}.id[{index}]")));
    }
    if let Some(name) = &value.name {
        order.push((name.span, 1, format!("{path}.name")));
    }
    push_text_order(&mut order, &value.service, 2, format!("{path}.service"));
    push_text_order(&mut order, &value.url, 3, format!("{path}.url"));
    push_text_order(&mut order, &value.email, 4, format!("{path}.email"));
    push_text_order(&mut order, &value.phone, 5, format!("{path}.phone"));
    for (index, contact) in value.contacts.iter().enumerate() {
        order.push((contact.span, 6, format!("{path}.contact[{index}]")));
    }
    check_order(report, order);
}

fn conform_contact(report: &mut ValidationReport<'_>, path: &str, value: &Contact<'_>) {
    check_attributes(
        report,
        path,
        value.span,
        &value.attributes,
        &["primarycontact"],
    );
    check_extensions(
        report,
        path,
        value.span,
        &value.extensions,
        &["name", "email", "phone", "address"],
    );
    required_rule(
        report,
        path,
        value.span,
        !value.names.is_empty(),
        "contact requires at least one name",
    );
    required_rule(
        report,
        path,
        value.span,
        !value.emails.is_empty() || !value.phones.is_empty(),
        "contact requires email or phone",
    );
    check_max(
        report,
        &format!("{path}.address"),
        value.span,
        value.addresses.len(),
        1,
    );
    check_max(
        report,
        &format!("{path}.email"),
        value.span,
        value.emails.len(),
        1,
    );
    for (index, name) in value.names.iter().enumerate() {
        check_attributes(
            report,
            &format!("{path}.name[{index}]"),
            name.span,
            &name.attributes,
            &["part", "type"],
        );
        check_text_parts_raw(
            report,
            &format!("{path}.name[{index}]"),
            name.span,
            &name.parts,
        );
    }
    for (index, email) in value.emails.iter().enumerate() {
        check_attributes(
            report,
            &format!("{path}.email[{index}]"),
            email.span,
            &email.attributes,
            &["preferredcontact"],
        );
        check_text_parts(report, &format!("{path}.email[{index}]"), email);
    }
    for (index, phone) in value.phones.iter().enumerate() {
        check_attributes(
            report,
            &format!("{path}.phone[{index}]"),
            phone.span,
            &phone.attributes,
            &["type", "time", "preferredcontact"],
        );
        check_text_parts(report, &format!("{path}.phone[{index}]"), phone);
    }
    for (index, address) in value.addresses.iter().enumerate() {
        conform_address(report, &format!("{path}.address[{index}]"), address);
    }
    let mut order = Vec::new();
    for (index, item) in value.names.iter().enumerate() {
        order.push((item.span, 0, format!("{path}.name[{index}]")));
    }
    for (index, item) in value.emails.iter().enumerate() {
        order.push((item.span, 1, format!("{path}.email[{index}]")));
    }
    for (index, item) in value.phones.iter().enumerate() {
        order.push((item.span, 2, format!("{path}.phone[{index}]")));
    }
    for (index, item) in value.addresses.iter().enumerate() {
        order.push((item.span, 3, format!("{path}.address[{index}]")));
    }
    check_order(report, order);
}

fn conform_address(report: &mut ValidationReport<'_>, path: &str, value: &Address<'_>) {
    check_attributes(report, path, value.span, &value.attributes, &["type"]);
    check_extensions(
        report,
        path,
        value.span,
        &value.extensions,
        &[
            "street",
            "apartment",
            "city",
            "regioncode",
            "postalcode",
            "country",
        ],
    );
    required_rule(
        report,
        path,
        value.span,
        !value.streets.is_empty(),
        "address requires at least one street",
    );
    check_max(
        report,
        &format!("{path}.street"),
        value.span,
        value.streets.len(),
        5,
    );
    for (index, street) in value.streets.iter().enumerate() {
        check_attributes(
            report,
            &format!("{path}.street[{index}]"),
            street.span,
            &street.attributes,
            &["line"],
        );
        check_text_parts(report, &format!("{path}.street[{index}]"), street);
        if let Some(line) = attr_value(&street.attributes, "line") {
            check_integer_range(
                report,
                &format!("{path}.street[{index}]@line"),
                street.span,
                line,
                1,
                5,
            );
        }
    }
    if let Some(country) = &value.country {
        check_iso_country_membership(
            report,
            &format!("{path}.country"),
            country.span,
            &country.value(),
        );
    }
    for (name, field) in [
        ("apartment", &value.apartment),
        ("city", &value.city),
        ("regioncode", &value.region_code),
        ("postalcode", &value.postal_code),
        ("country", &value.country),
    ] {
        check_plain_text(report, &format!("{path}.{name}"), field);
    }
    let mut order = Vec::new();
    for (index, street) in value.streets.iter().enumerate() {
        order.push((street.span, 0, format!("{path}.street[{index}]")));
    }
    for (field, rank, name) in [
        (&value.apartment, 1, "apartment"),
        (&value.city, 2, "city"),
        (&value.region_code, 3, "regioncode"),
        (&value.postal_code, 4, "postalcode"),
        (&value.country, 5, "country"),
    ] {
        push_text_order(&mut order, field, rank, format!("{path}.{name}"));
    }
    check_order(report, order);
}

fn conform_id(report: &mut ValidationReport<'_>, path: &str, value: &Id<'_>) {
    check_attributes(
        report,
        path,
        value.span,
        &value.attributes,
        &["sequence", "source"],
    );
    check_text_parts_raw(report, path, value.span, &value.parts);
    required_rule(
        report,
        path,
        value.span,
        value.source.is_some(),
        "id requires source",
    );
    if let Some(sequence) = value.sequence.as_deref() {
        check_positive_integer(report, &format!("{path}@sequence"), value.span, sequence);
    }
}

fn conform_price(report: &mut ValidationReport<'_>, path: &str, value: &Price<'_>) {
    check_attributes(
        report,
        path,
        value.span,
        &value.attributes,
        &["type", "currency", "delta", "relativeto", "source"],
    );
    check_text_parts_raw(report, path, value.span, &value.parts);
    if let Some(currency) = value.currency.as_deref() {
        check_iso_currency_membership(report, &format!("{path}@currency"), value.span, currency);
    }
}

fn required_rule(
    report: &mut ValidationReport<'_>,
    path: &str,
    span: Span,
    present: bool,
    message: &'static str,
) {
    if !present {
        report.error(
            ValidationCode::MissingRequired,
            path.to_owned(),
            span,
            message,
        );
    }
}

fn check_exact(
    report: &mut ValidationReport<'_>,
    path: &str,
    span: Span,
    actual: usize,
    expected: usize,
) {
    if actual < expected {
        report.error(
            ValidationCode::MissingRequired,
            path.to_owned(),
            span,
            format!("expected {expected}, found {actual}"),
        );
    }
    if actual > expected {
        report.error(
            ValidationCode::Excessive,
            path.to_owned(),
            span,
            format!("expected {expected}, found {actual}"),
        );
    }
}

fn check_max(
    report: &mut ValidationReport<'_>,
    path: &str,
    span: Span,
    actual: usize,
    maximum: usize,
) {
    if actual > maximum {
        report.error(
            ValidationCode::Excessive,
            path.to_owned(),
            span,
            format!("expected at most {maximum}, found {actual}"),
        );
    }
}

fn check_attributes(
    report: &mut ValidationReport<'_>,
    path: &str,
    span: Span,
    attributes: &[Attribute<'_>],
    allowed: &[&str],
) {
    let mut seen = std::collections::HashSet::new();
    for attribute in attributes {
        if !seen.insert(attribute.name.as_ref()) {
            report.error(
                ValidationCode::Duplicate,
                format!("{path}@{}", attribute.name),
                span,
                "duplicate attribute",
            );
        }
        if !allowed.contains(&attribute.name.as_ref())
            && (report.profile.rejects_extensions()
                || STANDARD_ATTRIBUTES.contains(&attribute.name.as_ref()))
        {
            report.error(
                ValidationCode::UnexpectedAttribute,
                format!("{path}@{}", attribute.name),
                span,
                "attribute is not defined by ADF 1.0",
            );
        }
    }
}

fn check_plain_text(
    report: &mut ValidationReport<'_>,
    path: &str,
    value: &Option<TextElement<'_>>,
) {
    if let Some(value) = value {
        check_attributes(report, path, value.span, &value.attributes, &[]);
        check_text_parts(report, path, value);
    }
}

fn check_text_parts(report: &mut ValidationReport<'_>, path: &str, value: &TextElement<'_>) {
    check_text_parts_raw(report, path, value.span, &value.parts);
}

fn check_text_parts_raw(
    report: &mut ValidationReport<'_>,
    path: &str,
    span: Span,
    parts: &[TextPart<'_>],
) {
    for part in parts {
        if let TextPart::Node(node) = part {
            check_extensions(report, path, span, std::slice::from_ref(node), &[]);
        }
    }
}

fn check_extensions(
    report: &mut ValidationReport<'_>,
    path: &str,
    span: Span,
    extensions: &[XmlNode<'_>],
    known: &[&str],
) {
    for extension in extensions {
        match extension {
            XmlNode::Element(element) if known.contains(&element.name.as_ref()) => report.error(
                ValidationCode::Duplicate,
                format!("{path}.{}", element.name),
                element.span,
                "duplicate singular ADF element",
            ),
            XmlNode::Element(element)
                if report.profile.rejects_extensions()
                    || STANDARD_ELEMENTS.contains(&element.name.as_ref()) =>
            {
                report.error(
                    ValidationCode::UnexpectedElement,
                    format!("{path}.{}", element.name),
                    element.span,
                    "element is not defined at this ADF 1.0 location",
                )
            }
            XmlNode::Text(text) | XmlNode::CData(text)
                if report.profile.rejects_extensions() && !text.trim().is_empty() =>
            {
                report.error(
                    ValidationCode::UnexpectedElement,
                    path.to_owned(),
                    span,
                    "non-whitespace text is not allowed in this container",
                )
            }
            XmlNode::EntityRef(_) if report.profile.rejects_extensions() => report.error(
                ValidationCode::UnexpectedElement,
                path.to_owned(),
                span,
                "entity text is not allowed in this container",
            ),
            _ => {}
        }
    }
}

const STANDARD_ATTRIBUTES: &[&str] = &[
    "status",
    "interest",
    "units",
    "width",
    "height",
    "alttext",
    "type",
    "limit",
    "currency",
    "primarycontact",
    "part",
    "preferredcontact",
    "time",
    "line",
    "sequence",
    "source",
    "delta",
    "relativeto",
];

const STANDARD_ELEMENTS: &[&str] = &[
    "adf",
    "prospect",
    "requestdate",
    "vehicle",
    "year",
    "make",
    "model",
    "vin",
    "stock",
    "trim",
    "doors",
    "bodystyle",
    "transmission",
    "odometer",
    "condition",
    "colorcombination",
    "interiorcolor",
    "exteriorcolor",
    "preference",
    "imagetag",
    "price",
    "pricecomments",
    "option",
    "optionname",
    "manufacturercode",
    "weighting",
    "finance",
    "method",
    "amount",
    "balance",
    "customer",
    "timeframe",
    "description",
    "earliestdate",
    "latestdate",
    "vendor",
    "vendorname",
    "provider",
    "service",
    "contact",
    "name",
    "email",
    "phone",
    "address",
    "street",
    "apartment",
    "city",
    "regioncode",
    "postalcode",
    "country",
    "comments",
    "url",
    "id",
];

fn push_text_order(
    order: &mut Vec<(Span, usize, String)>,
    value: &Option<TextElement<'_>>,
    rank: usize,
    path: String,
) {
    if let Some(value) = value {
        order.push((value.span, rank, path));
    }
}

fn check_order(report: &mut ValidationReport<'_>, mut values: Vec<(Span, usize, String)>) {
    values.retain(|(span, _, _)| *span != Span::default());
    values.sort_by_key(|(span, _, _)| span.start);
    let mut highest = 0;
    for (span, rank, path) in values {
        if rank < highest {
            report.error(
                ValidationCode::OutOfOrder,
                path,
                span,
                "ADF element is out of specification order",
            );
        }
        highest = highest.max(rank);
    }
}

fn check_positive_integer(report: &mut ValidationReport<'_>, path: &str, span: Span, value: &str) {
    match value.trim().parse::<u64>() {
        Ok(number) if number > 0 => {}
        _ => report.error(
            ValidationCode::InvalidRange,
            path.to_owned(),
            span,
            "value must be a positive integer",
        ),
    }
}

fn check_integer_range(
    report: &mut ValidationReport<'_>,
    path: &str,
    span: Span,
    value: &str,
    minimum: i64,
    maximum: i64,
) {
    match value.trim().parse::<i64>() {
        Ok(number) if (minimum..=maximum).contains(&number) => {}
        _ => report.error(
            ValidationCode::InvalidRange,
            path.to_owned(),
            span,
            format!("value must be an integer from {minimum} through {maximum}"),
        ),
    }
}

// Snapshots of actively assigned codes checked on 2026-07-15. Keeping the
// registries as data files makes updates reviewable independently of logic.
const ISO_COUNTRIES: &str = include_str!("data/iso_3166_1_alpha2_2026-07-15.txt");
const ISO_CURRENCIES: &str = include_str!("data/iso_4217_2026-07-15.txt");

fn code_in_registry(registry: &str, value: &str) -> bool {
    registry.split_ascii_whitespace().any(|code| code == value)
}

fn check_iso_country_membership(
    report: &mut ValidationReport<'_>,
    path: &str,
    span: Span,
    value: &str,
) {
    let value = value.trim();
    if !code_in_registry(ISO_COUNTRIES, value) {
        report.error(
            ValidationCode::InvalidFormat,
            path.to_owned(),
            span,
            format!("{value:?} is not an active ISO 3166-1 alpha-2 code"),
        );
    }
}

fn check_iso_currency_membership(
    report: &mut ValidationReport<'_>,
    path: &str,
    span: Span,
    value: &str,
) {
    if !code_in_registry(ISO_CURRENCIES, value) {
        report.error(
            ValidationCode::InvalidFormat,
            path.to_owned(),
            span,
            format!("{value:?} is not an active ISO 4217 code"),
        );
    }
}

fn attr_value<'a>(attributes: &'a [Attribute<'a>], name: &str) -> Option<&'a str> {
    attributes
        .iter()
        .find(|attr| attr.name.as_ref() == name)
        .map(|attr| attr.value.as_ref())
}

fn check_phone_attributes(
    report: &mut ValidationReport<'_>,
    path: impl FnOnce() -> String,
    phone: &TextElement<'_>,
) {
    let path = path();
    check_enum(
        report,
        || format!("{path}@type"),
        phone.span,
        attr_value(&phone.attributes, "type"),
        PHONE_TYPE,
    );
    check_enum(
        report,
        || format!("{path}@time"),
        phone.span,
        attr_value(&phone.attributes, "time"),
        PHONE_TIME,
    );
    check_enum(
        report,
        || format!("{path}@preferredcontact"),
        phone.span,
        attr_value(&phone.attributes, "preferredcontact"),
        BOOL_FLAG,
    );
}

fn check_enum(
    report: &mut ValidationReport<'_>,
    path: impl FnOnce() -> String,
    span: Span,
    value: Option<&str>,
    allowed: AllowedValues,
) {
    let Some(value) = value else { return };
    if allowed.values.contains(&value) {
        return;
    }
    report.rule(
        ValidationCode::InvalidEnum,
        path(),
        span,
        format!("value {value:?} is not one of: {}", allowed.display),
    );
}

fn check_iso_currency(
    report: &mut ValidationReport<'_>,
    path: impl FnOnce() -> String,
    span: Span,
    value: &str,
) {
    if value.len() == 3 && value.chars().all(|ch| ch.is_ascii_uppercase()) {
        return;
    }
    report.rule(
        ValidationCode::InvalidFormat,
        path(),
        span,
        format!("currency {value:?} is not shaped like a 3-letter ISO 4217 code"),
    );
}

fn check_iso_country(
    report: &mut ValidationReport<'_>,
    path: impl FnOnce() -> String,
    span: Span,
    value: &str,
) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return;
    }
    if trimmed.len() == 2 && trimmed.chars().all(|ch| ch.is_ascii_uppercase()) {
        return;
    }
    report.rule(
        ValidationCode::InvalidFormat,
        path(),
        span,
        format!("country {trimmed:?} is not shaped like a 2-letter ISO 3166-1 alpha-2 code"),
    );
}

fn check_iso_datetime(
    report: &mut ValidationReport<'_>,
    path: impl FnOnce() -> String,
    span: Span,
    value: &str,
) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return;
    }
    if is_iso_datetime(trimmed) {
        return;
    }
    report.rule(
        ValidationCode::InvalidFormat,
        path(),
        span,
        format!("date {trimmed:?} is not in the supported ISO 8601 datetime shape"),
    );
}

fn check_adf_datetime(report: &mut ValidationReport<'_>, path: &str, span: Span, value: &str) {
    let value = value.trim();
    let bytes = value.as_bytes();
    let exact_shape = match bytes.len() {
        20 => bytes
            .get(15)
            .is_some_and(|byte| matches!(byte, b'+' | b'-')),
        25 => bytes
            .get(19)
            .is_some_and(|byte| matches!(byte, b'+' | b'-')),
        _ => false,
    };
    if !exact_shape || !is_iso_datetime(value) {
        report.error(
            ValidationCode::InvalidFormat,
            path.to_owned(),
            span,
            "date must use one of the four ADF 1.0 ISO 8601 forms with an offset",
        );
    }
}

fn is_iso_datetime(value: &str) -> bool {
    let bytes = value.as_bytes();
    let len = bytes.len();
    if len < 15 {
        return false;
    }

    // Distinguish extended (with dashes) from basic (without).
    if bytes[4] == b'-' {
        // CCYY-MM-DDThh:mm:ss with optional fractional + offset
        // Minimum length 19; e.g. 2024-01-02T03:04:05
        if len < 19 {
            return false;
        }
        if !ascii_digits(&bytes[0..4])
            || bytes[4] != b'-'
            || !ascii_digits(&bytes[5..7])
            || bytes[7] != b'-'
            || !ascii_digits(&bytes[8..10])
            || bytes[10] != b'T'
            || !ascii_digits(&bytes[11..13])
            || bytes[13] != b':'
            || !ascii_digits(&bytes[14..16])
            || bytes[16] != b':'
            || !ascii_digits(&bytes[17..19])
        {
            return false;
        }
        if !valid_date_time_fields(
            number(&bytes[0..4]),
            number(&bytes[5..7]),
            number(&bytes[8..10]),
            number(&bytes[11..13]),
            number(&bytes[14..16]),
            number(&bytes[17..19]),
        ) {
            return false;
        }
        check_offset(&bytes[19..], true)
    } else {
        // CCYYMMDDThhmmss with optional fractional + offset
        if !ascii_digits(&bytes[0..8]) || bytes[8] != b'T' || !ascii_digits(&bytes[9..15]) {
            return false;
        }
        if !valid_date_time_fields(
            number(&bytes[0..4]),
            number(&bytes[4..6]),
            number(&bytes[6..8]),
            number(&bytes[9..11]),
            number(&bytes[11..13]),
            number(&bytes[13..15]),
        ) {
            return false;
        }
        check_offset(&bytes[15..], false)
    }
}

fn check_offset(rest: &[u8], extended: bool) -> bool {
    let mut cursor = 0;
    // Optional fractional seconds: .ddd...
    if cursor < rest.len() && rest[cursor] == b'.' {
        cursor += 1;
        let start = cursor;
        while cursor < rest.len() && rest[cursor].is_ascii_digit() {
            cursor += 1;
        }
        if cursor == start {
            return false;
        }
    }

    if cursor == rest.len() {
        return true;
    }

    match rest[cursor] {
        b'Z' => cursor + 1 == rest.len(),
        b'+' | b'-' => {
            let remainder = &rest[cursor + 1..];
            if extended {
                remainder.len() == 5
                    && ascii_digits(&remainder[0..2])
                    && remainder[2] == b':'
                    && ascii_digits(&remainder[3..5])
                    && valid_offset(number(&remainder[0..2]), number(&remainder[3..5]))
            } else {
                remainder.len() == 4
                    && ascii_digits(remainder)
                    && valid_offset(number(&remainder[0..2]), number(&remainder[2..4]))
            }
        }
        _ => false,
    }
}

fn ascii_digits(bytes: &[u8]) -> bool {
    !bytes.is_empty() && bytes.iter().all(|b| b.is_ascii_digit())
}

fn number(bytes: &[u8]) -> u32 {
    bytes
        .iter()
        .fold(0_u32, |value, byte| value * 10 + u32::from(byte - b'0'))
}

fn valid_date_time_fields(
    year: u32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
) -> bool {
    (1..=12).contains(&month)
        && (1..=days_in_month(year, month)).contains(&day)
        && hour <= 23
        && minute <= 59
        && second <= 59
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: u32) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

fn valid_offset(hour: u32, minute: u32) -> bool {
    hour <= 23 && minute <= 59
}

impl<'a> ValidationReport<'a> {
    fn error(
        &mut self,
        code: ValidationCode,
        path: impl Into<Cow<'a, str>>,
        span: Span,
        message: impl Into<Cow<'a, str>>,
    ) {
        self.issues.push(ValidationIssue {
            code,
            severity: Severity::Error,
            path: path.into(),
            message: message.into(),
            span: span_option(span),
        });
    }

    fn required(
        &mut self,
        options: ValidationOptions,
        path: &str,
        span: Span,
        present: bool,
        message: &'static str,
    ) {
        if present {
            return;
        }
        if !matches!(options.profile, ValidationProfile::Lenient) {
            self.error(
                ValidationCode::MissingRequired,
                path.to_owned(),
                span,
                message,
            );
        } else {
            self.issues.push(ValidationIssue {
                code: ValidationCode::MissingRequired,
                severity: Severity::Warning,
                path: Cow::Owned(path.to_owned()),
                message: Cow::Borrowed(message),
                span: span_option(span),
            });
        }
    }

    fn rule(
        &mut self,
        code: ValidationCode,
        path: impl Into<Cow<'a, str>>,
        span: Span,
        message: impl Into<Cow<'a, str>>,
    ) {
        if self.profile.is_conformance() {
            self.error(code, path, span, message);
        } else {
            self.issues.push(ValidationIssue {
                code,
                severity: Severity::Warning,
                path: path.into(),
                message: message.into(),
                span: span_option(span),
            });
        }
    }
}

fn span_option(span: Span) -> Option<Span> {
    if span.start == 0 && span.end == 0 {
        None
    } else {
        Some(span)
    }
}
