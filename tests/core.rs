use adf::{
    AdfDocument, DEFAULT_MAX_DOCTYPE_LEN, Error, ParseOptions, Severity, TextPart,
    ValidationOptions, ValidationReport, XmlNode, parse, parse_with,
};
use insta::assert_snapshot;
use pretty_assertions::assert_eq;
use std::borrow::Cow;
use std::fmt::Write as _;
use std::io::Write as IoWrite;
use std::ops::Range;
use std::sync::{Arc, Mutex};
use tracing::Level;
use tracing_subscriber::fmt::MakeWriter;

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

const MAXIMAL_ADF: &str = r#"<adf xmlns:p="urn:partner" p:batch="42"><prospect status="new" partner="prospect"><id sequence="1" source="lead">P-1</id><requestdate>2024-01-02T03:04:05-05:00</requestdate><vehicle interest="buy" status="new" partner-id="v"><id sequence="2" source="vin">V-1</id><year>2024</year><make>Honda</make><model>Civic</model><vin>1HGCM82633A004352</vin><stock>STK-1</stock><trim>Touring</trim><doors>4</doors><bodystyle>Sedan</bodystyle><transmission>Automatic</transmission><odometer status="original" units="mi">123</odometer><condition>excellent</condition><colorcombination><interiorcolor>Black</interiorcolor><exteriorcolor>Blue</exteriorcolor><preference>1</preference></colorcombination><imagetag>https://example.test/car.jpg</imagetag><price type="quote" currency="USD" delta="absolute" relativeto="msrp" source="dealer">25000</price><pricecomments>Plus taxes</pricecomments><option><optionname>Sunroof</optionname><manufacturercode>SUN</manufacturercode><stock>in-stock</stock><weighting>1</weighting><price type="msrp" currency="USD">900</price></option><finance><method>finance</method><amount type="downpayment" limit="minimum" currency="USD">1000</amount><balance type="finance" currency="USD">24000</balance></finance><comments>Vehicle comment</comments></vehicle><customer><contact primarycontact="1"><name part="full" type="individual">Jane Doe</name><email preferredcontact="1">jane@example.test</email><phone type="voice" time="day" preferredcontact="0">555-0100</phone><address type="home"><street>1 Main</street><street>Unit 2</street><apartment>2A</apartment><city>Detroit</city><regioncode>MI</regioncode><postalcode>48201</postalcode><country>US</country></address></contact><id sequence="3" source="crm">C-1</id><timeframe><description>soon</description><earliestdate>2024-02-01</earliestdate><latestdate>2024-03-01</latestdate></timeframe><comments>Customer comment</comments></customer><vendor><id source="vendor">D-1</id><vendorname>Dealer</vendorname><url>https://dealer.example</url><contact><name part="full">Sales</name><email>sales@example.test</email></contact></vendor><provider><id source="provider">PR-1</id><name part="full" type="business">Provider</name><service>Leads</service><url>https://provider.example</url><email>provider@example.test</email><phone type="voice">555-0101</phone><contact><name part="full">Provider Contact</name><email>pc@example.test</email></contact></provider></prospect></adf>"#;

fn normalized_issues(input: &str, report: &ValidationReport<'_>) -> String {
    let mut output = String::new();
    for issue in &report.issues {
        let severity = match issue.severity {
            Severity::Warning => "warning",
            Severity::Error => "error",
        };
        let span_text = issue
            .span
            .map(|span| input[span.start..span.end].replace('\n', "\\n"))
            .unwrap_or_else(|| "<none>".to_owned());
        writeln!(
            output,
            "{severity}|{}|{}|{span_text}",
            issue.path, issue.message
        )
        .unwrap();
    }
    output
}

fn prospect_span(doc: &AdfDocument<'_>, index: usize) -> Range<usize> {
    doc.root()
        .children
        .iter()
        .filter_map(|node| match node {
            XmlNode::Element(element) if element.name.as_ref() == "prospect" => {
                Some(element.span.start..element.span.end)
            }
            _ => None,
        })
        .nth(index)
        .expect("prospect span")
}

fn assert_original_unchanged_outside_span(input: &str, output: &str, span: Range<usize>) {
    let prefix = &input[..span.start];
    let suffix = &input[span.end..];
    assert!(
        output.starts_with(prefix),
        "output prefix before dirty span changed\nexpected prefix:\n{prefix}\noutput:\n{output}"
    );
    assert!(
        output.ends_with(suffix),
        "output suffix after dirty span changed\nexpected suffix:\n{suffix}\noutput:\n{output}"
    );
}

#[derive(Clone, Default)]
struct TraceBuffer {
    bytes: Arc<Mutex<Vec<u8>>>,
}

impl TraceBuffer {
    fn output(&self) -> String {
        String::from_utf8(self.bytes.lock().unwrap().clone()).expect("trace output is UTF-8")
    }
}

struct TraceBufferWriter {
    bytes: Arc<Mutex<Vec<u8>>>,
}

impl IoWrite for TraceBufferWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.bytes.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'writer> MakeWriter<'writer> for TraceBuffer {
    type Writer = TraceBufferWriter;

    fn make_writer(&'writer self) -> Self::Writer {
        TraceBufferWriter {
            bytes: Arc::clone(&self.bytes),
        }
    }
}

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
            .map(|date| date.value().into_owned())
            .as_deref(),
        Some("2000-03-30T15:30:20-08:00")
    );

    let vehicle = &prospect.vehicles[0];
    assert_eq!(vehicle.interest.as_deref(), Some("buy"));
    assert_eq!(vehicle.status.as_deref(), Some("used"));
    assert_eq!(
        vehicle
            .make
            .as_ref()
            .map(|make| make.value().into_owned())
            .as_deref(),
        Some("Chevrolet")
    );
    assert_eq!(vehicle.prices[0].currency.as_deref(), Some("USD"));

    let customer = prospect.customer.as_ref().unwrap();
    let contact = &customer.contacts[0];
    assert_eq!(contact.primary_contact.as_deref(), Some("1"));
    assert_eq!(contact.names[0].value().as_ref(), "John");
    assert_eq!(
        contact.emails[0].attributes[0].name.as_ref(),
        "preferredcontact"
    );
}

#[test]
fn tracing_omits_pii_and_secret_payloads() {
    let buffer = TraceBuffer::default();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .without_time()
        .with_ansi(false)
        .with_writer(buffer.clone())
        .finish();

    let input = r#"<adf><prospect status="mystery" partner-token="dealer-secret"><id source="crm">LEAD-SECRET-42</id><requestdate>not-a-date</requestdate><vehicle><year>2024</year><make>Honda</make><model>Civic</model><vin>1HGCM82633A004352</vin><comments>Customer says password is rosebud</comments></vehicle><customer><contact primarycontact="1"><name part="full">Jane Private</name><email preferredcontact="1">jane.private@example.com</email><phone type="voice">313-555-0182</phone><address type="home"><street>123 Secret Lane</street><city>Detroit</city><postalcode>48201</postalcode><country>US</country></address><p:loyalty xmlns:p="urn:partner" api-key="sk_live_123">VIP customer</p:loyalty></contact><comments>SSN 111-22-3333</comments></customer><vendor><vendorname>Confidential Motors</vendorname><url>https://dealer.example/private</url></vendor></prospect></adf>"#;

    tracing::subscriber::set_global_default(subscriber).expect("global subscriber installs once");
    tracing::callsite::rebuild_interest_cache();

    let mut doc = parse(input).expect("valid ADF should parse");
    let report = doc.validate();
    assert!(
        !report.issues.is_empty(),
        "input should produce validation events"
    );
    doc.prospect_mut(0).unwrap().status = Some(Cow::Borrowed("resend"));
    let _ = doc.to_original_preserving_string().unwrap();
    let _ = doc.to_typed_string().unwrap();
    let _ = doc.root();

    let traces = buffer.output();
    assert!(
        traces.contains("ADF validation complete"),
        "missing validation trace:\n{traces}"
    );
    assert!(
        traces.contains("ADF write complete"),
        "missing write trace:\n{traces}"
    );

    for sensitive in [
        "mystery",
        "dealer-secret",
        "LEAD-SECRET-42",
        "not-a-date",
        "1HGCM82633A004352",
        "password is rosebud",
        "Jane Private",
        "jane.private@example.com",
        "313-555-0182",
        "123 Secret Lane",
        "Detroit",
        "48201",
        "sk_live_123",
        "VIP customer",
        "SSN 111-22-3333",
        "Confidential Motors",
        "https://dealer.example/private",
    ] {
        assert!(
            !traces.contains(sensitive),
            "trace output leaked sensitive payload {sensitive:?}:\n{traces}"
        );
    }
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
    let input = r#"<adf><prospect><customer><contact /></customer></prospect></adf>"#;
    let doc = parse(input).expect("well formed XML should parse");
    let report = doc.validate();

    assert!(report.is_valid());
    assert!(
        report
            .issues
            .iter()
            .all(|issue| issue.severity == Severity::Warning)
    );
    assert_snapshot!(
        "validation_reports_structural_warnings",
        normalized_issues(input, &report)
    );
}

#[test]
fn performance_invariant_entity_decoding_allocates_when_needed() {
    let doc = parse(
        r#"<adf><prospect><customer><contact><name part="full">Jane &amp; Co</name><email>a@example.com</email></contact></customer></prospect></adf>"#,
    )
    .expect("entity should decode");
    let value = doc.adf().prospects[0].customer.as_ref().unwrap().contacts[0].names[0].value();

    assert_eq!(value.as_ref(), "Jane & Co");
    assert!(matches!(value, Cow::Owned(_)));
}

#[test]
fn typed_writer_emits_normalized_adf() {
    let doc = parse(FULL_LEAD).expect("valid ADF should parse");
    let xml = doc.to_typed_string().unwrap();

    assert_snapshot!("typed_writer_emits_normalized_adf", xml);
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
    let value = doc.adf().prospects[0].customer.as_ref().unwrap().contacts[0].names[0].value();

    assert_eq!(value.as_ref(), "AB & Co");
    assert_snapshot!(
        "decodes_numeric_character_references",
        doc.to_typed_string().unwrap()
    );
}

#[test]
fn typed_writer_preserves_root_extensions() {
    let doc = parse(r#"<adf><partner-meta key="v">x</partner-meta><prospect /></adf>"#)
        .expect("root extensions should parse");
    let xml = doc.to_typed_string().unwrap();

    assert_snapshot!("typed_writer_preserves_root_extensions", xml);
}

#[test]
fn typed_writer_preserves_root_attributes_for_namespaced_extensions() {
    let input =
        r#"<adf xmlns:p="urn:partner" p:batch="abc"><p:meta key="v">x</p:meta><prospect /></adf>"#;
    let doc = parse(input).expect("root attributes should parse");
    let xml = doc.to_typed_string().unwrap();

    let reparsed = parse(&xml).expect("typed XML should reparse");
    assert_eq!(reparsed.adf().attributes.len(), 2);
    assert_snapshot!(
        "typed_writer_preserves_root_attributes_for_namespaced_extensions",
        xml
    );
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

    assert_snapshot!(
        "prospect_rewrite_preserves_unknown_compact_element_attributes",
        output
    );
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
    let mut doc = parse(input).expect("valid ADF should parse");
    let dirty_span = prospect_span(&doc, 0);
    doc.prospect_mut(0).unwrap().vehicles[0]
        .make
        .as_mut()
        .unwrap()
        .set_value(Cow::Borrowed("Honda"));

    assert!(doc.is_dirty());
    let output = doc.to_original_preserving_string().unwrap();

    assert_ne!(output, input);
    assert_original_unchanged_outside_span(input, &output, dirty_span);
    assert!(output.contains("<make>Honda</make>"));
    assert!(!output.contains("<make>Toyota</make>"));
    assert_snapshot!(
        "original_preserving_writer_replaces_only_dirty_prospect_span",
        output
    );
}

#[test]
fn broad_adf_mutation_uses_typed_writer() {
    let mut doc = parse(FULL_LEAD).expect("valid ADF should parse");
    doc.adf_mut().prospects[0].status = Some(Cow::Borrowed("contacted"));

    let output = doc.to_original_preserving_string().unwrap();

    assert!(!output.contains("<!-- keep me -->"));
    assert_snapshot!("broad_adf_mutation_uses_typed_writer", output);
}

#[test]
fn typed_writer_emits_contact_before_id_in_customer() {
    let input = r#"<adf><prospect><customer><id source="crm">99</id><contact><name part="full">A</name><email>a@example.com</email></contact></customer></prospect></adf>"#;
    let doc = parse(input).expect("valid ADF should parse");
    let output = doc.to_typed_string().unwrap();

    let contact_at = output.find("<contact").expect("contact element");
    let id_at = output.find("<id ").expect("id element");
    assert!(
        contact_at < id_at,
        "contact must precede id per DTD: {output}"
    );
    assert_snapshot!("typed_writer_emits_contact_before_id_in_customer", output);
}

#[test]
fn typed_writer_preserves_unknown_container_attributes() {
    let input = r#"<adf><prospect xmlns:p="urn:partner"><vehicle interest="buy" partner-id="x"><year>2024</year><make>Honda</make><model>Civic</model></vehicle><customer><contact partner="y"><name part="full">A</name><email>a@example.com</email></contact></customer></prospect></adf>"#;
    let doc = parse(input).expect("valid ADF should parse");
    let output = doc.to_typed_string().unwrap();

    let reparsed = parse(&output).expect("typed XML should reparse");
    let prospect = &reparsed.adf().prospects[0];
    assert_eq!(prospect.attributes[0].name.as_ref(), "xmlns:p");
    assert_eq!(
        prospect.vehicles[0].attributes[1].name.as_ref(),
        "partner-id"
    );
    assert_eq!(
        prospect.customer.as_ref().unwrap().contacts[0].attributes[0]
            .name
            .as_ref(),
        "partner"
    );
    assert_snapshot!(
        "typed_writer_preserves_unknown_container_attributes",
        output
    );
}

#[test]
fn prospect_rewrite_preserves_unknown_container_attributes() {
    let input = r#"<adf><prospect>
  <vehicle interest="buy" partner-id="x"><year>2024</year><make>Honda</make><model>Civic</model></vehicle>
  <customer><contact partner="y" primarycontact="1"><name part="full">A</name><email>a@example.com</email></contact></customer>
  <address-meta><address type="home" partner="z"><street>1 Main</street></address></address-meta>
</prospect></adf>"#;

    let mut doc = parse(input).expect("valid ADF should parse");
    doc.prospect_mut(0).unwrap().vehicles[0]
        .make
        .as_mut()
        .unwrap()
        .set_value(Cow::Borrowed("Honda"));
    let output = doc.to_original_preserving_string().unwrap();

    assert_snapshot!(
        "prospect_rewrite_preserves_unknown_container_attributes",
        output
    );
}

#[test]
fn typed_writer_preserves_entity_refs() {
    let input = r#"<adf><prospect><customer><contact><name part="full">A</name><email>a@example.com</email></contact><comments>Jane &amp; &nbsp; Co</comments></customer></prospect></adf>"#;
    let doc = parse(input).expect("valid ADF should parse");
    let output = doc.to_typed_string().unwrap();

    let reparsed = parse(&output).expect("typed XML should reparse");
    let comments = reparsed.adf().prospects[0]
        .customer
        .as_ref()
        .unwrap()
        .comments
        .as_ref()
        .unwrap();
    assert!(
        comments
            .parts
            .iter()
            .any(|part| matches!(part, TextPart::EntityRef(name) if name.as_ref() == "nbsp"))
    );
    assert_snapshot!("typed_writer_preserves_entity_refs", output);
}

#[test]
fn typed_writer_preserves_embedded_partner_xml_in_text_elements() {
    let input = r#"<adf xmlns:p="urn:partner"><prospect><customer><contact><name part="full">A</name><email>a@example.com</email></contact><comments>Jane <p:token code="x">VIP</p:token> Co</comments></customer></prospect></adf>"#;
    let doc = parse(input).expect("valid ADF should parse");
    let output = doc.to_typed_string().unwrap();

    let reparsed = parse(&output).expect("typed XML should reparse");
    let comments = reparsed.adf().prospects[0]
        .customer
        .as_ref()
        .unwrap()
        .comments
        .as_ref()
        .unwrap();
    assert!(matches!(comments.parts[1], TextPart::Node(_)));
    assert_snapshot!(
        "typed_writer_preserves_embedded_partner_xml_in_text_elements",
        output
    );
}

#[test]
fn typed_writer_keeps_extensions_near_original_dtd_slot() {
    let input = r#"<adf><prospect><vehicle><year>2024</year><partner-score>97</partner-score><make>Honda</make><model>Civic</model></vehicle><customer><contact><name part="full">A</name><email>a@example.com</email></contact></customer></prospect></adf>"#;
    let doc = parse(input).expect("valid ADF should parse");
    let output = doc.to_typed_string().unwrap();

    let year_at = output.find("<year>").expect("year");
    let extension_at = output.find("<partner-score>").expect("extension");
    let make_at = output.find("<make>").expect("make");
    assert!(year_at < extension_at);
    assert!(extension_at < make_at);
    assert_snapshot!(
        "typed_writer_keeps_extensions_near_original_dtd_slot",
        output
    );
}

#[test]
fn typed_writer_preserves_cdata_wrapper() {
    let input = r#"<adf><prospect><customer><contact><name part="full">A</name><email>a@example.com</email></contact><comments><![CDATA[<b>hi</b>]]></comments></customer></prospect></adf>"#;
    let doc = parse(input).expect("valid ADF should parse");
    let output = doc.to_typed_string().unwrap();

    let reparsed = parse(&output).expect("typed XML should reparse");
    let comments = reparsed.adf().prospects[0]
        .customer
        .as_ref()
        .unwrap()
        .comments
        .as_ref()
        .unwrap();
    assert!(matches!(comments.parts[0], TextPart::CData(_)));
    assert_snapshot!("typed_writer_preserves_cdata_wrapper", output);
}

#[test]
fn typed_writer_splits_cdata_containing_terminator() {
    let parts = vec![TextPart::CData(Cow::Borrowed("before]]>after"))];
    let mut doc = parse(r#"<adf><prospect><customer><contact><name part="full">A</name><email>a@example.com</email></contact><comments>x</comments></customer></prospect></adf>"#).expect("valid ADF should parse");
    let comments = doc.adf_mut().prospects[0]
        .customer
        .as_mut()
        .unwrap()
        .comments
        .as_mut()
        .unwrap();
    comments.parts = parts;

    let output = doc.to_typed_string().unwrap();

    assert_snapshot!("typed_writer_splits_cdata_containing_terminator", output);

    let reparsed = parse(&output).expect("reparses cleanly");
    let value = reparsed.adf().prospects[0]
        .customer
        .as_ref()
        .unwrap()
        .comments
        .as_ref()
        .unwrap()
        .value();
    assert_eq!(value.as_ref(), "before]]>after");
}

#[test]
fn validate_strict_promotes_required_fields_to_errors() {
    let input = r#"<adf><prospect><customer><contact /></customer></prospect></adf>"#;
    let doc = parse(input).expect("valid ADF should parse");

    let lenient = doc.validate();
    assert!(lenient.is_valid());

    let strict = doc.validate_strict();
    assert!(!strict.is_valid());
    assert_snapshot!(
        "validate_strict_promotes_required_fields_to_errors",
        normalized_issues(input, &strict)
    );
}

#[test]
fn validate_warns_on_bad_enum_values() {
    let input = r#"<adf><prospect status="weird"><vehicle interest="loan" status="brand-new"><year>2024</year><make>X</make><model>Y</model><price type="bizarre" currency="USD">1</price></vehicle><customer><contact><name part="full">A</name><email>a@example.com</email></contact></customer><vendor><vendorname>V</vendorname><contact><name part="full">B</name><email>b@example.com</email></contact></vendor></prospect></adf>"#;
    let doc = parse(input).expect("valid ADF should parse");
    let report = doc.validate();

    assert_snapshot!(
        "validate_warns_on_bad_enum_values",
        normalized_issues(input, &report)
    );
}

#[test]
fn validate_warns_on_bad_iso_formats() {
    let input = r#"<adf><prospect><requestdate>not-a-date</requestdate><vehicle><year>2024</year><make>X</make><model>Y</model><price type="quote" currency="usd">1</price></vehicle><customer><contact><name part="full">A</name><email>a@example.com</email><address type="home"><country>USA</country></address></contact></customer><vendor><vendorname>V</vendorname><contact><name part="full">B</name><email>b@example.com</email></contact></vendor></prospect></adf>"#;
    let doc = parse(input).expect("valid ADF should parse");
    let report = doc.validate();

    assert_snapshot!(
        "validate_warns_on_bad_iso_formats",
        normalized_issues(input, &report)
    );
}

#[test]
fn validate_strict_passes_minimal_spec_example() {
    let minimal = r#"<?xml version="1.0"?>
<?adf version="1.0"?>
<adf>
  <prospect>
    <requestdate>2024-01-02T03:04:05-05:00</requestdate>
    <vehicle>
      <year>2024</year>
      <make>Honda</make>
      <model>Civic</model>
    </vehicle>
    <customer>
      <contact>
        <name part="full">Jane Doe</name>
        <email>jane@example.com</email>
      </contact>
    </customer>
    <vendor>
      <vendorname>Dealer</vendorname>
      <contact>
        <name part="full">Sales Desk</name>
        <email>sales@example.com</email>
      </contact>
    </vendor>
  </prospect>
</adf>"#;

    let doc = parse(minimal).expect("valid ADF should parse");
    let report = doc.validate_strict();
    let errors: Vec<_> = report
        .issues
        .iter()
        .filter(|issue| issue.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "expected no errors, got: {errors:#?}");
}

#[test]
fn typed_writer_emits_maximal_modeled_adf_fixture() {
    let doc = parse(MAXIMAL_ADF).expect("maximal modeled ADF should parse");
    let report = doc.validate_strict();
    let errors: Vec<_> = report
        .issues
        .iter()
        .filter(|issue| issue.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "expected no errors, got: {errors:#?}");

    assert_snapshot!(
        "typed_writer_emits_maximal_modeled_adf_fixture",
        doc.to_typed_string().unwrap()
    );
}

#[test]
fn validate_with_lenient_options_matches_default() {
    let doc = parse(r#"<adf><prospect><customer><contact /></customer></prospect></adf>"#)
        .expect("valid ADF should parse");
    let lenient = adf::validate_with(doc.adf(), ValidationOptions::default());
    assert!(lenient.is_valid());
}

#[test]
fn pricecomment_singular_survives_as_extension() {
    let input = r#"<adf><prospect><vehicle><year>2024</year><make>Honda</make><model>Civic</model><pricecomment>special offer</pricecomment></vehicle><customer><contact><name part="full">A</name><email>a@example.com</email></contact></customer></prospect></adf>"#;

    let doc = parse(input).expect("valid ADF should parse");
    let original = doc.to_original_preserving_string().unwrap();
    assert_eq!(original, input);

    let vehicle = &doc.adf().prospects[0].vehicles[0];
    assert!(vehicle.extensions.iter().any(
        |node| matches!(node, XmlNode::Element(element) if element.name.as_ref() == "pricecomment")
    ));
    assert!(vehicle.price_comments.is_none());

    let typed = doc.to_typed_string().unwrap();
    assert!(!typed.contains("<pricecomments>"));
    assert_snapshot!("pricecomment_singular_survives_as_extension", typed);
}

#[test]
fn validation_issues_carry_byte_spans_into_original_input() {
    let input = r#"<adf>
  <prospect status="weird">
    <vehicle interest="loan">
      <year>2024</year>
      <make>X</make>
      <model>Y</model>
      <price type="bizarre" currency="usd">1</price>
    </vehicle>
    <customer><contact /></customer>
  </prospect>
</adf>"#;

    let doc = parse(input).expect("valid ADF should parse");
    let report = doc.validate();

    let prospect_status = report
        .issues
        .iter()
        .find(|issue| issue.path.ends_with("prospect[0]@status"))
        .expect("status enum issue");
    let span = prospect_status.span.expect("prospect should have span");
    assert!(input[span.start..span.end].starts_with("<prospect status=\"weird\""));
    assert!(input[span.start..span.end].ends_with("</prospect>"));

    let currency = report
        .issues
        .iter()
        .find(|issue| issue.path.ends_with("price[0]@currency"))
        .expect("currency issue");
    let span = currency.span.expect("price should have span");
    assert_eq!(
        &input[span.start..span.end],
        r#"<price type="bizarre" currency="usd">1</price>"#
    );

    let missing_name = report
        .issues
        .iter()
        .find(|issue| issue.message.contains("missing name"))
        .expect("missing-name issue");
    let span = missing_name.span.expect("contact should have span");
    assert_eq!(&input[span.start..span.end], "<contact />");
}

#[test]
fn empty_adf_issue_span_covers_root_element() {
    let input = "<adf></adf>";
    let doc = parse(input).expect("valid ADF should parse");
    let report = doc.validate();

    let issue = report
        .issues
        .iter()
        .find(|issue| issue.message.contains("at least one prospect"))
        .expect("empty-adf issue");
    let span = issue.span.expect("adf root should have span");
    assert_eq!(&input[span.start..span.end], "<adf></adf>");
}

#[test]
fn performance_invariant_parser_borrows_element_names_for_ascii_input() {
    let input = r#"<adf><prospect><vehicle><year>2024</year></vehicle></prospect></adf>"#;
    let doc = parse(input).expect("valid ADF should parse");
    let root = doc.root();
    let prospect = match &root.children[0] {
        XmlNode::Element(element) => element,
        other => panic!("expected element, got {other:?}"),
    };
    assert!(matches!(prospect.name, Cow::Borrowed(_)));
    let vehicle = match &prospect.children[0] {
        XmlNode::Element(element) => element,
        other => panic!("expected element, got {other:?}"),
    };
    assert!(matches!(vehicle.name, Cow::Borrowed(_)));
}

#[test]
fn lazy_root_remains_available_after_typed_mutation() {
    let input = r#"<adf><prospect><vehicle><year>2024</year></vehicle></prospect></adf>"#;
    let mut doc = parse(input).expect("valid ADF should parse");
    doc.adf_mut().prospects[0].status = Some(Cow::Borrowed("resend"));

    let root = doc.root();
    assert_eq!(root.name.as_ref(), "adf");
    let prospect = match &root.children[0] {
        XmlNode::Element(element) => element,
        other => panic!("expected element, got {other:?}"),
    };
    assert_eq!(prospect.name.as_ref(), "prospect");
}

#[test]
fn default_parse_preserves_small_doctype() {
    let input = "<!DOCTYPE adf>\n<adf><prospect /></adf>";
    let doc = parse(input).expect("a small DOCTYPE should be preserved by default");
    assert!(matches!(
        doc.root().name,
        Cow::Borrowed("adf") | Cow::Owned(_)
    ));
    let out = doc.to_original_preserving_string().unwrap();
    assert_eq!(out, input);
}

#[test]
fn reject_doctype_option_errors_on_dtd() {
    let input = "<!DOCTYPE adf>\n<adf><prospect /></adf>";
    let options = ParseOptions::default().reject_doctype(true);
    assert!(matches!(
        parse_with(input, &options),
        Err(Error::DocTypeForbidden { .. })
    ));
    // A document without a DOCTYPE still parses under the strict option.
    assert!(parse_with("<adf><prospect /></adf>", &options).is_ok());
}

#[test]
fn default_doctype_length_cap_rejects_entity_bomb() {
    // A DTD internal subset large enough to exceed the default cap, e.g. a
    // billion-laughs style entity definition block.
    let bomb = format!(
        "<!DOCTYPE adf [ {} ]>\n<adf><prospect /></adf>",
        "<!ENTITY a \"aaaaaaaaaa\">".repeat(400)
    );
    assert!(bomb.len() > DEFAULT_MAX_DOCTYPE_LEN);
    match parse(&bomb) {
        Err(Error::DocTypeTooLong { length, limit, .. }) => {
            assert_eq!(limit, DEFAULT_MAX_DOCTYPE_LEN);
            assert!(length > limit);
        }
        other => panic!("expected DocTypeTooLong, got {other:?}"),
    }
}

#[test]
fn doctype_length_cap_is_configurable() {
    let input = "<!DOCTYPE adf [ <!ENTITY x \"value\"> ]>\n<adf><prospect /></adf>";

    // A tiny cap rejects an otherwise harmless internal subset.
    let tight = ParseOptions::default().max_doctype_len(4);
    assert!(matches!(
        parse_with(input, &tight),
        Err(Error::DocTypeTooLong { limit: 4, .. })
    ));

    // Disabling the cap accepts an arbitrarily large internal subset.
    let unlimited = ParseOptions::default().without_doctype_limit();
    let huge = format!(
        "<!DOCTYPE adf [ {} ]>\n<adf><prospect /></adf>",
        "<!ENTITY a \"aaaaaaaaaa\">".repeat(1000)
    );
    assert!(parse_with(&huge, &unlimited).is_ok());
}

#[test]
fn custom_entities_are_never_expanded() {
    // A declared custom entity must not be expanded; it survives verbatim as a
    // reference, so recursive/exponential expansion is structurally impossible.
    let input = concat!(
        "<!DOCTYPE adf [ <!ENTITY lol \"ha\"> ]>\n",
        "<adf><prospect><customer><contact>",
        "<name>&lol;</name>",
        "</contact></customer></prospect></adf>"
    );
    let doc = parse(input).expect("custom entity reference should parse without expansion");
    let value = doc.adf().prospects[0].customer.as_ref().unwrap().contacts[0].names[0].value();
    assert_eq!(value.as_ref(), "&lol;");
    assert_snapshot!(
        "custom_entities_are_never_expanded",
        doc.to_typed_string().unwrap()
    );
}

#[test]
fn external_entities_are_never_resolved() {
    // A SYSTEM (external) entity reference must not be fetched or resolved; it
    // is preserved as an unresolved reference (no XXE surface).
    let input = concat!(
        "<!DOCTYPE adf [ <!ENTITY xxe SYSTEM \"file:///etc/passwd\"> ]>\n",
        "<adf><prospect><customer><contact>",
        "<name>&xxe;</name>",
        "</contact></customer></prospect></adf>"
    );
    let doc = parse(input).expect("external entity reference should parse without resolution");
    let value = doc.adf().prospects[0].customer.as_ref().unwrap().contacts[0].names[0].value();
    assert_eq!(value.as_ref(), "&xxe;");
}
