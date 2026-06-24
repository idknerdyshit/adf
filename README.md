# adf

Lightweight Rust parsing and writing for Auto-lead Data Format (ADF) 1.0 XML leads.

This crate is aimed at low-overhead ADF processing:

- parses XML with `quick-xml`
- borrows input text where possible through `Cow<'a, str>`
- exposes a typed ADF model for common lead fields
- keeps unknown XML elements and attributes — on containers and compact elements alike — instead of discarding partner data
- preserves CDATA wrappers and unknown entity references through the typed writer
- can write the original document byte-for-byte when it has not been changed
- can rewrite only dirty prospect spans for localized edits
- keeps ADF-specific validation separate from XML parsing, with optional strict mode plus DTD enum and ISO format checks
- never resolves external entities or expands custom ones, and bounds (or rejects) `<!DOCTYPE>` declarations to keep untrusted input safe

## Installation

```toml
[dependencies]
adf = "0.4"
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
        <vendor><vendorname>Example Dealer</vendorname></vendor>
      </prospect>
    </adf>"#;

    let mut doc = parse(input)?;

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

## Writing Modes

`AdfDocument::to_original_preserving_string()` preserves the original XML when the document is clean. If a single prospect is modified through `prospect_mut`, only that prospect's original byte span is rewritten and the surrounding XML is copied through unchanged.

`AdfDocument::to_typed_string()` writes normalized ADF XML from the typed model. This is useful when broad structural edits are made through `adf_mut`, or when normalized output is preferred over preserving original formatting.

`AdfDocument::root()` exposes the raw XML tree for callers that need it. The tree is parsed lazily on first access so typed-only processing does not retain both the full raw tree and the typed ADF model.

## Parsing Safety

The parser never resolves external entities and never expands custom (DTD-defined) entities: only the five predefined XML entities and numeric character references are substituted, and any other reference is preserved verbatim. This makes classic XXE and entity-expansion ("billion laughs") attacks structurally impossible.

`parse` keeps `<!DOCTYPE>` declarations so partner documents round-trip, but caps the internal subset at `DEFAULT_MAX_DOCTYPE_LEN` (4096 bytes) by default. Use `parse_with` and `ParseOptions` to tighten or relax this:

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

## Validation

Parsing only requires well-formed XML. ADF-specific checks are available through `AdfDocument::validate()`:

```rust
fn main() -> Result<(), adf::Error> {
    let report = adf::parse("<adf><prospect /></adf>")?.validate();

    for issue in report.issues {
        eprintln!("{:?}: {}: {}", issue.severity, issue.path, issue.message);
    }

    Ok(())
}
```

The default validator reports DTD-required elements as warnings, checks DTD enumerated attribute values (`prospect@status`, `vehicle@interest`, `price@type`, etc.), and warns on malformed ISO 8601 dates, ISO 4217 currency codes, and ISO 3166 country codes.

`AdfDocument::validate_strict()` (or `validate_with(adf, ValidationOptions::default().strict(true))`) promotes the "missing required element" warnings to errors, suitable for gating on conformance.

## License

Licensed under either of:

- Apache License, Version 2.0
- MIT license

at your option.

## Development

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
```
