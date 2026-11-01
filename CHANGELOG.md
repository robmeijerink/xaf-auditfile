# Changelog

## 0.1.1 - 2026-11-01

- `Error::Xml` now reports the underlying parser error as its source.
- Every form of UTF-16 and UTF-32 is rejected with `Error::Encoding`. Files without a byte order
  mark or without an XML declaration used to be reported as not being an auditfile.
- `AuditfileStream` implements `Debug`.
- The results of `Auditfile::validate`, `Auditfile::validate_master_data` and the `Validator`
  methods are `#[must_use]`, so dropping them is no longer silent.
- Documentation: the memory figures are consistent and match the measurements.

## 0.1.0 - 2026-11-01

First version.

- Reads XAF 4.0.x into a typed model: header, company, customers and suppliers, general ledger,
  VAT codes, periods, opening balance, and journals with transactions and lines.
- Rejects other XAF versions up front with `Error::UnsupportedVersion`.
- `Auditfile::validate` reports unbalanced transactions, dangling references, duplicate
  identifiers and control totals that do not match.
- `Auditfile::stream_from_reader` and `stream_from_path` read the transactions one at a time, for
  files too large to hold in memory, with `Validator` for the same checks.
- Decodes UTF-8, ISO-8859-1, Windows-1252 and other ASCII-compatible encodings.
