# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

`adf` is a small Rust crate for minimal-overhead ADF 1.0 XML parsing and writing. The core design goal is to parse common ADF fields into a typed model while preserving enough of the original XML structure to avoid losing partner-specific data.

## Commands

Run these before finishing code changes:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
```

Use `cargo fmt` to apply formatting when needed. Run a single integration test with `cargo test --test core <name>`.

## Architecture

The crate is organized around a parse → typed-model → write pipeline that keeps the original input around so unchanged documents (or unchanged regions) can be emitted byte-for-byte.

- `src/parse.rs`: XML tree parsing and conversion into the typed ADF model. Borrows from the input where possible.
- `src/model.rs`: public typed ADF structs (`Adf`, `Prospect`, customer/vehicle/vendor/etc.).
- `src/document.rs`: `AdfDocument` wraps the typed model plus the original text, raw XML tree, dirty tracking, and per-prospect byte spans. This is the entry point for mutation and writing.
- `src/write.rs`: two writers — an original-preserving writer that copies untouched spans verbatim and rewrites only dirty prospects, and a typed writer that serializes the model from scratch.
- `src/validate.rs`: ADF-specific structural validation, deliberately separate from XML parsing.
- `src/error.rs`: crate `Error` / `Result`.
- `tests/core.rs`: integration coverage for parsing, preservation, writing, and validation. Add regression tests here for every parser/writer bug fix.

Two distinctions are load-bearing:

1. **XML parsing vs. ADF validation** — well-formed XML must parse successfully even when it is incomplete or invalid ADF. Don't bake ADF semantics into `parse.rs`.
2. **Original-preserving vs. typed write** — `to_original_preserving_string` is the default for round-tripping partner data; `to_typed_string` is for normalized output after broad structural edits via `adf_mut`. Per-prospect edits via `prospect_mut` mark only that prospect dirty so its original span is the only region rewritten.

## Development Notes

- Prefer preserving input data over normalizing it away.
- Keep unknown XML elements in `extensions` and unknown attributes on every typed element (containers and compact alike) so partner data round-trips.
- Text-bearing elements (`TextElement`, `Id`, `Price`, `Name`) store children as `Vec<TextPart>` so CDATA wrappers and entity refs survive the writer; use `.value()` to get the joined string and `.set_value(...)` to replace it.
- Keep parsing allocation-conscious: borrow input text (element names, attribute names/values, text segments) via `Cow<'a, str>` where possible and allocate only when decoding or joining requires it.
- Validator has a lenient default and a strict mode (`AdfDocument::validate_strict` / `validate_with`). Add new structural rules under the strict toggle and keep format/enum checks as warnings in both modes.
- Add regression tests for every parser or writer bug fix.
