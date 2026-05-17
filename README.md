# adf

Lightweight Rust parsing and writing for ADF 1.0 XML leads.

This crate is aimed at low-overhead ADF processing:

- parses XML with `quick-xml`
- borrows input text where possible through `Cow<'a, str>`
- exposes a typed ADF model for common lead fields
- keeps unknown XML elements and attributes instead of discarding partner data
- can write the original document byte-for-byte when it has not been changed
- can rewrite only dirty prospect spans for localized edits
- keeps ADF-specific validation separate from XML parsing

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
        .status = Some(Cow::Borrowed("contacted"));

    let output = doc.to_original_preserving_string()?;
    assert!(output.contains(r#"<prospect status="contacted">"#));

    Ok(())
}
```

## Writing Modes

`AdfDocument::to_original_preserving_string()` preserves the original XML when the document is clean. If a single prospect is modified through `prospect_mut`, only that prospect's original byte span is rewritten and the surrounding XML is copied through unchanged.

`AdfDocument::to_typed_string()` writes normalized ADF XML from the typed model. This is useful when broad structural edits are made through `adf_mut`, or when normalized output is preferred over preserving original formatting.

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

The current validator focuses on structural warnings and errors for the supported model surface.

## Development

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
```
