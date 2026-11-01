# xaf-auditfile

A Rust library that reads the Dutch XML Auditfile Financieel (XAF), the audit file format used by
the Belastingdienst and others, into a typed model.

Amounts are `Decimal` (from `rust_decimal`) and dates are `Date` (from `time`), both re-exported by
this crate. Debit and credit are an enum, and references between records (account, VAT code,
customer or supplier) are plain identifier types.

**Only XAF 4.0.x is supported, and only reading.** The version is taken from the namespace of the
root element. Any other version, such as 3.2, is rejected up front with
`Error::UnsupportedVersion`, before the contents are looked at.

```sh
cargo add xaf-auditfile
```

## Example

```rust
use std::collections::BTreeMap;

use xaf_auditfile::{Auditfile, Decimal};

let file = Auditfile::from_path("auditfile.xaf")?;

for finding in file.validate() {
    eprintln!("{finding}");
}

// VAT per code; debit counts as positive, credit as negative.
let mut vat_by_code: BTreeMap<&str, Decimal> = BTreeMap::new();
for (_journal, transaction) in file.transactions() {
    for line in &transaction.lines {
        for vat in &line.vat {
            *vat_by_code.entry(vat.vat_code.as_str()).or_default() += vat.amount.signed();
        }
    }
}
# Ok::<(), xaf_auditfile::Error>(())
```

## What is read

The header (fiscal year, dates, currency, software), the company, customers and suppliers, the
general ledger, VAT codes, periods, the opening balance, and journals with their transactions and
transaction lines, including the VAT entries and foreign currency amounts on a line.

Names are the English forms of the schema's element names (`trLine` becomes `TransactionLine`,
`accID` becomes `account`), and the rustdoc of every field says which element it comes from.
Optional elements are `Option`, and an empty element such as `<bankAccNr/>` is read as absent.
Values from a code list (journal type, account type, customer/supplier type) have an
`Unknown(String)` variant, so a code that is not in the list does not fail the parse. Debit and
credit has no such variant: without it a balance cannot be checked, so any other value is an
error.

## Errors

`Auditfile::from_reader` and `Auditfile::from_path` return `Result<Auditfile, Error>`:

| Error | Meaning |
| --- | --- |
| `Io` | the file could not be read |
| `UnsupportedVersion(namespace)` | an XAF file of another version |
| `NotAnAuditfile` | XML, but not an auditfile (other root element, no namespace, empty, or content outside the root element) |
| `Encoding` | the text encoding is not supported (UTF-16, UTF-32) or the file is not valid in the encoding it declares |
| `Xml` | the XML is not well-formed, for example because the file is truncated |
| `Deserialize` | well-formed XML that does not match the schema, such as a missing required element or text mixed with elements |
| `NestingTooDeep(limit)` | elements nested more than 128 levels deep (the schema needs about 8) |
| `OutOfOrder(element)` | only when streaming: an element is not in the order of the schema |
| `InvalidValue` | a date, amount, code or number that cannot be read; names the element, journal, transaction and line |

A `Deserialize` error names the missing or duplicate element but not where it was in the file.

The file must be well-formed XML to the end, in both modes: a file that is cut off, has a second
root element, or has text outside the root element or mixed with elements is an error, so a
truncated file is not read as a smaller, complete one.

## Testing code that uses this crate

The model types are `#[non_exhaustive]`, so a field can be added later without breaking you. That
also means you cannot create them with a struct expression. In a test, read a small file with
`Auditfile::from_reader` instead; the rustdoc of the `model` module has a complete example.

## Validation

`Auditfile::validate()` is separate from parsing and returns a list of findings instead of an
error. It reports:

- transactions and the opening balance that are not in balance;
- lines that refer to a ledger account, VAT code, customer or supplier that does not exist, and
  the same for the accounts of VAT codes, journals and the opening balance;
- transactions in a period that is not listed;
- duplicate account, VAT code and customer/supplier identifiers, duplicate RGS codes, and
  duplicate transaction numbers within a journal;
- control totals (`linesCount`, `totalDebit`, `totalCredit`) that differ from the lines.

References are checked against the lists in the file, so a file that has no list at all (for
example no `periods`) has every reference to it reported.

## Streaming (for exceptionally large files)

Reading the whole file with `Auditfile::from_reader` or `Auditfile::from_path` is the normal way
to use this crate and what you should do unless memory is a real problem. Streaming is an
exception for files too large to hold in memory:

```rust
use xaf_auditfile::Auditfile;

let stream = Auditfile::stream_from_path("huge.xaf")?;

// Header, company, master data and opening balance; `journals` is empty.
let mut findings = stream.auditfile().validate_master_data();
let mut validator = stream.auditfile().validator();

for item in stream {
    let item = item?;
    findings.extend(validator.check(&item.journal, &item.transaction));
    // handle item.transaction here
}
findings.extend(validator.finish());
# Ok::<(), xaf_auditfile::Error>(())
```

What you give up, compared with reading the whole file:

- The elements must be in the order of the schema, with the transactions last. A file that is not
  fails with `Error::OutOfOrder`; the whole-file reader accepts any order.
- There is no complete `Auditfile`: `journals` is empty and `validate()` does not apply. The
  `Validator` gives the same findings as `validate()` when combined with
  `validate_master_data()`.
- Journals without transactions are not yielded.
- Errors in a transaction surface when that transaction is reached, after which the stream ends.
- It is somewhat slower.

Measured on the synthetic files described under Limitations, streaming used about 10 MB regardless
of the size of the file (against 0.4 and 0.6 GB for reading it whole), and took roughly 10% to 15%
longer. The numbers are for one machine and synthetic data.

## Limitations

- **Memory.** The whole file is read into memory and deserialized in one go, so memory use grows
  with the number of transaction lines (roughly 1.3 to 2.1 KB per line) rather than with the size
  of the file. Measured on a release build with synthetic files of 300,000 lines: 0.4 GB and 2.8
  seconds for a 68 MB file with few fields per line, 0.6 GB and 5.5 seconds for a 201 MB file with
  every field filled. A year with a million lines needs a few GB. That is fine for the yearly file
  of a small or medium company. For the rare file that does not fit, see
  [Streaming](#streaming-for-exceptionally-large-files).
- **Untrusted input.** Entities declared in a DOCTYPE are neither expanded nor fetched (such a
  file is rejected as malformed), so a file cannot make the reader open other files or blow up
  through entity expansion, and elements nested more than 128 levels deep are rejected. Amounts of
  21 digits or more are rejected. There is no limit on the size of the input, though, so limit it
  yourself when reading files you do not control.
- **Not an XSD validator.** Only what the model needs is checked. String lengths and the number of
  decimals of the schema are not enforced, and unknown elements are ignored so that a later 4.0.x
  that adds an element still reads.
- **Time zones.** A time zone on a date (`2024-01-31+01:00`) is accepted and ignored.
- **Encoding.** UTF-8 and the other ASCII-compatible encodings that the XML declaration names, such
  as ISO-8859-1 and Windows-1252, are decoded. UTF-16 and UTF-32 are not supported and give
  `Error::Encoding`.
- **Journal type `O`.** The schema allows it, the functional description does not list it, so it
  is read as `JournalKind::Unknown("O")`.

## Sources

The model follows `XmlAuditfileFinancieel4.0.xsd` of XAF 4.0.3. The meaning of the code lists and
the uniqueness rules used by `validate()` come from the functional description that accompanies
the schema. The test fixtures in `crates/xaf-auditfile/tests/fixtures` are made up.

## Minimum supported Rust version

Rust 1.88 (edition 2024).

## License

MIT, see [LICENSE](LICENSE).

---

This library is developed by [Solvalutions](https://solvalutions.nl) and [Rob Meijerink](https://robmeijerink.nl).

*&copy;&nbsp;2026 &mdash; Built by Rob Meijerink.*
