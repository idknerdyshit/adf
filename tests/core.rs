use adf::{
    DEFAULT_MAX_DOCTYPE_LEN, Error, ParseOptions, Severity, TextPart, ValidationOptions, XmlNode,
    parse, parse_with,
};
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
    let value = doc.adf().prospects[0].customer.as_ref().unwrap().contacts[0].names[0].value();

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
    let value = doc.adf().prospects[0].customer.as_ref().unwrap().contacts[0].names[0].value();

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
        .set_value(Cow::Borrowed("Honda"));

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
}

#[test]
fn typed_writer_preserves_unknown_container_attributes() {
    let input = r#"<adf><prospect xmlns:p="urn:partner"><vehicle interest="buy" partner-id="x"><year>2024</year><make>Honda</make><model>Civic</model></vehicle><customer><contact partner="y"><name part="full">A</name><email>a@example.com</email></contact></customer></prospect></adf>"#;
    let doc = parse(input).expect("valid ADF should parse");
    let output = doc.to_typed_string().unwrap();

    assert!(output.contains(r#"xmlns:p="urn:partner""#));
    assert!(output.contains(r#"partner-id="x""#));
    assert!(output.contains(r#"partner="y""#));
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

    assert!(output.contains(r#"<vehicle interest="buy" partner-id="x">"#));
    assert!(output.contains(r#"<contact partner="y" primarycontact="1">"#));
}

#[test]
fn typed_writer_preserves_entity_refs() {
    let input = r#"<adf><prospect><customer><contact><name part="full">A</name><email>a@example.com</email></contact><comments>Jane &amp; &nbsp; Co</comments></customer></prospect></adf>"#;
    let doc = parse(input).expect("valid ADF should parse");
    let output = doc.to_typed_string().unwrap();

    assert!(output.contains("Jane &amp; &nbsp; Co"));
}

#[test]
fn typed_writer_preserves_cdata_wrapper() {
    let input = r#"<adf><prospect><customer><contact><name part="full">A</name><email>a@example.com</email></contact><comments><![CDATA[<b>hi</b>]]></comments></customer></prospect></adf>"#;
    let doc = parse(input).expect("valid ADF should parse");
    let output = doc.to_typed_string().unwrap();

    assert!(output.contains("<![CDATA[<b>hi</b>]]>"));
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

    assert!(output.contains("<comments><![CDATA[before]]]]><![CDATA[>after]]></comments>"));

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
    let doc = parse(r#"<adf><prospect><customer><contact /></customer></prospect></adf>"#)
        .expect("valid ADF should parse");

    let lenient = doc.validate();
    assert!(lenient.is_valid());

    let strict = doc.validate_strict();
    assert!(!strict.is_valid());
    assert!(
        strict
            .issues
            .iter()
            .any(|issue| issue.severity == Severity::Error && issue.message.contains("vehicle"))
    );
}

#[test]
fn validate_warns_on_bad_enum_values() {
    let doc = parse(r#"<adf><prospect status="weird"><vehicle interest="loan" status="brand-new"><year>2024</year><make>X</make><model>Y</model><price type="bizarre" currency="USD">1</price></vehicle><customer><contact><name part="full">A</name><email>a@example.com</email></contact></customer><vendor><vendorname>V</vendorname><contact><name part="full">B</name><email>b@example.com</email></contact></vendor></prospect></adf>"#).expect("valid ADF should parse");
    let report = doc.validate();

    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.path.contains("@status") && issue.message.contains("weird"))
    );
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.path.contains("@interest") && issue.message.contains("loan"))
    );
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.path.contains("price[0]@type") && issue.message.contains("bizarre"))
    );
}

#[test]
fn validate_warns_on_bad_iso_formats() {
    let doc = parse(r#"<adf><prospect><requestdate>not-a-date</requestdate><vehicle><year>2024</year><make>X</make><model>Y</model><price type="quote" currency="usd">1</price></vehicle><customer><contact><name part="full">A</name><email>a@example.com</email><address type="home"><country>USA</country></address></contact></customer><vendor><vendorname>V</vendorname><contact><name part="full">B</name><email>b@example.com</email></contact></vendor></prospect></adf>"#).expect("valid ADF should parse");
    let report = doc.validate();

    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.path.contains("requestdate") && issue.message.contains("ISO"))
    );
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.path.contains("@currency") && issue.message.contains("4217"))
    );
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.path.contains("country") && issue.message.contains("3166"))
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
    assert!(typed.contains("<pricecomment>special offer</pricecomment>"));
    assert!(!typed.contains("<pricecomments>"));
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
fn parser_borrows_element_names_for_ascii_input() {
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
fn default_parse_preserves_small_doctype() {
    let input = "<!DOCTYPE adf>\n<adf><prospect /></adf>";
    let doc = parse(input).expect("a small DOCTYPE should be preserved by default");
    assert!(matches!(
        doc.root().name,
        Cow::Borrowed("adf") | Cow::Owned(_)
    ));
    let out = doc.to_original_preserving_string().unwrap();
    assert!(out.contains("<!DOCTYPE adf>"));
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
    assert!(
        doc.to_typed_string()
            .unwrap()
            .contains("<name>&lol;</name>")
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
