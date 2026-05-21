# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.3.0] - 2026-05-20

### Added

- Byte spans on every typed model node and on `ValidationIssue`, so a validation warning or error can be mapped back to its exact location in the original input. (db48dc6)
- `parse_with` and `ParseOptions` for configuring parser hardening, plus the `DEFAULT_MAX_DOCTYPE_LEN` constant (4096 bytes).
- `ParseOptions::reject_doctype` to reject any document containing a `<!DOCTYPE>` declaration, and `max_doctype_len` to bound the size of a DOCTYPE internal subset (checked on raw bytes before decoding).
- Builder methods for ergonomic option construction: `ParseOptions::reject_doctype`, `max_doctype_len`, `without_doctype_limit`, and `ValidationOptions::strict`.

### Changed

- **Breaking:** `ValidationOptions` and `ParseOptions` are now `#[non_exhaustive]`. Construct them from `ValidationOptions::default()` / `ParseOptions::default()` and the builder methods rather than struct literals.
- **Breaking:** `parse` now rejects `<!DOCTYPE>` declarations whose internal subset exceeds `DEFAULT_MAX_DOCTYPE_LEN` (4096 bytes) by default. Use `ParseOptions::max_doctype_len` or `without_doctype_limit` to change this.

### Security

- Documented and regression-tested that the parser never resolves external entities and never expands custom (DTD-defined) entities, leaving classic XXE and entity-expansion ("billion laughs") attacks structurally impossible.
- Default DOCTYPE size cap bounds the cost of processing an untrusted DTD internal subset.

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
