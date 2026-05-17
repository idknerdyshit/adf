# CLAUDE.md

Guidance for future agent work in this repository.

## Project

`adf` is a small Rust crate for minimal-overhead ADF 1.0 XML parsing and writing. The core design goal is to parse common ADF fields into a typed model while preserving enough XML structure to avoid losing partner-specific data.

## Important Commands

Run these before finishing code changes:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
```

Use `cargo fmt` to apply formatting when needed.

## Architecture

- `src/parse.rs`: XML tree parsing and conversion into the typed ADF model.
- `src/model.rs`: public typed ADF structs.
- `src/document.rs`: `AdfDocument`, original text, dirty tracking, and write entry points.
- `src/write.rs`: original-preserving and typed XML writers.
- `src/validate.rs`: ADF-specific validation.
- `tests/core.rs`: integration coverage for parsing, preservation, writing, and validation.

## Development Notes

- Prefer preserving input data over normalizing it away.
- Keep unknown XML elements in `extensions`.
- Keep unknown attributes on typed compact elements when rewriting.
- Keep parsing allocation-conscious: borrow input text where possible and allocate only when decoding or joining requires it.
- Preserve the distinction between XML parsing and ADF validation. Well-formed XML can parse even if it is incomplete ADF.
- Add regression tests for every parser or writer bug fix.

