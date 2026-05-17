use adf::{Severity, XmlNode, parse};
use pretty_assertions::assert_eq;
use std::borrow::Cow;

const FULL_LEAD: &str = r#"<?xml version="1.0"?>
<?adf version="1.0"?>
<adf>
  <!-- keep me -->
  <prospect status="new">
    <id sequence="1" source="Cobalt">38889</id>
    <requestdate>2000-03-30T15:30:20-08:00</requestdate>
    <vehicle interest="buy" status="used">
      <year>1997</year>
      <make>Chevrolet</make>
      <model>Blazer</model>
      <vin>1GNDT13W5V2200000</vin>
      <price type="quote" currency="USD">26995</price>
      <option>
        <optionname>Sport</optionname>
        <manufacturercode>p394</manufacturercode>
      </option>
      <partner-score value="hot">97</partner-score>
    </vehicle>
    <customer>
      <contact primarycontact="1">
        <name part="first">John</name>
        <name part="last">Doe</name>
        <email preferredcontact="1">jdoe@example.com</email>
        <phone type="voice" time="morning">393-999-3922</phone>
      </contact>
      <comments>Can deliver by Thursday?</comments>
    </customer>
    <vendor>
      <vendorname>Koons Internet Outlet</vendorname>
    </vendor>
    <provider>
      <name part="full">CarPoint</name>
      <service>Used Car Classifieds</service>
    </provider>
  </prospect>
</adf>"#;

#[test]
fn parses_core_adf_shape() {
    let doc = parse(FULL_LEAD).expect("valid ADF should parse");
    let adf = doc.adf();

    assert_eq!(adf.prospects.len(), 1);
    let prospect = &adf.prospects[0];
    assert_eq!(prospect.status.as_deref(), Some("new"));
    assert_eq!(
        prospect
            .request_date
            .as_ref()
            .map(|date| date.value.as_ref()),
        Some("2000-03-30T15:30:20-08:00")
    );

    let vehicle = &prospect.vehicles[0];
    assert_eq!(vehicle.interest.as_deref(), Some("buy"));
    assert_eq!(vehicle.status.as_deref(), Some("used"));
    assert_eq!(
        vehicle.make.as_ref().map(|make| make.value.as_ref()),
        Some("Chevrolet")
    );
    assert_eq!(vehicle.prices[0].currency.as_deref(), Some("USD"));

    let customer = prospect.customer.as_ref().unwrap();
    let contact = &customer.contacts[0];
    assert_eq!(contact.primary_contact.as_deref(), Some("1"));
    assert_eq!(contact.names[0].value.as_ref(), "John");
    assert_eq!(
        contact.emails[0].attributes[0].name.as_ref(),
        "preferredcontact"
    );
}

#[test]
fn original_preserving_output_is_byte_for_byte_identical() {
    let doc = parse(FULL_LEAD).expect("valid ADF should parse");
    assert!(!doc.is_dirty());
    assert_eq!(doc.to_original_preserving_string().unwrap(), FULL_LEAD);
}

#[test]
fn keeps_unknown_vendor_extensions_in_typed_model() {
    let doc = parse(FULL_LEAD).expect("valid ADF should parse");
    let vehicle = &doc.adf().prospects[0].vehicles[0];

    assert_eq!(vehicle.extensions.len(), 1);
    match &vehicle.extensions[0] {
        XmlNode::Element(element) => {
            assert_eq!(element.name.as_ref(), "partner-score");
            assert_eq!(element.attributes[0].name.as_ref(), "value");
            assert_eq!(element.attributes[0].value.as_ref(), "hot");
        }
        other => panic!("expected unknown element, got {other:?}"),
    }
}

#[test]
fn validation_reports_structural_warnings() {
    let doc = parse(r#"<adf><prospect><customer><contact /></customer></prospect></adf>"#)
        .expect("well formed XML should parse");
    let report = doc.validate();

    assert!(report.is_valid());
    assert!(
        report
            .issues
            .iter()
            .all(|issue| issue.severity == Severity::Warning)
    );
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.message.contains("requestdate"))
    );
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.message.contains("email or phone"))
    );
}

#[test]
fn entity_decoding_allocates_only_when_needed() {
    let doc = parse(
        r#"<adf><prospect><customer><contact><name part="full">Jane &amp; Co</name><email>a@example.com</email></contact></customer></prospect></adf>"#,
    )
    .expect("entity should decode");
    let value = &doc.adf().prospects[0].customer.as_ref().unwrap().contacts[0].names[0].value;

    assert_eq!(value.as_ref(), "Jane & Co");
    assert!(matches!(value, Cow::Owned(_)));
}

#[test]
fn typed_writer_emits_normalized_adf() {
    let doc = parse(FULL_LEAD).expect("valid ADF should parse");
    let xml = doc.to_typed_string().unwrap();

    assert!(xml.starts_with("<?xml version=\"1.0\"?>\n<?adf version=\"1.0\"?>\n<adf>"));
    assert!(xml.contains("<prospect status=\"new\">"));
    assert!(xml.contains("<vehicle interest=\"buy\" status=\"used\">"));
    assert!(xml.contains("<make>Chevrolet</make>"));
    assert!(xml.contains("<email preferredcontact=\"1\">jdoe@example.com</email>"));
}

#[test]
fn supports_multiple_prospects() {
    let doc = parse(
        r#"<adf><prospect><requestdate>one</requestdate></prospect><prospect status="resend"><requestdate>two</requestdate></prospect></adf>"#,
    )
    .expect("multiple prospects should parse");

    assert_eq!(doc.adf().prospects.len(), 2);
    assert_eq!(doc.adf().prospects[1].status.as_deref(), Some("resend"));
}

#[test]
fn rejects_non_whitespace_content_outside_root() {
    assert!(parse("junk<adf/>").is_err());
    assert!(parse("<adf/>junk").is_err());
    assert!(parse(" \n\t<adf/>\n ").is_ok());
    assert!(parse("<!-- before --><adf/><?after root?>").is_ok());
}

#[test]
fn decodes_numeric_character_references() {
    let doc = parse(
        r#"<adf><prospect><customer><contact><name>&#65;&#x42; &amp; Co</name></contact></customer></prospect></adf>"#,
    )
    .expect("numeric character references should parse");
    let value = &doc.adf().prospects[0].customer.as_ref().unwrap().contacts[0].names[0].value;

    assert_eq!(value.as_ref(), "AB & Co");
    assert!(
        doc.to_typed_string()
            .unwrap()
            .contains("<name>AB &amp; Co</name>")
    );
}

#[test]
fn typed_writer_preserves_root_extensions() {
    let doc = parse(r#"<adf><partner-meta key="v">x</partner-meta><prospect /></adf>"#)
        .expect("root extensions should parse");
    let xml = doc.to_typed_string().unwrap();

    assert!(xml.contains(r#"<partner-meta key="v">x</partner-meta>"#));
    assert!(xml.contains("<prospect></prospect>"));
}

#[test]
fn prospect_rewrite_preserves_unknown_compact_element_attributes() {
    let input = r#"<adf><prospect>
  <id sequence="1" source="s" partner="p">123</id>
  <vehicle><price type="quote" taxable="yes" currency="USD">10</price></vehicle>
  <customer><contact><name part="full" xml:lang="en">Jane</name><email>a@example.com</email></contact></customer>
</prospect></adf>"#;

    let mut doc = parse(input).expect("valid ADF should parse");
    doc.prospect_mut(0).unwrap().ids[0].source = Some(Cow::Borrowed("changed"));
    let output = doc.to_original_preserving_string().unwrap();

    assert!(output.contains(r#"<id sequence="1" source="changed" partner="p">123</id>"#));
    assert!(output.contains(r#"<price type="quote" taxable="yes" currency="USD">10</price>"#));
    assert!(output.contains(r#"<name part="full" xml:lang="en">Jane</name>"#));
}

#[test]
fn original_preserving_writer_replaces_only_dirty_prospect_span() {
    let input = r#"<adf>
  <!-- before first -->
  <prospect>
    <requestdate>one</requestdate>
    <vehicle><year>2024</year><make>Toyota</make><model>Camry</model></vehicle>
    <customer><contact><name part="full">A</name><email>a@example.com</email></contact></customer>
    <vendor><vendorname>Dealer One</vendorname></vendor>
  </prospect>
  <!-- between -->
  <prospect status="resend">
    <requestdate>two</requestdate>
    <vehicle><year>2025</year><make>Ford</make><model>F-150</model></vehicle>
  </prospect>
  <!-- after second -->
</adf>"#;
    let untouched_second = r#"<prospect status="resend">
    <requestdate>two</requestdate>
    <vehicle><year>2025</year><make>Ford</make><model>F-150</model></vehicle>
  </prospect>"#;

    let mut doc = parse(input).expect("valid ADF should parse");
    doc.prospect_mut(0).unwrap().vehicles[0]
        .make
        .as_mut()
        .unwrap()
        .value = Cow::Borrowed("Honda");

    assert!(doc.is_dirty());
    let output = doc.to_original_preserving_string().unwrap();

    assert_ne!(output, input);
    assert!(output.contains("<make>Honda</make>"));
    assert!(!output.contains("<make>Toyota</make>"));
    assert!(output.contains("<!-- before first -->"));
    assert!(output.contains("<!-- between -->"));
    assert!(output.contains("<!-- after second -->"));
    assert!(output.contains(untouched_second));
}

#[test]
fn broad_adf_mutation_uses_typed_writer() {
    let mut doc = parse(FULL_LEAD).expect("valid ADF should parse");
    doc.adf_mut().prospects[0].status = Some(Cow::Borrowed("contacted"));

    let output = doc.to_original_preserving_string().unwrap();

    assert!(output.starts_with("<?xml version=\"1.0\"?>\n<?adf version=\"1.0\"?>\n<adf>"));
    assert!(output.contains("<prospect status=\"contacted\">"));
    assert!(!output.contains("<!-- keep me -->"));
}
