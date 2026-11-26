# Vendored `mln-rs-expression` 0.0.12

This is the published crate, unchanged except for its locale backend.

## Why

`mln-rs-expression` 0.0.12 uses ICU4X 2.4 for collation and number formatting on
desktop targets that are neither Apple nor Android. Sortlens' `craft-i18n`
pins ICU4X 2.3 for its generated CLDR data, and Cargo cannot resolve both
versions in one graph.

OpenFreeMap styles use `number-format` and `collator` expressions only for
label text, so the map does not need full CLDR data. This fork replaces
`src/locale/icu4x.rs` with `src/locale/fallback.rs`: a case-insensitive string
collator and a decimal formatter with thousands grouping, fraction limits,
and the currency code or unit name appended. Apple and Android keep their
platform backends, exactly as upstream.

## Changes

- Removed the ICU4X and `fixed_decimal`/`writeable` dependencies.
- Replaced `mod icu4x` and its re-export with `mod fallback`.
- Deleted `src/locale/icu4x.rs`; everything else is the published 0.0.12 source.

## Upstream

<https://crates.io/crates/mln-rs-expression/0.0.12>

BSD-2-Clause. The upstream licence is in `LICENSE.md`.
