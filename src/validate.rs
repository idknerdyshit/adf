use crate::model::*;
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

pub(crate) fn validate<'a>(adf: &Adf<'a>) -> ValidationReport<'a> {
    let mut report = ValidationReport::default();

    if adf.prospects.is_empty() {
        report.error("adf", "ADF document should contain at least one prospect");
    }

    for (index, prospect) in adf.prospects.iter().enumerate() {
        let path = format!("adf.prospect[{index}]");
        if prospect.request_date.is_none() {
            report.warn(path.clone(), "prospect is missing requestdate");
        }
        if prospect.vehicles.is_empty() {
            report.warn(path.clone(), "prospect is missing vehicle");
        }
        if prospect.customer.is_none() {
            report.warn(path.clone(), "prospect is missing customer");
        }
        if prospect.vendor.is_none() {
            report.warn(path.clone(), "prospect is missing vendor");
        }
        if let Some(customer) = &prospect.customer {
            validate_customer(&mut report, &path, customer);
        }
        if let Some(vendor) = &prospect.vendor {
            validate_vendor(&mut report, &path, vendor);
        }
        for (vehicle_index, vehicle) in prospect.vehicles.iter().enumerate() {
            validate_vehicle(
                &mut report,
                &format!("{path}.vehicle[{vehicle_index}]"),
                vehicle,
            );
        }
    }

    report
}

fn validate_customer(
    report: &mut ValidationReport<'_>,
    prospect_path: &str,
    customer: &Customer<'_>,
) {
    if customer.contacts.is_empty() {
        report.warn(
            format!("{prospect_path}.customer"),
            "customer is missing contact",
        );
    }

    for (index, contact) in customer.contacts.iter().enumerate() {
        let path = format!("{prospect_path}.customer.contact[{index}]");
        if contact.names.is_empty() {
            report.warn(path.clone(), "contact is missing name");
        }
        if contact.emails.is_empty() && contact.phones.is_empty() {
            report.warn(path, "contact should contain email or phone");
        }
    }
}

fn validate_vendor(report: &mut ValidationReport<'_>, prospect_path: &str, vendor: &Vendor<'_>) {
    if vendor.vendor_name.is_none() {
        report.warn(
            format!("{prospect_path}.vendor"),
            "vendor is missing vendorname",
        );
    }
}

fn validate_vehicle(report: &mut ValidationReport<'_>, path: &str, vehicle: &Vehicle<'_>) {
    if vehicle.year.is_none() {
        report.warn(path.to_owned(), "vehicle is missing year");
    }
    if vehicle.make.is_none() {
        report.warn(path.to_owned(), "vehicle is missing make");
    }
    if vehicle.model.is_none() {
        report.warn(path.to_owned(), "vehicle is missing model");
    }
}

impl<'a> ValidationReport<'a> {
    fn warn(&mut self, path: impl Into<Cow<'a, str>>, message: impl Into<Cow<'a, str>>) {
        self.issues.push(ValidationIssue {
            severity: Severity::Warning,
            path: path.into(),
            message: message.into(),
        });
    }

    fn error(&mut self, path: impl Into<Cow<'a, str>>, message: impl Into<Cow<'a, str>>) {
        self.issues.push(ValidationIssue {
            severity: Severity::Error,
            path: path.into(),
            message: message.into(),
        });
    }
}
