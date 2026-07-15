use adf::{
    Address, Adf, AdfDocument, Attribute, ColorCombination, ColorSelection, Contact, ContactMethod,
    Customer, DEFAULT_MAX_DOCTYPE_LEN, Error, Finance, Id, Name, ParseLimit, ParseOptions, Price,
    Prospect, Provider, Severity, Span, TextElement, TextPart, Timeframe, TimeframeWindow,
    UnknownEntityPolicy, ValidationCode, ValidationOptions, ValidationProfile, ValidationReport,
    Vehicle, VehicleOption, Vendor, WriteOptions, XmlElement, XmlNode, parse, parse_bytes,
    parse_bytes_with, parse_owned, parse_reader, parse_with, to_string, to_string_with,
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
fn rejects_non_adf_root() {
    assert!(matches!(
        parse("<not-adf />"),
        Err(Error::UnexpectedRoot { found, .. }) if found == "not-adf"
    ));
}

#[test]
fn rejects_invalid_xml_comments() {
    assert!(parse("<adf><!--bad--comment--><prospect /></adf>").is_err());
    assert!(
        parse("<adf><prospect><customer><contact><name><!--bad--comment--></name></contact></customer></prospect></adf>")
            .is_err()
    );
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
fn rejects_xml_illegal_character_references_and_text() {
    let nul_ref = r#"<adf><prospect><customer><contact><name>&#0;</name></contact></customer></prospect></adf>"#;
    assert!(matches!(
        parse(nul_ref),
        Err(Error::InvalidCharacterReference { .. })
    ));

    let direct_control = format!(
        "<adf><prospect><customer><contact><name>A{}B</name></contact></customer></prospect></adf>",
        char::from_u32(1).unwrap()
    );
    assert!(matches!(
        parse(&direct_control),
        Err(Error::IllegalCharacter { .. })
    ));
}

#[test]
fn rejects_malformed_entity_reference_names() {
    let bad_text = r#"<adf><prospect><customer><contact><name>&bad ref;</name></contact></customer></prospect></adf>"#;
    assert!(matches!(
        parse(bad_text),
        Err(Error::InvalidEntityReference { .. })
    ));

    let empty_text = r#"<adf><prospect><customer><contact><name>&;</name></contact></customer></prospect></adf>"#;
    assert!(matches!(
        parse(empty_text),
        Err(Error::InvalidEntityReference { .. })
    ));

    let bad_attr = r#"<adf><prospect status="&bad ref;" /></adf>"#;
    assert!(matches!(
        parse(bad_attr),
        Err(Error::InvalidEntityReference { .. })
    ));
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

    assert!(output.contains("<!-- keep me -->"));
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
    assert!(matches!(
        doc.to_typed_string(),
        Err(Error::UndeclaredEntityReference { .. })
    ));
    let output = doc
        .to_typed_string_with(
            &WriteOptions::default().unknown_entity_policy(UnknownEntityPolicy::Preserve),
        )
        .unwrap();

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
fn typed_writer_preserves_non_element_container_extensions() {
    let input = r#"<adf><prospect><!--prospect note--><?partner keep?><vehicle><year>2024</year><!--vehicle note--><make>Honda</make><model>Civic</model></vehicle><customer><contact><name part="full">A</name><email>a@example.com</email></contact></customer></prospect></adf>"#;
    let mut doc = parse(input).expect("valid ADF should parse");

    let typed = doc.to_typed_string().unwrap();
    assert!(typed.contains("<!--prospect note-->"));
    assert!(typed.contains("<?partner keep?>"));
    assert!(typed.contains("<!--vehicle note-->"));
    parse(&typed).expect("typed XML should reparse");

    doc.prospect_mut(0).unwrap().status = Some(Cow::Borrowed("resend"));
    let localized = doc.to_original_preserving_string().unwrap();
    assert!(localized.contains("<!--prospect note-->"));
    assert!(localized.contains("<?partner keep?>"));
    assert!(localized.contains("<!--vehicle note-->"));
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
fn typed_writer_rejects_invalid_public_xml_tokens() {
    let mut doc = parse("<adf><prospect /></adf>").expect("valid ADF should parse");
    doc.adf_mut()
        .extensions
        .push(XmlNode::Comment(Cow::Borrowed("bad--comment")));
    assert!(matches!(
        doc.to_typed_string(),
        Err(Error::InvalidXmlToken {
            kind: "comment",
            ..
        })
    ));

    let mut doc = parse("<adf><prospect /></adf>").expect("valid ADF should parse");
    doc.adf_mut()
        .extensions
        .push(XmlNode::EntityRef(Cow::Borrowed("bad ref")));
    assert!(matches!(
        doc.to_typed_string(),
        Err(Error::InvalidEntityReference { .. })
    ));

    let mut doc = parse("<adf><prospect /></adf>").expect("valid ADF should parse");
    doc.adf_mut().extensions.push(XmlNode::Element(XmlElement {
        name: Cow::Borrowed("bad name"),
        attributes: Vec::new(),
        children: Vec::new(),
        span: Span::default(),
    }));
    assert!(matches!(
        doc.to_typed_string(),
        Err(Error::InvalidName { kind: "element" })
    ));

    let mut doc = parse("<adf><prospect /></adf>").expect("valid ADF should parse");
    doc.adf_mut().attributes.push(Attribute {
        name: Cow::Borrowed("bad attr"),
        value: Cow::Borrowed("value"),
    });
    assert!(matches!(
        doc.to_typed_string(),
        Err(Error::InvalidName { kind: "attribute" })
    ));

    let model = Adf::builder(Prospect::default()).build();
    assert!(matches!(
        to_string_with(&model, &WriteOptions::default().doctype("adf><injected/"),),
        Err(Error::InvalidXmlToken {
            kind: "DOCTYPE",
            ..
        })
    ));
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
fn validate_strict_keeps_bad_enum_values_as_warnings() {
    let input = r#"<adf><prospect status="weird"><requestdate>2024-01-02T03:04:05-05:00</requestdate><vehicle interest="loan" status="brand-new"><year>2024</year><make>X</make><model>Y</model><price type="bizarre" currency="USD">1</price></vehicle><customer><contact><name part="full">A</name><email>a@example.com</email></contact></customer><vendor><vendorname>V</vendorname><contact><name part="full">B</name><email>b@example.com</email></contact></vendor></prospect></adf>"#;
    let doc = parse(input).expect("valid ADF should parse");
    let report = doc.validate_strict();

    assert!(report.is_valid());
    assert!(
        report
            .issues
            .iter()
            .all(|issue| issue.severity == Severity::Warning)
    );
    assert_snapshot!(
        "validate_strict_keeps_bad_enum_values_as_warnings",
        normalized_issues(input, &report)
    );
}

#[test]
fn validate_strict_keeps_bad_iso_shapes_as_warnings() {
    let input = r#"<adf><prospect><requestdate>9999-99-99T99:99:99</requestdate><vehicle><year>2024</year><make>X</make><model>Y</model><price type="quote" currency="usd">1</price></vehicle><customer><contact><name part="full">A</name><email>a@example.com</email><address type="home"><country>USA</country></address></contact></customer><vendor><vendorname>V</vendorname><contact><name part="full">B</name><email>b@example.com</email></contact></vendor></prospect></adf>"#;
    let doc = parse(input).expect("valid ADF should parse");
    let report = doc.validate_strict();

    assert!(report.is_valid());
    assert!(
        report
            .issues
            .iter()
            .all(|issue| issue.severity == Severity::Warning)
    );
    assert_snapshot!(
        "validate_strict_keeps_bad_iso_shapes_as_warnings",
        normalized_issues(input, &report)
    );
}

#[test]
fn validate_checks_provider_contacts_and_direct_contact_fields() {
    let input = r#"<adf><prospect><requestdate>2024-01-02T03:04:05-05:00</requestdate><vehicle><year>2024</year><make>X</make><model>Y</model></vehicle><customer><contact><name part="full">A</name><email>a@example.com</email></contact></customer><vendor><vendorname>V</vendorname><contact><name part="full">B</name><email>b@example.com</email></contact></vendor><provider><email preferredcontact="yes">p@example.com</email><phone type="sms" time="never" preferredcontact="maybe">555</phone><contact primarycontact="yes"><phone type="sms">555</phone></contact></provider></prospect></adf>"#;
    let doc = parse(input).expect("valid ADF should parse");
    let report = doc.validate_strict();

    assert_snapshot!(
        "validate_checks_provider_contacts_and_direct_contact_fields",
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
fn empty_adf_uses_lenient_and_strict_severity() {
    let input = "<adf></adf>";
    let doc = parse(input).expect("valid ADF should parse");

    let lenient = doc.validate();
    assert!(lenient.is_valid());
    assert_eq!(lenient.issues[0].severity, Severity::Warning);

    let strict = doc.validate_strict();
    assert!(!strict.is_valid());
    assert_eq!(strict.issues[0].severity, Severity::Error);
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
    // A DOCTYPE payload large enough to exceed the default cap, e.g. a
    // billion-laughs style entity definition block in an internal subset.
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

    // A tiny cap rejects an otherwise harmless DOCTYPE payload.
    let tight = ParseOptions::default().max_doctype_len(4);
    assert!(matches!(
        parse_with(input, &tight),
        Err(Error::DocTypeTooLong { limit: 4, .. })
    ));

    // Disabling the cap accepts an arbitrarily large DOCTYPE payload.
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
fn custom_entities_in_attributes_are_preserved_as_literal_text() {
    let input = concat!(
        "<!DOCTYPE adf [ <!ENTITY custom \"resend\"> ]>\n",
        "<adf><prospect status=\"&custom;\" /></adf>"
    );
    let doc = parse(input).expect("custom attribute entity should parse without expansion");

    assert_eq!(doc.adf().prospects[0].status.as_deref(), Some("&custom;"));
    assert_eq!(doc.to_original_preserving_string().unwrap(), input);

    let typed = doc.to_typed_string().unwrap();
    assert!(
        typed.contains(r#"status="&amp;custom;""#),
        "typed writer should escape the literal preserved reference: {typed}"
    );
    let reparsed = parse(&typed).expect("typed XML should reparse");
    assert_eq!(
        reparsed.adf().prospects[0].status.as_deref(),
        Some("&custom;")
    );
}

fn built_adf() -> Adf<'static> {
    let customer_contact = Contact::builder(
        Name::builder("Jane Doe".to_owned())
            .part("full")
            .name_type("individual")
            .build(),
        ContactMethod::Email("jane@example.test".to_owned().into()),
    )
    .primary_contact(true)
    .phone(
        TextElement::builder("555-0100".to_owned())
            .attribute("type", "voice")
            .attribute("time", "day")
            .build(),
    )
    .address(
        Address::builder(
            TextElement::builder("1 Main St".to_owned())
                .attribute("line", "1")
                .build(),
        )
        .address_type("home")
        .city("Detroit".to_owned())
        .region_code("MI".to_owned())
        .postal_code("48201".to_owned())
        .country("US".to_owned())
        .build(),
    )
    .build();
    let vendor_contact = Contact::builder(
        Name::builder("Sales".to_owned()).part("full").build(),
        ContactMethod::Phone("555-0199".to_owned().into()),
    )
    .build();
    let provider_contact = Contact::builder(
        Name::builder("Support".to_owned()).part("full").build(),
        ContactMethod::Email("support@example.test".to_owned().into()),
    )
    .build();

    let option = VehicleOption::builder("Sunroof".to_owned(), "100".to_owned())
        .manufacturer_code("SUN".to_owned())
        .stock("available".to_owned())
        .price(
            Price::builder("900")
                .price_type("msrp")
                .currency("USD")
                .build(),
        )
        .build();
    let finance = Finance::builder(
        "finance".to_owned(),
        TextElement::builder("1000".to_owned())
            .attribute("type", "downpayment")
            .attribute("limit", "minimum")
            .attribute("currency", "USD")
            .build(),
    )
    .balance(
        TextElement::builder("24000".to_owned())
            .attribute("type", "finance")
            .attribute("currency", "USD")
            .build(),
    )
    .build();
    let vehicle = Vehicle::builder("2026".to_owned(), "Honda".to_owned(), "Civic".to_owned())
        .id(Id::builder("vehicle-1", "inventory").sequence("1").build())
        .interest("buy")
        .status("new")
        .vin("1HGCM82633A004352".to_owned())
        .stock("STK-1".to_owned())
        .trim("Touring".to_owned())
        .doors("4".to_owned())
        .body_style("Sedan".to_owned())
        .transmission("Automatic".to_owned())
        .odometer(
            TextElement::builder("100".to_owned())
                .attribute("status", "original")
                .attribute("units", "mi")
                .build(),
        )
        .condition("excellent".to_owned())
        .color_combination(
            ColorCombination::builder(
                ColorSelection::Both {
                    interior: "Black".to_owned().into(),
                    exterior: "Blue".to_owned().into(),
                },
                "1".to_owned(),
            )
            .build(),
        )
        .image_tag(
            TextElement::builder("https://example.test/car.jpg".to_owned())
                .attribute("width", "640")
                .attribute("height", "480")
                .attribute("alttext", "car")
                .build(),
        )
        .price(
            Price::builder("25000")
                .price_type("quote")
                .currency("USD")
                .source("dealer")
                .build(),
        )
        .price_comments("Plus taxes".to_owned())
        .option(option)
        .finance(finance)
        .comments("Vehicle comment".to_owned())
        .build();
    let customer = Customer::builder(customer_contact)
        .id(Id::new("customer-1", "crm"))
        .timeframe(
            Timeframe::builder(TimeframeWindow::Range {
                earliest: "20260715T120000-0400".to_owned().into(),
                latest: "2026-08-15T12:00:00-04:00".to_owned().into(),
            })
            .description("Within a month".to_owned())
            .build(),
        )
        .comments("Customer comment".to_owned())
        .build();
    let vendor = Vendor::builder("Example Motors".to_owned(), vendor_contact)
        .id(Id::new("vendor-1", "directory"))
        .url("https://dealer.example".to_owned())
        .build();
    let provider = Provider::builder(
        Name::builder("Lead Provider".to_owned())
            .part("full")
            .name_type("business")
            .build(),
    )
    .id(Id::new("provider-1", "provider"))
    .service("Internet leads".to_owned())
    .url("https://provider.example".to_owned())
    .email("provider@example.test".to_owned())
    .phone("555-0188".to_owned())
    .contact(provider_contact)
    .build();
    let prospect = Prospect::builder(
        "2026-07-15T12:00:00-04:00".to_owned(),
        vehicle,
        customer,
        vendor,
    )
    .status("new")
    .id(Id::builder("lead-1", "provider").sequence("1").build())
    .provider(provider)
    .build();
    Adf::builder(prospect).build()
}

#[test]
fn builders_write_reparse_and_conform() {
    let minimal = Adf::builder(
        Prospect::builder(
            "2026-07-15T12:00:00-04:00".to_owned(),
            Vehicle::builder("2026".to_owned(), "Honda".to_owned(), "Civic".to_owned()).build(),
            Customer::builder(
                Contact::builder(
                    Name::new("Jane"),
                    ContactMethod::Email("jane@example.test".to_owned().into()),
                )
                .build(),
            )
            .build(),
            Vendor::builder(
                "Dealer".to_owned(),
                Contact::builder(
                    Name::new("Sales"),
                    ContactMethod::Phone("555-0100".to_owned().into()),
                )
                .build(),
            )
            .build(),
        )
        .build(),
    )
    .build();
    let minimal_xml = to_string(&minimal).expect("minimal model should write");
    assert!(parse(&minimal_xml).unwrap().validate_adf_1_0().is_valid());

    let model = built_adf();
    for profile in [ValidationProfile::Adf10, ValidationProfile::Adf10Extended] {
        let report = adf::validate_with(&model, ValidationOptions::default().profile(profile));
        assert!(report.is_valid(), "{profile:?}: {:#?}", report.issues);
    }

    let xml = to_string(&model).expect("constructed model should write");
    assert!(xml.starts_with("<?xml version=\"1.0\"?>\n<?adf version=\"1.0\"?>\n<adf>"));
    let reparsed = parse(&xml).expect("constructed output should reparse");
    assert!(reparsed.validate_adf_1_0().is_valid());
    assert!(reparsed.validate_adf_1_0_extended().is_valid());
}

#[test]
fn exact_and_extended_profiles_differ_only_on_partner_content() {
    let input = r#"<adf partner="x"><prospect><requestdate>2026-07-15T12:00:00-04:00</requestdate><vehicle><year>2026</year><make>Honda</make><model>Civic</model><partner-data>ok</partner-data></vehicle><customer><contact><name part="full">Jane</name><email>jane@example.test</email></contact></customer><vendor><vendorname>Dealer</vendorname><contact><name part="full">Sales</name><phone>555-0100</phone></contact></vendor></prospect></adf>"#;
    let document = parse(input).unwrap();
    let exact = document.validate_adf_1_0();
    let extended = document.validate_adf_1_0_extended();
    assert!(!exact.is_valid());
    assert!(exact.issues.iter().any(|issue| matches!(
        issue.code,
        ValidationCode::UnexpectedAttribute | ValidationCode::UnexpectedElement
    )));
    assert!(extended.is_valid(), "{:#?}", extended.issues);

    let mixed = input.replace("<year>2026</year>", "<year>20<partner/>26</year>");
    let mixed = parse(&mixed).unwrap();
    assert!(
        mixed
            .validate_adf_1_0()
            .issues
            .iter()
            .any(|issue| issue.code == ValidationCode::UnexpectedElement)
    );
    assert!(mixed.validate_adf_1_0_extended().is_valid());
}

#[test]
fn conformance_reports_duplicate_order_enum_format_and_range_codes() {
    let input = r#"<adf><prospect status="again"><requestdate>2026-07-15T12:00:00Z</requestdate><requestdate>2026-07-15T12:00:00-04:00</requestdate><vehicle><make>Honda</make><year>2026</year><model>Civic</model><option><optionname>x</optionname><weighting>101</weighting></option></vehicle><customer><id source="crm">1</id><contact><name part="full">Jane</name><email>jane@example.test</email></contact></customer><vendor><vendorname>Dealer</vendorname><contact><name part="full">Sales</name><phone>555-0100</phone></contact></vendor></prospect></adf>"#;
    let report = parse(input).unwrap().validate_adf_1_0();
    for expected in [
        ValidationCode::Duplicate,
        ValidationCode::OutOfOrder,
        ValidationCode::InvalidEnum,
        ValidationCode::InvalidFormat,
        ValidationCode::InvalidRange,
    ] {
        assert!(
            report.issues.iter().any(|issue| issue.code == expected),
            "missing {expected:?}: {:#?}",
            report.issues
        );
    }
}

#[test]
fn conformance_codes_cover_required_excessive_registry_and_placement_rules() {
    let cases = [
        (
            "missing required",
            r#"<adf><prospect/></adf>"#,
            ValidationCode::MissingRequired,
        ),
        (
            "excessive email",
            r#"<adf><prospect><requestdate>2026-07-15T12:00:00-04:00</requestdate><vehicle><year>2026</year><make>Honda</make><model>Civic</model></vehicle><customer><contact><name>Jane</name><email>a@example.test</email><email>b@example.test</email></contact></customer><vendor><vendorname>Dealer</vendorname><contact><name>Sales</name><phone>555-0100</phone></contact></vendor></prospect></adf>"#,
            ValidationCode::Excessive,
        ),
        (
            "inactive currency",
            r#"<adf><prospect><requestdate>2026-07-15T12:00:00-04:00</requestdate><vehicle><year>2026</year><make>Honda</make><model>Civic</model><price currency="CUC">1</price></vehicle><customer><contact><name>Jane</name><email>a@example.test</email></contact></customer><vendor><vendorname>Dealer</vendorname><contact><name>Sales</name><phone>555-0100</phone></contact></vendor></prospect></adf>"#,
            ValidationCode::InvalidFormat,
        ),
        (
            "unassigned country",
            r#"<adf><prospect><requestdate>2026-07-15T12:00:00-04:00</requestdate><vehicle><year>2026</year><make>Honda</make><model>Civic</model></vehicle><customer><contact><name>Jane</name><email>a@example.test</email><address><street>1 Main</street><country>ZZ</country></address></contact></customer><vendor><vendorname>Dealer</vendorname><contact><name>Sales</name><phone>555-0100</phone></contact></vendor></prospect></adf>"#,
            ValidationCode::InvalidFormat,
        ),
    ];
    for (name, input, expected) in cases {
        let report = parse(input).unwrap().validate_adf_1_0();
        assert!(
            report.issues.iter().any(|issue| issue.code == expected),
            "{name} did not emit {expected:?}: {:#?}",
            report.issues
        );
    }

    let misplaced = r#"<adf><prospect><requestdate>2026-07-15T12:00:00-04:00</requestdate><vehicle><year>2026</year><make>Honda</make><model>Civic</model></vehicle><customer status="new"><contact><name>Jane</name><email>a@example.test</email></contact></customer><vendor><vendorname>Dealer</vendorname><contact><name>Sales</name><phone>555-0100</phone></contact></vendor></prospect></adf>"#;
    let report = parse(misplaced).unwrap().validate_adf_1_0_extended();
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == ValidationCode::UnexpectedAttribute)
    );
}

#[test]
fn comments_and_processing_instructions_do_not_affect_content_order() {
    let input = r#"<adf><!--a--><prospect><?partner x?><requestdate>2026-07-15T12:00:00-04:00</requestdate><!--b--><vehicle><year>2026</year><make>Honda</make><model>Civic</model></vehicle><customer><contact><name>Jane</name><email>a@example.test</email></contact></customer><vendor><vendorname>Dealer</vendorname><contact><name>Sales</name><phone>555-0100</phone></contact></vendor></prospect></adf>"#;
    let report = parse(input).unwrap().validate_adf_1_0();
    assert!(report.is_valid(), "{:#?}", report.issues);
}

#[test]
fn duplicate_singular_elements_survive_dirty_rewrites() {
    let input = r#"<adf><prospect><requestdate>2026-07-15T12:00:00-04:00</requestdate><requestdate>2026-07-16T12:00:00-04:00</requestdate><vehicle><year>2026</year><make>Honda</make><model>Civic</model></vehicle><customer><contact><name part="full">Jane</name><email>jane@example.test</email></contact></customer><vendor><vendorname>Dealer</vendorname><contact><name part="full">Sales</name><phone>555-0100</phone></contact></vendor></prospect></adf>"#;
    let mut document = parse(input).unwrap();
    assert_eq!(
        document.adf().prospects[0]
            .request_date
            .as_ref()
            .unwrap()
            .value(),
        "2026-07-15T12:00:00-04:00"
    );
    assert!(
        document
            .validate_adf_1_0()
            .issues
            .iter()
            .any(|issue| issue.code == ValidationCode::Duplicate)
    );
    assert_eq!(document.to_original_preserving_string().unwrap(), input);
    document.prospect_mut(0).unwrap().status = Some(Cow::Borrowed("new"));
    let rewritten = document.to_original_preserving_string().unwrap();
    assert_eq!(rewritten.matches("<requestdate>").count(), 2);
}

#[test]
fn owned_ingestion_and_conversion_cover_string_bytes_and_reader() {
    let input = "<adf><prospect/></adf>".to_owned();
    let owned = parse_owned(input.clone()).unwrap();
    assert_eq!(owned.original(), input);
    let typed: Adf<'static> = owned.into_adf();
    assert_eq!(typed.prospects.len(), 1);

    assert_eq!(parse_bytes(input.as_bytes()).unwrap().original(), input);
    assert_eq!(
        parse_reader(std::io::Cursor::new(input.as_bytes()))
            .unwrap()
            .original(),
        input
    );

    let converted: AdfDocument<'static> = {
        let temporary = "<adf><prospect/></adf>".to_owned();
        parse(&temporary).unwrap().into_owned()
    };
    assert_eq!(converted.original(), input);
}

#[test]
fn parsing_limits_are_enforced_at_boundaries() {
    let input = "<adf/>";
    assert!(parse_with(input, &ParseOptions::default().max_input_len(input.len())).is_ok());
    assert!(matches!(
        parse_with(input, &ParseOptions::default().max_input_len(input.len() - 1)),
        Err(Error::LimitExceeded {
            limit: ParseLimit::InputLength,
            maximum,
            actual,
            ..
        }) if maximum == input.len() - 1 && actual == input.len()
    ));
    assert!(matches!(
        parse_with("<adf><x/></adf>", &ParseOptions::default().max_depth(1)),
        Err(Error::LimitExceeded {
            limit: ParseLimit::Depth,
            ..
        })
    ));
    assert!(matches!(
        parse_with(
            "<adf a=\"1\"/>",
            &ParseOptions::default().max_attributes_per_element(0)
        ),
        Err(Error::LimitExceeded {
            limit: ParseLimit::AttributesPerElement,
            ..
        })
    ));
    assert!(matches!(
        parse_with("<!--x--><adf/>", &ParseOptions::default().max_nodes(1)),
        Err(Error::LimitExceeded {
            limit: ParseLimit::Nodes,
            ..
        })
    ));

    assert!(matches!(
        parse_bytes(&[0xff]),
        Err(Error::Utf8 { position: 0, .. })
    ));
    assert!(matches!(
        parse_bytes(br#"<?xml version="1.0" encoding="ISO-8859-1"?><adf/>"#),
        Err(Error::UnsupportedEncoding { .. })
    ));
    assert!(matches!(
        parse_bytes_with(input.as_bytes(), &ParseOptions::default().max_input_len(1)),
        Err(Error::LimitExceeded {
            limit: ParseLimit::InputLength,
            ..
        })
    ));
}

#[test]
fn normalized_writing_preserves_document_misc_and_applies_entity_policies() {
    let input = concat!(
        "<?xml version=\"1.0\"?>\n",
        "<?adf version=\"1.0\"?>\n",
        "<!--before--><?partner mode=\"x\"?>",
        "<!DOCTYPE adf [<!ENTITY nbsp \"&#160;\">]>",
        "<adf><prospect><customer><comments>&nbsp;</comments></customer></prospect></adf>",
        "<!--after-->"
    );
    let document = parse(input).unwrap();
    let normalized = document.to_typed_string().unwrap();
    for expected in [
        "<!--before-->",
        "<?partner mode=\"x\"?>",
        "<!DOCTYPE adf",
        "&nbsp;",
        "<!--after-->",
    ] {
        assert!(
            normalized.contains(expected),
            "missing {expected}: {normalized}"
        );
    }

    let model = document.adf().clone();
    assert!(matches!(
        to_string(&model),
        Err(Error::UndeclaredEntityReference { .. })
    ));
    let escaped = to_string_with(
        &model,
        &WriteOptions::default().unknown_entity_policy(UnknownEntityPolicy::Escape),
    )
    .unwrap();
    assert!(escaped.contains("&amp;nbsp;"));
    let preserved = to_string_with(
        &model,
        &WriteOptions::default().doctype("adf [<!ENTITY nbsp \"&#160;\">]"),
    )
    .unwrap();
    assert!(preserved.contains("<!DOCTYPE adf [<!ENTITY nbsp \"&#160;\">]>"));
    assert!(preserved.contains("&nbsp;"));

    for doctype in [
        "<!DOCTYPE adf>",
        "<!DOCTYPE adf [<!-- <!ENTITY nbsp \"fake\"> -->]>",
        "<!DOCTYPE adf [<?partner <!ENTITY nbsp \"fake\"> ?>]>",
    ] {
        let input = format!(
            "{doctype}<adf><prospect><customer><comments>&nbsp;</comments></customer></prospect></adf>"
        );
        let document = parse(&input).unwrap();
        assert!(matches!(
            document.to_typed_string(),
            Err(Error::UndeclaredEntityReference { ref name }) if name == "nbsp"
        ));

        let mut document = document;
        document.adf_mut();
        assert!(matches!(
            document.to_original_preserving_string(),
            Err(Error::UndeclaredEntityReference { ref name }) if name == "nbsp"
        ));
    }
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
