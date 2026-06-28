use crate::error::Error;
use crate::model::{Adf, Contact, Customer, Prospect, Provider, Vehicle, Vendor};
use crate::validate::{Severity, ValidationReport};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct DocumentStats {
    pub prospects: usize,
    pub vehicles: usize,
    pub contacts: usize,
    pub addresses: usize,
    pub extensions: usize,
}

impl DocumentStats {
    pub(crate) fn from_adf(adf: &Adf<'_>) -> Self {
        let mut stats = Self {
            prospects: adf.prospects.len(),
            extensions: adf.extensions.len(),
            ..Self::default()
        };
        for prospect in &adf.prospects {
            stats.add_prospect(prospect);
        }
        stats
    }

    fn add_prospect(&mut self, prospect: &Prospect<'_>) {
        self.vehicles += prospect.vehicles.len();
        self.extensions += prospect.extensions.len();

        for vehicle in &prospect.vehicles {
            self.add_vehicle(vehicle);
        }
        if let Some(customer) = &prospect.customer {
            self.add_customer(customer);
        }
        if let Some(vendor) = &prospect.vendor {
            self.add_vendor(vendor);
        }
        if let Some(provider) = &prospect.provider {
            self.add_provider(provider);
        }
    }

    fn add_vehicle(&mut self, vehicle: &Vehicle<'_>) {
        self.extensions += vehicle.extensions.len();
        for colors in &vehicle.color_combinations {
            self.extensions += colors.extensions.len();
        }
        for option in &vehicle.options {
            self.extensions += option.extensions.len();
        }
        if let Some(finance) = &vehicle.finance {
            self.extensions += finance.extensions.len();
        }
    }

    fn add_customer(&mut self, customer: &Customer<'_>) {
        self.contacts += customer.contacts.len();
        self.extensions += customer.extensions.len();
        if let Some(timeframe) = &customer.timeframe {
            self.extensions += timeframe.extensions.len();
        }
        for contact in &customer.contacts {
            self.add_contact(contact);
        }
    }

    fn add_vendor(&mut self, vendor: &Vendor<'_>) {
        self.contacts += vendor.contacts.len();
        self.extensions += vendor.extensions.len();
        for contact in &vendor.contacts {
            self.add_contact(contact);
        }
    }

    fn add_provider(&mut self, provider: &Provider<'_>) {
        self.contacts += provider.contacts.len();
        self.extensions += provider.extensions.len();
        for contact in &provider.contacts {
            self.add_contact(contact);
        }
    }

    fn add_contact(&mut self, contact: &Contact<'_>) {
        self.addresses += contact.addresses.len();
        self.extensions += contact.extensions.len();
        for address in &contact.addresses {
            self.extensions += address.extensions.len();
        }
    }
}

pub(crate) fn dirty_prospect_count(dirty_prospects: &[bool]) -> usize {
    dirty_prospects.iter().filter(|dirty| **dirty).count()
}

pub(crate) fn validation_issue_counts(report: &ValidationReport<'_>) -> (usize, usize) {
    let mut warnings = 0;
    let mut errors = 0;
    for issue in &report.issues {
        match issue.severity {
            Severity::Warning => warnings += 1,
            Severity::Error => errors += 1,
        }
    }
    (warnings, errors)
}

pub(crate) fn error_kind(error: &Error) -> &'static str {
    match error {
        Error::Xml { .. } => "xml",
        Error::Attribute { .. } => "attribute",
        Error::Encoding { .. } => "encoding",
        Error::Utf8 { .. } => "utf8",
        Error::MismatchedEnd { .. } => "mismatched_end",
        Error::UnexpectedEnd { .. } => "unexpected_end",
        Error::ContentOutsideRoot { .. } => "content_outside_root",
        Error::InvalidCharacterReference { .. } => "invalid_character_reference",
        Error::DocTypeForbidden { .. } => "doctype_forbidden",
        Error::DocTypeTooLong { .. } => "doctype_too_long",
        Error::MissingRoot => "missing_root",
        Error::MultipleRoots => "multiple_roots",
        Error::Io(_) => "io",
    }
}

pub(crate) fn error_position(error: &Error) -> Option<u64> {
    match error {
        Error::Xml { position, .. }
        | Error::Attribute { position, .. }
        | Error::Encoding { position, .. }
        | Error::Utf8 { position, .. }
        | Error::MismatchedEnd { position, .. }
        | Error::UnexpectedEnd { position, .. }
        | Error::ContentOutsideRoot { position }
        | Error::InvalidCharacterReference { position, .. }
        | Error::DocTypeForbidden { position }
        | Error::DocTypeTooLong { position, .. } => Some(*position),
        Error::MissingRoot | Error::MultipleRoots | Error::Io(_) => None,
    }
}

pub(crate) fn record_error(operation: &'static str, error: &Error) {
    tracing::debug!(
        operation,
        error_kind = error_kind(error),
        error_position = ?error_position(error),
        "ADF operation failed"
    );
}
