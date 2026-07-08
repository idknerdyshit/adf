# AGENTS.md

## Scope

These instructions apply to the whole repository.

`adf` is a Rust crate for low-overhead Auto-lead Data Format (ADF) 1.0 XML
parsing and writing. The main design goal is to expose common ADF fields as a
typed model while preserving partner-specific XML data.

## Commands

Run these before finishing code changes:

```sh
cargo fmt --all --check
cargo test
cargo clippy --all-targets -- -D warnings
```

Use `cargo fmt --all` to apply formatting when needed.

Run a single integration test with:

```sh
cargo test --test core <name>
```

## Architecture

- `src/parse.rs`: XML parsing and conversion into the typed ADF model.
- `src/model.rs`: public typed ADF structs.
- `src/document.rs`: `AdfDocument`, original XML storage, raw tree access,
  dirty tracking, and per-prospect spans.
- `src/write.rs`: original-preserving and typed XML writers.
- `src/validate.rs`: ADF-specific structural validation.
- `src/error.rs`: crate `Error` and `Result` types.
- `tests/core.rs`: integration coverage for parsing, preservation, writing,
  validation, and regression tests.

## Core Invariants

- Keep XML parsing separate from ADF validation. Well-formed XML rooted at
  `<adf>` should parse even when ADF content is incomplete or invalid.
- Keep deeper ADF content rules in `validate.rs`, not in the parser.
- Preserve input data by default. Unknown XML elements and unknown attributes
  must round-trip on both container and compact typed elements.
- Preserve CDATA wrappers and unknown text entity references through
  `TextPart`-based fields. Use `.value()` to read joined text and
  `.set_value(...)` to replace it.
- `to_original_preserving_string()` should emit clean documents byte-for-byte
  and rewrite only dirty prospect spans after `prospect_mut`.
- `to_typed_string()` is for normalized output after broad structural edits via
  `adf_mut`.
- Never resolve external entities or expand custom DTD-defined entities.
- Keep parsing allocation-conscious. Borrow input with `Cow<'a, str>` where
  practical and allocate only when decoding or joining requires it.
- Keep tracing passive and structural. Do not log raw XML, names, emails,
  phone numbers, addresses, identifiers, URLs, comments, extension payloads, or
  validation messages.

## Testing Expectations

- Add regression tests in `tests/core.rs` for parser or writer bug fixes.
- When changing validation, cover lenient defaults and strict mode where
  relevant.
- When changing write behavior, test both original-preserving output and typed
  output if both paths are affected.
- When changing extension or text handling, test round-tripping of unknown
  elements, attributes, CDATA, and entity references as applicable.

## Style

- Follow the existing public API style and keep changes narrowly scoped.
- Prefer preserving caller and partner data over normalizing it away.
- Keep new dependencies out unless they are clearly justified.
- Do not introduce `unsafe`; it is forbidden by crate lints.
