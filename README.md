# adf

Lightweight Rust parsing and writing for Auto-lead Data Format (ADF) 1.0 XML leads.

The format is defined by the [ADF 1.0 specification](https://adfxml.info/adf_spec.pdf).

This crate is aimed at low-overhead ADF processing:

- parses XML with `quick-xml`
- borrows input text where possible through `Cow<'a, str>`
- exposes a typed ADF model for common lead fields
- keeps unknown XML elements and attributes — on containers and compact elements alike — instead of discarding partner data
- preserves CDATA wrappers and unknown text entity references through the typed writer
- can write the original document byte-for-byte when it has not been changed
- can rewrite only dirty prospect spans for localized edits
- keeps ADF-specific validation separate from XML parsing, with optional strict mode plus DTD enum and lightweight ISO-like format checks
- never resolves external entities or expands custom ones, and bounds (or rejects) `<!DOCTYPE>` declarations to keep untrusted input safe

## Installation

```toml
[dependencies]
adf = "0.5"
```

## Example

```rust
use adf::parse;
use std::borrow::Cow;

fn main() -> Result<(), adf::Error> {
    let input = r#"<adf>
      <prospect status="new">
        <requestdate>2026-05-17T12:00:00-04:00</requestdate>
        <vehicle><year>2024</year><make>Toyota</make><model>Camry</model></vehicle>
        <customer><contact><name part="full">Jane Doe</name><email>jane@example.com</email></contact></customer>
        <vendor>
          <vendorname>Example Dealer</vendorname>
          <contact><name part="full">Sales Team</name><phone>555-0100</phone></contact>
        </vendor>
      </prospect>
    </adf>"#;

    let mut doc = parse(input)?;

    // The complete example passes the crate's exact ADF 1.0 conformance profile.
    assert!(doc.validate_adf_1_0().is_valid());

    let prospect = &doc.adf().prospects[0];
    assert_eq!(prospect.status.as_deref(), Some("new"));

    doc.prospect_mut(0)
        .unwrap()
        .status = Some(Cow::Borrowed("resend"));

    let output = doc.to_original_preserving_string()?;
    assert!(output.contains(r#"<prospect status="resend">"#));

    Ok(())
}
```

## Generating ADF From Scratch

The builders require the minimum structure needed by the ADF 1.0 conformance
profile. Optional fields can be added with the remaining builder methods.

```rust
use adf::{
    Adf, Contact, ContactMethod, Customer, Name, Prospect, Vehicle, Vendor, parse, to_string,
};

fn main() -> Result<(), adf::Error> {
    let customer_contact = Contact::builder(
        Name::new("Jane Doe"),
        ContactMethod::Email("jane@example.com".into()),
    )
    .build();
    let vendor_contact = Contact::builder(
        Name::new("Sales Team"),
        ContactMethod::Phone("555-0100".into()),
    )
    .build();

    let adf = Adf::builder(
        Prospect::builder(
            "2026-05-17T12:00:00-04:00",
            Vehicle::builder("2024", "Toyota", "Camry").build(),
            Customer::builder(customer_contact).build(),
            Vendor::builder("Example Dealer", vendor_contact).build(),
        )
        .status("new")
        .build(),
    )
    .build();

    let xml = to_string(&adf)?;
    assert!(parse(&xml)?.validate_adf_1_0().is_valid());
    Ok(())
}
```

## Owned and Long-Lived Processing

`parse` borrows from its input where possible. Use `parse_owned` when ownership
is available up front, or call `into_owned` before a borrowed document outlives
its input scope.

```rust
use adf::{AdfDocument, parse, parse_owned};

fn parse_for_queue(xml: String) -> Result<AdfDocument<'static>, adf::Error> {
    parse_owned(xml)
}

fn copy_for_later(xml: &str) -> Result<AdfDocument<'static>, adf::Error> {
    Ok(parse(xml)?.into_owned())
}

fn main() -> Result<(), adf::Error> {
    let queued = parse_for_queue("<adf />".to_owned())?;
    let retained = copy_for_later("<adf />")?;
    assert_eq!(queued.original(), retained.original());
    Ok(())
}
```

## Inspecting and Mutating Extensions

Unknown partner elements are retained as `XmlNode` values. Mutating through
`prospect_mut` keeps the rewrite localized to that prospect.

```rust
use adf::{XmlNode, parse};
use std::borrow::Cow;

fn main() -> Result<(), adf::Error> {
    let input = r#"<adf><prospect><requestdate>2026-05-17T12:00:00-04:00</requestdate><vehicle><year>2024</year><make>Toyota</make><model>Camry</model><partner-score>97</partner-score></vehicle><customer><contact><name part="full">Jane Doe</name><email>jane@example.com</email></contact></customer><vendor><vendorname>Example Dealer</vendorname><contact><name part="full">Sales Team</name><phone>555-0100</phone></contact></vendor></prospect></adf>"#;
    let mut doc = parse(input)?;
    assert!(doc.validate_adf_1_0_extended().is_valid());

    let extensions = &mut doc.prospect_mut(0).unwrap().vehicles[0].extensions;
    let score = extensions.iter_mut().find_map(|node| match node {
        XmlNode::Element(element) if element.name == "partner-score" => Some(element),
        _ => None,
    });
    if let Some(score) = score {
        score.children = vec![XmlNode::Text(Cow::Borrowed("98"))];
    }

    let output = doc.to_original_preserving_string()?;
    assert!(output.contains("<partner-score>98</partner-score>"));
    Ok(())
}
```

## Writing Modes

`AdfDocument::to_original_preserving_string()` preserves the original XML when the document is clean. If a single prospect is modified through `prospect_mut`, only that prospect's original byte span is rewritten and the surrounding XML is copied through unchanged.

`AdfDocument::to_typed_string()` writes normalized ADF XML from the typed model. This is useful when broad structural edits are made through `adf_mut`, or when normalized output is preferred over preserving original formatting. Non-element extension nodes such as comments, processing instructions, CDATA, and custom entity references are preserved, but only element extensions carry source spans for relative ordering around typed children.

`AdfDocument::root()` exposes the raw XML tree for callers that need it. The tree is parsed lazily on first access so typed-only processing does not retain both the full raw tree and the typed ADF model.

## Parsing Safety

The parser never resolves external entities and never expands custom (DTD-defined) entities: only the five predefined XML entities and legal numeric character references are substituted. Unknown references in text are preserved as entity-reference parts. Unknown references in attributes are kept as literal `&name;` text; normalized typed output escapes those ampersands because attributes are modeled as flat strings. This makes classic XXE and entity-expansion ("billion laughs") attacks structurally impossible.

`parse` keeps `<!DOCTYPE>` declarations so partner documents round-trip, but caps the declaration payload at `DEFAULT_MAX_DOCTYPE_LEN` (4096 bytes) by default. Use `parse_with` and `ParseOptions` to tighten or relax this:

```rust
use adf::{parse_with, ParseOptions};

fn main() -> Result<(), adf::Error> {
    // Reject any document carrying a DOCTYPE.
    let strict = ParseOptions::default().reject_doctype(true);
    assert!(parse_with("<!DOCTYPE adf>\n<adf/>", &strict).is_err());

    // Or just adjust the size cap (use `without_doctype_limit()` to disable).
    let relaxed = ParseOptions::default().max_doctype_len(16 * 1024);
    parse_with("<adf><prospect /></adf>", &relaxed)?;

    Ok(())
}
```

## Lenient Parsing vs. Conformance Validation

Parsing requires well-formed XML rooted at `<adf>`, but deliberately accepts
incomplete or nonconforming ADF content. This keeps syntax handling separate
from application policy. ADF-specific content checks are available through
`AdfDocument::validate()`:

```rust
fn main() -> Result<(), adf::Error> {
    let report = adf::parse("<adf><prospect /></adf>")?.validate();

    for issue in report.issues {
        eprintln!("{:?}: {}: {}", issue.severity, issue.path, issue.message);
    }

    Ok(())
}
```

The default validator reports DTD-required elements as warnings, checks DTD enumerated attribute values (`prospect@status`, `vehicle@interest`, `price@type`, etc.), and warns on unsupported ISO 8601 datetime shapes plus malformed ISO 4217/ISO 3166 code shapes. It does not attempt full registry validation for every currency or country code.

Choose the validation entry point that matches the boundary being enforced:

- `validate()` reports missing required structure and other structural concerns
  without rejecting the document.
- `validate_strict()` promotes missing required structure to errors, but is not
  the complete ADF 1.0 conformance profile. Enum and lightweight format issues
  remain warnings.
- `validate_adf_1_0()` enforces the complete modeled ADF 1.0 content model,
  ordering, cardinality, attributes, enums, formats, and ranges, and rejects
  partner extensions.
- `validate_adf_1_0_extended()` performs the same conformance checks while
  permitting partner extension elements and attributes.

## Limitations and Interoperability

- Input must be UTF-8. The byte and reader APIs validate UTF-8 but do not
  transcode other XML encodings.
- ADF integrations commonly carry partner-specific elements and attributes.
  They are preserved, but callers should use the extended conformance profile
  and verify partner requirements independently.
- Clean original-preserving writes are byte-for-byte, and `prospect_mut` limits
  rewrites to dirty prospect spans. Broad edits through `adf_mut` use normalized
  typed output, which is not intended to reproduce every lexical detail of the
  source document.
- Custom DTD entities are retained rather than expanded. Unknown references in
  flat attribute values are escaped during normalized output.
- Currency and country checks use bundled registries, while date validation and
  some other checks enforce the ADF-defined lexical shapes. Validation does not
  establish that a lead will satisfy every receiving system's business rules.
- MIME or SMTP extraction, message transport, delivery, retries, and mailbox
  processing are outside this crate's scope and belong in the application.

## Logging and Tracing

`adf` emits passive `tracing` spans and events for parse, validation, and write operations. The crate does not install a subscriber; applications decide whether and how to collect those events.

Trace fields are limited to structural metadata such as byte counts, parse options, model counts, dirty flags, validation issue counts, and error categories/positions. They intentionally do not include raw XML, element text, attribute values, validation messages, names, emails, phone numbers, addresses, identifiers, URLs, comments, or extension payloads.

The public model and `AdfDocument::original()` still expose lead payloads; avoid logging those values directly when handling sensitive data.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Development

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
```

### Parsing benchmarks

The Criterion suite measures borrowed parsing, owned input paths, lazy raw-tree
construction, and a diagnostic `quick-xml` event scan across fabricated ADF
documents ranging from an empty root to a 1,000-prospect batch. The tokenizer
scan is a lower-level diagnostic, not a semantically equivalent ADF parse.

Run the complete suite or filter it to one benchmark group:

```sh
cargo bench --bench parsing
cargo bench --bench parsing -- borrowed_parse
cargo bench --bench parsing -- input_ownership
cargo bench --bench parsing -- raw_tree
cargo bench --bench parsing -- tokenizer_floor
```

Compile the suite without collecting timing measurements with:

```sh
cargo bench --bench parsing --no-run
```
