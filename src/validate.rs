use crate::document::Span;
use crate::model::*;
use crate::{Attribute, TextElement};
use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue<'a> {
    pub severity: Severity,
    pub path: Cow<'a, str>,
    pub message: Cow<'a, str>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ValidationReport<'a> {
    pub issues: Vec<ValidationIssue<'a>>,
}

impl ValidationReport<'_> {
    pub fn is_valid(&self) -> bool {
        !self
            .issues
            .iter()
            .any(|issue| issue.severity == Severity::Error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct ValidationOptions {
    pub strict: bool,
}

impl ValidationOptions {
    /// Promote structural requirement checks from warnings to errors.
    #[must_use]
    pub fn strict(mut self, strict: bool) -> Self {
        self.strict = strict;
        self
    }
}

const PROSPECT_STATUS: &[&str] = &["new", "resend"];
const VEHICLE_INTEREST: &[&str] = &["buy", "lease", "sell", "trade-in", "test-drive"];
const VEHICLE_STATUS: &[&str] = &["new", "used"];
const PRICE_TYPE: &[&str] = &[
    "quote",
    "offer",
    "msrp",
    "invoice",
    "call",
    "appraisal",
    "asking",
];
const PRICE_DELTA: &[&str] = &["absolute", "relative", "percentage"];
const PRICE_RELATIVE_TO: &[&str] = &["msrp", "invoice"];
const NAME_PART: &[&str] = &["surname", "first", "middle", "last", "full"];
const NAME_TYPE: &[&str] = &["business", "individual"];
const BOOL_FLAG: &[&str] = &["0", "1"];
const PHONE_TYPE: &[&str] = &["voice", "fax", "cellphone", "pager"];
const PHONE_TIME: &[&str] = &["morning", "afternoon", "evening", "nopreference", "day"];
const ADDRESS_TYPE: &[&str] = &["work", "home", "delivery"];
const ODOMETER_STATUS: &[&str] = &["unknown", "rolledover", "replaced", "original"];
const ODOMETER_UNITS: &[&str] = &["km", "mi"];
const CONDITION_VALUES: &[&str] = &["excellent", "good", "fair", "poor", "unknown"];
const FINANCE_METHOD: &[&str] = &["cash", "finance", "lease"];
const AMOUNT_TYPE: &[&str] = &[
    "downpayment",
    "tradein",
    "rebate",
    "total",
    "monthly",
    "fee",
    "tax",
    "other",
];
const AMOUNT_LIMIT: &[&str] = &["minimum", "maximum", "exact"];
const BALANCE_TYPE: &[&str] = &["finance", "residual", "payoff", "other"];

pub fn validate<'a>(adf: &Adf<'a>) -> ValidationReport<'a> {
    validate_with(adf, ValidationOptions::default())
}

pub fn validate_with<'a>(adf: &Adf<'a>, options: ValidationOptions) -> ValidationReport<'a> {
    let mut report = ValidationReport::default();

    if adf.prospects.is_empty() {
        report.error(
            "adf",
            adf.span,
            "ADF document should contain at least one prospect",
        );
    }

    for (index, prospect) in adf.prospects.iter().enumerate() {
        let path = format!("adf.prospect[{index}]");
        validate_prospect(&mut report, &path, prospect, options);
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
        format!("{path}@status"),
        prospect.span,
        prospect.status.as_deref(),
        PROSPECT_STATUS,
    );

    if let Some(date) = &prospect.request_date {
        check_iso_datetime(
            report,
            format!("{path}.requestdate"),
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
        validate_provider(report, path, provider);
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
            format!("{customer_path}.timeframe.earliestdate"),
            date.span,
            &date.value(),
        );
    }
    if let Some(date) = &timeframe.latest_date {
        check_iso_datetime(
            report,
            format!("{customer_path}.timeframe.latestdate"),
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
) {
    let provider_path = format!("{prospect_path}.provider");
    if let Some(name) = &provider.name {
        check_enum(
            report,
            format!("{provider_path}.name@part"),
            name.span,
            name.part.as_deref(),
            NAME_PART,
        );
        check_enum(
            report,
            format!("{provider_path}.name@type"),
            name.span,
            name.name_type.as_deref(),
            NAME_TYPE,
        );
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
        report.warn(
            path.to_owned(),
            contact.span,
            "contact should contain email or phone",
        );
    }

    check_enum(
        report,
        format!("{path}@primarycontact"),
        contact.span,
        contact.primary_contact.as_deref(),
        BOOL_FLAG,
    );

    for (index, name) in contact.names.iter().enumerate() {
        check_enum(
            report,
            format!("{path}.name[{index}]@part"),
            name.span,
            name.part.as_deref(),
            NAME_PART,
        );
        check_enum(
            report,
            format!("{path}.name[{index}]@type"),
            name.span,
            name.name_type.as_deref(),
            NAME_TYPE,
        );
    }

    for (index, email) in contact.emails.iter().enumerate() {
        let preferred = attr_value(&email.attributes, "preferredcontact");
        check_enum(
            report,
            format!("{path}.email[{index}]@preferredcontact"),
            email.span,
            preferred,
            BOOL_FLAG,
        );
    }

    for (index, phone) in contact.phones.iter().enumerate() {
        let phone_path = format!("{path}.phone[{index}]");
        check_enum(
            report,
            format!("{phone_path}@type"),
            phone.span,
            attr_value(&phone.attributes, "type"),
            PHONE_TYPE,
        );
        check_enum(
            report,
            format!("{phone_path}@time"),
            phone.span,
            attr_value(&phone.attributes, "time"),
            PHONE_TIME,
        );
        check_enum(
            report,
            format!("{phone_path}@preferredcontact"),
            phone.span,
            attr_value(&phone.attributes, "preferredcontact"),
            BOOL_FLAG,
        );
    }

    for (index, address) in contact.addresses.iter().enumerate() {
        let address_path = format!("{path}.address[{index}]");
        check_enum(
            report,
            format!("{address_path}@type"),
            address.span,
            address.address_type.as_deref(),
            ADDRESS_TYPE,
        );
        if let Some(country) = &address.country {
            check_iso_country(
                report,
                format!("{address_path}.country"),
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
        format!("{path}@interest"),
        vehicle.span,
        vehicle.interest.as_deref(),
        VEHICLE_INTEREST,
    );
    check_enum(
        report,
        format!("{path}@status"),
        vehicle.span,
        vehicle.status.as_deref(),
        VEHICLE_STATUS,
    );

    if let Some(odometer) = &vehicle.odometer {
        check_enum(
            report,
            format!("{path}.odometer@status"),
            odometer.span,
            attr_value(&odometer.attributes, "status"),
            ODOMETER_STATUS,
        );
        check_enum(
            report,
            format!("{path}.odometer@units"),
            odometer.span,
            attr_value(&odometer.attributes, "units"),
            ODOMETER_UNITS,
        );
    }

    if let Some(condition) = &vehicle.condition {
        let value = condition.value();
        let trimmed = value.trim();
        if !trimmed.is_empty() && !CONDITION_VALUES.contains(&trimmed) {
            report.warn(
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
        format!("{path}@type"),
        price.span,
        price.price_type.as_deref(),
        PRICE_TYPE,
    );
    check_enum(
        report,
        format!("{path}@delta"),
        price.span,
        price.delta.as_deref(),
        PRICE_DELTA,
    );
    check_enum(
        report,
        format!("{path}@relativeto"),
        price.span,
        price.relative_to.as_deref(),
        PRICE_RELATIVE_TO,
    );
    if let Some(currency) = price.currency.as_deref() {
        check_iso_currency(report, format!("{path}@currency"), price.span, currency);
    }
}

fn validate_finance(report: &mut ValidationReport<'_>, path: &str, finance: &Finance<'_>) {
    if let Some(method) = &finance.method {
        let value = method.value();
        let trimmed = value.trim();
        if !trimmed.is_empty() && !FINANCE_METHOD.contains(&trimmed) {
            report.warn(
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
            format!("{amount_path}@limit"),
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
    type_values: &[&str],
) {
    check_enum(
        report,
        format!("{path}@type"),
        amount.span,
        attr_value(&amount.attributes, "type"),
        type_values,
    );
    if let Some(currency) = attr_value(&amount.attributes, "currency") {
        check_iso_currency(report, format!("{path}@currency"), amount.span, currency);
    }
}

fn attr_value<'a>(attributes: &'a [Attribute<'a>], name: &str) -> Option<&'a str> {
    attributes
        .iter()
        .find(|attr| attr.name.as_ref() == name)
        .map(|attr| attr.value.as_ref())
}

fn check_enum(
    report: &mut ValidationReport<'_>,
    path: String,
    span: Span,
    value: Option<&str>,
    allowed: &[&str],
) {
    let Some(value) = value else { return };
    if allowed.contains(&value) {
        return;
    }
    let allowed_display = allowed.join(", ");
    report.warn(
        path,
        span,
        format!("value {value:?} is not one of: {allowed_display}"),
    );
}

fn check_iso_currency(report: &mut ValidationReport<'_>, path: String, span: Span, value: &str) {
    if value.len() == 3 && value.chars().all(|ch| ch.is_ascii_uppercase()) {
        return;
    }
    report.warn(
        path,
        span,
        format!("currency {value:?} is not a 3-letter ISO 4217 code"),
    );
}

fn check_iso_country(report: &mut ValidationReport<'_>, path: String, span: Span, value: &str) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return;
    }
    if trimmed.len() == 2 && trimmed.chars().all(|ch| ch.is_ascii_uppercase()) {
        return;
    }
    report.warn(
        path,
        span,
        format!("country {trimmed:?} is not a 2-letter ISO 3166-1 alpha-2 code"),
    );
}

fn check_iso_datetime(report: &mut ValidationReport<'_>, path: String, span: Span, value: &str) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return;
    }
    if is_iso_datetime(trimmed) {
        return;
    }
    report.warn(
        path,
        span,
        format!("date {trimmed:?} is not ISO 8601 datetime"),
    );
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
        check_offset(&bytes[19..], true)
    } else {
        // CCYYMMDDThhmmss with optional fractional + offset
        if !ascii_digits(&bytes[0..8]) || bytes[8] != b'T' || !ascii_digits(&bytes[9..15]) {
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
            } else {
                remainder.len() == 4 && ascii_digits(remainder)
            }
        }
        _ => false,
    }
}

fn ascii_digits(bytes: &[u8]) -> bool {
    !bytes.is_empty() && bytes.iter().all(|b| b.is_ascii_digit())
}

impl<'a> ValidationReport<'a> {
    fn warn(
        &mut self,
        path: impl Into<Cow<'a, str>>,
        span: Span,
        message: impl Into<Cow<'a, str>>,
    ) {
        self.issues.push(ValidationIssue {
            severity: Severity::Warning,
            path: path.into(),
            message: message.into(),
            span: span_option(span),
        });
    }

    fn error(
        &mut self,
        path: impl Into<Cow<'a, str>>,
        span: Span,
        message: impl Into<Cow<'a, str>>,
    ) {
        self.issues.push(ValidationIssue {
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
        if options.strict {
            self.error(path.to_owned(), span, message);
        } else {
            self.warn(path.to_owned(), span, message);
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
