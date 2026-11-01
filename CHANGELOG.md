# Changelog

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
