//! The public, version-independent domain model.
//!
//! The structs and code lists here are `#[non_exhaustive]`, so that a field or variant can be
//! added when the schema grows without breaking code that uses this crate. You can read, match and
//! pass them around freely, but not create them with a struct expression. To get a value in a
//! test, read a small, consistent file instead, which also exercises the code that reads it.
//! This one is a sale of 100.00 with 21% VAT:
//!
//! ```
//! use xaf_auditfile::{Auditfile, Decimal};
//!
//! let xml = r#"
//! <auditfile xmlns="http://www.odb.belastingdienst.nl/Belastingdienst/BCPP/1.1/structures/XmlauditfileXAF_4.0">
//!   <header>
//!     <fiscalYear>2024</fiscalYear>
//!     <startDate>2024-01-01</startDate>
//!     <endDate>2024-12-31</endDate>
//!     <curCode>EUR</curCode>
//!     <dateCreated>2025-02-01</dateCreated>
//!     <softwareDesc>Test</softwareDesc>
//!     <softwareVersion>1</softwareVersion>
//!   </header>
//!   <company>
//!     <companyName>Test B.V.</companyName>
//!     <taxRegistrationCountry>NL</taxRegistrationCountry>
//!     <taxRegIdent>NL000000000B01</taxRegIdent>
//!     <generalLedger>
//!       <ledgerAccount>
//!         <accID>1300</accID>
//!         <accDesc>Debiteuren</accDesc>
//!         <accTp>B</accTp>
//!       </ledgerAccount>
//!       <ledgerAccount>
//!         <accID>1800</accID>
//!         <accDesc>BTW af te dragen</accDesc>
//!         <accTp>B</accTp>
//!       </ledgerAccount>
//!       <ledgerAccount>
//!         <accID>8000</accID>
//!         <accDesc>Omzet</accDesc>
//!         <accTp>P</accTp>
//!       </ledgerAccount>
//!     </generalLedger>
//!     <vatCodes>
//!       <vatCode>
//!         <vatID>HOOG</vatID>
//!         <vatDesc>BTW hoog tarief</vatDesc>
//!         <vatToPayAccID>1800</vatToPayAccID>
//!       </vatCode>
//!     </vatCodes>
//!     <periods>
//!       <period>
//!         <periodNumber>1</periodNumber>
//!         <startDatePeriod>2024-01-01</startDatePeriod>
//!         <endDatePeriod>2024-01-31</endDatePeriod>
//!       </period>
//!     </periods>
//!     <transactions>
//!       <linesCount>3</linesCount>
//!       <totalDebit>121.00</totalDebit>
//!       <totalCredit>121.00</totalCredit>
//!       <journal>
//!         <jrnID>VERK</jrnID>
//!         <desc>Verkoop</desc>
//!         <transaction>
//!           <nr>1</nr>
//!           <periodNumber>1</periodNumber>
//!           <trDt>2024-01-10</trDt>
//!           <trLine>
//!             <nr>1</nr>
//!             <accID>1300</accID>
//!             <docRef>5001</docRef>
//!             <effDate>2024-01-10</effDate>
//!             <amnt>121.00</amnt>
//!             <amntTp>D</amntTp>
//!           </trLine>
//!           <trLine>
//!             <nr>2</nr>
//!             <accID>8000</accID>
//!             <docRef>5001</docRef>
//!             <effDate>2024-01-10</effDate>
//!             <amnt>100.00</amnt>
//!             <amntTp>C</amntTp>
//!             <vat>
//!               <vatID>HOOG</vatID>
//!               <vatPerc>21</vatPerc>
//!               <vatAmnt>21.00</vatAmnt>
//!               <vatAmntTp>C</vatAmntTp>
//!             </vat>
//!           </trLine>
//!           <trLine>
//!             <nr>3</nr>
//!             <accID>1800</accID>
//!             <docRef>5001</docRef>
//!             <effDate>2024-01-10</effDate>
//!             <amnt>21.00</amnt>
//!             <amntTp>C</amntTp>
//!           </trLine>
//!         </transaction>
//!       </journal>
//!     </transactions>
//!   </company>
//! </auditfile>"#;
//!
//! let file = Auditfile::from_reader(xml.as_bytes())?;
//! assert_eq!(file.transactions().count(), 1);
//! assert!(file.validate().is_empty());
//!
//! // The VAT of the file, per code; credit counts as negative.
//! let vat: Vec<_> = file
//!     .transactions()
//!     .flat_map(|(_, transaction)| &transaction.lines)
//!     .flat_map(|line| &line.vat)
//!     .map(|vat| (vat.vat_code.as_str(), vat.percentage, vat.amount.signed()))
//!     .collect();
//! assert_eq!(vat, [("HOOG", Decimal::from(21), Decimal::from(-21))]);
//! # Ok::<(), xaf_auditfile::Error>(())
//! ```
//!
//! Matching on a struct needs `..` for the same reason: `let Period { number, .. } = period;`.

mod amount;
mod codes;
mod company;
mod header;
mod ids;
mod journal;
mod ledger;
mod relation;
mod vat;

pub use amount::{Amount, Side};
pub use codes::{AccountKind, JournalKind, RelationKind};
pub use company::{Address, Company};
pub use header::{FiscalYear, Header, Period, Software};
pub use ids::{AccountId, CountryCode, CurrencyCode, JournalId, RelationId, VatCodeId};
pub use journal::{ControlTotals, ForeignAmount, Journal, Transaction, TransactionLine, VatLine};
pub use ledger::{LedgerAccount, OpeningBalance, OpeningBalanceLine};
pub use relation::Relation;
pub use vat::VatCode;

/// A parsed XAF auditfile.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Auditfile {
    /// The `header` element.
    pub header: Header,
    /// The company the file belongs to.
    pub company: Company,
    /// Customers and suppliers (`customersSuppliers`).
    pub relations: Vec<Relation>,
    /// Ledger accounts (`generalLedger`).
    pub accounts: Vec<LedgerAccount>,
    /// VAT codes (`vatCodes`).
    pub vat_codes: Vec<VatCode>,
    /// Booking periods (`periods`).
    pub periods: Vec<Period>,
    /// Opening balance (`openingBalance`).
    pub opening_balance: Option<OpeningBalance>,
    /// Control totals of the `transactions` element.
    pub transaction_totals: Option<ControlTotals>,
    /// Journals with their transactions.
    pub journals: Vec<Journal>,
}

impl Auditfile {
    /// All transactions in document order, each with the journal it belongs to.
    pub fn transactions(&self) -> impl Iterator<Item = (&Journal, &Transaction)> {
        self.journals
            .iter()
            .flat_map(|journal| journal.transactions.iter().map(move |t| (journal, t)))
    }
}
