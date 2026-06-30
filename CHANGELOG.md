# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Fixed

- Reject XML-illegal characters and invalid comments during parsing.
- Reject XML documents whose root element is not `<adf>`.
- Preserve unknown entity references in attribute values as literal text instead of rejecting them.
- Preserve non-element extension nodes during typed conversion instead of dropping them.
- Keep lenient validation warnings lenient for empty `<adf>` documents.
- Validate provider contacts and direct provider email/phone attributes.
- Clarify that currency/country validation is a shape check, not full registry validation.
- Clarify and test that strict validation only promotes missing required structure.
- Avoid quadratic extension-order key construction in typed writing.
- Validate caller-constructed raw XML tokens before typed writing.

## [0.4.1] - 2026-06-28

### Added

- Redacted `tracing` instrumentation for parsing, validation, writing, and dirty-state transitions. (9e9cfb9)

## [0.4.0] - 2026-06-24

### Added

- Root `<adf>` attributes now round-trip through the typed writer, preserving document-level namespace declarations required by vendor extensions.
- Text-like ADF elements now retain embedded XML nodes in `TextPart::Node`, so partner markup inside fields such as `<comments>` is not dropped by normalized output.
- Snapshot-based regression tests now pin exact writer output and validation reports.
- `lefthook` pre-push configuration now runs formatting, linting, and tests before pushes.
- Cargo lint configuration now enables higher-signal Rust and Clippy checks.

### Changed

- **Breaking:** `Adf` now includes root `<adf>` attributes and `TextPart` has a `Node` variant for embedded XML, which affects struct literals and exhaustive enum matches.
- `AdfDocument` now keeps the raw XML tree lazy; `root()` reparses the original input on first access instead of every parse retaining both raw and typed document representations.
- Parser conversion now moves raw XML nodes into the typed model instead of cloning extension subtrees out of an eagerly retained raw tree.
- Typed writing now avoids child sorting for extension-free containers, streams attributes without temporary vectors, and escapes text/attributes in chunks.
- Validation enum checks now use precomputed allowed-value display strings and build issue paths only when a warning is emitted.
- The typed writer now emits known ADF children in DTD order while keeping parsed extension elements near their original source positions.
- README examples now target the current crate version and use a valid `prospect@status` value.

### Fixed

- Removed tracked macOS metadata from the crate package contents.

## [0.3.0] - 2026-05-20

### Added

- Byte spans on every typed model node and on `ValidationIssue`, so a validation warning or error can be mapped back to its exact location in the original input. (db48dc6)
- `parse_with` and `ParseOptions` for configuring parser hardening, plus the `DEFAULT_MAX_DOCTYPE_LEN` constant (4096 bytes).
- `ParseOptions::reject_doctype` to reject any document containing a `<!DOCTYPE>` declaration, and `max_doctype_len` to bound the size of a DOCTYPE declaration payload (checked on raw bytes before decoding).
- Builder methods for ergonomic option construction: `ParseOptions::reject_doctype`, `max_doctype_len`, `without_doctype_limit`, and `ValidationOptions::strict`.

### Changed

- **Breaking:** `ValidationOptions` and `ParseOptions` are now `#[non_exhaustive]`. Construct them from `ValidationOptions::default()` / `ParseOptions::default()` and the builder methods rather than struct literals.
- **Breaking:** `parse` now rejects `<!DOCTYPE>` declarations whose payload exceeds `DEFAULT_MAX_DOCTYPE_LEN` (4096 bytes) by default. Use `ParseOptions::max_doctype_len` or `without_doctype_limit` to change this.

### Security

- Documented and regression-tested that the parser never resolves external entities and never expands custom (DTD-defined) entities, leaving classic XXE and entity-expansion ("billion laughs") attacks structurally impossible.
- Default DOCTYPE size cap bounds the cost of processing an untrusted DTD declaration.

## [0.1.0] - 2026-05-17

### Added

- Initial Auto-lead Data Format XML parsing and writing crate.
- Typed ADF 1.0 model for common prospect, vehicle, customer, vendor, provider, contact, address, ID, price, and text fields.
- Original-preserving document output for clean documents and localized dirty prospect rewrites.
- Normalized typed writer for broad model rewrites.
- Extension preservation for unknown XML elements and attributes.
- ADF-specific structural validation report.
- Regression coverage for entity decoding, root content handling, extension preservation, and dirty prospect rewriting.

## [0.2.0] - 2026-05-17

### Added

- `TextPart` enum and `parts: Vec<TextPart<'a>>` on `TextElement`, `Id`, `Price`, and `Name` so CDATA wrappers and unknown entity references round-trip through the typed writer.
- `XmlNode::EntityRef` variant for unknown general entities in the raw tree.
- `attributes: Vec<Attribute<'a>>` on container structs (`Prospect`, `Vehicle`, `Customer`, `Contact`, `Address`, `ColorCombination`, `VehicleOption`, `Finance`, `Timeframe`, `Vendor`, `Provider`) so unknown XML attributes survive both the typed writer and per-prospect rewrites.
- `ValidationOptions` with a `strict` toggle, `validate_with`, and `AdfDocument::validate_strict`. Strict mode promotes "missing DTD-required element" warnings to errors.
- DTD enumerated-value warnings for `prospect@status`, `vehicle@interest`, `vehicle@status`, `price@type` / `@delta` / `@relativeto`, `name@part` / `@type`, `email`/`phone@preferredcontact`, `phone@type` / `@time`, `address@type`, `odometer@status` / `@units`, `<condition>` body, `<finance>` method, and `<amount>` / `<balance>` `@type` / `@limit`.
- ISO format warnings for `requestdate` / `earliestdate` / `latestdate` (ISO 8601), `@currency` (ISO 4217), and `<country>` (ISO 3166-1 alpha-2).
- Missing-`contact` check for `<vendor>`.

### Changed

- **Breaking:** `TextElement`, `Id`, `Price`, and `Name` no longer expose `value: Cow<'a, str>` as a field. Use `.value()` to read the joined string (resolves standard entities, leaves unknown entity names literal) and `.set_value(...)` to replace with a flat string.
- Typed writer now emits `<customer>` children in DTD order (`contact`, `id`, `timeframe`, `comments`).
- Container writers route through `attrs_preserving_known` / `attrs_from_slice` so unknown attributes survive round-trip.
- `write_cdata` now splits payloads containing `]]>` across two CDATA sections instead of producing an invalid `]]>` literal.
- Parser borrows element names, attribute names, and undecoded attribute values from the input slice instead of always allocating.

### Removed

- Singular `pricecomment` alias in the parser; the element now falls through to `Vehicle::extensions` like any other unknown element.
