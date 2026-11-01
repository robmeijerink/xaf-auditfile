//! Reader for the Dutch XML Auditfile Financieel (XAF), version 4.0.x.
//!
//! Only reading is supported, and only version 4.0.x: the version is taken from the namespace of
//! the root element, and any other version is rejected with [`Error::UnsupportedVersion`] before
//! the contents are looked at.
//!
//! The file is read into a version-independent model ([`Auditfile`]). Amounts are
//! [`rust_decimal::Decimal`], dates are [`time::Date`], and references between records are
//! identifier types such as [`AccountId`]. [`Auditfile::validate`] checks the file for
//! inconsistencies the schema does not prevent and returns a list of [`Finding`]s.
//!
//! ```no_run
//! use std::collections::BTreeMap;
//!
//! use xaf_auditfile::{Auditfile, Decimal};
//!
//! let file = Auditfile::from_path("auditfile.xaf")?;
//!
//! for finding in file.validate() {
//!     eprintln!("{finding}");
//! }
//!
//! // VAT per code; debit counts as positive, credit as negative.
//! let mut vat_by_code: BTreeMap<&str, Decimal> = BTreeMap::new();
//! for (_journal, transaction) in file.transactions() {
//!     for line in &transaction.lines {
//!         for vat in &line.vat {
//!             *vat_by_code.entry(vat.vat_code.as_str()).or_default() += vat.amount.signed();
//!         }
//!     }
//! }
//! # Ok::<(), xaf_auditfile::Error>(())
//! ```
//!
//! The whole file is read into memory, which takes roughly 1.5 to 2 KB per transaction line. That
//! is the normal way to use this crate. For an exceptionally large file there is
//! [`Auditfile::stream_from_reader`], which hands out the transactions one at a time at the cost of
//! some conveniences; see its documentation before reaching for it.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod document;
mod error;
pub mod model;
mod read;
mod v4;
mod validate;
mod version;

pub use error::{Error, Location};
pub use model::*;
pub use rust_decimal::Decimal;
pub use time::Date;
pub use v4::{AuditfileStream, StreamedTransaction};
pub use validate::{Finding, FindingKind, Validator};
