use rust_decimal::Decimal;
use time::Date;

use super::amount::Amount;
use super::codes::JournalKind;
use super::ids::{AccountId, CurrencyCode, JournalId, RelationId, VatCodeId};

/// Control totals stated in the file (`linesCount`, `totalDebit`, `totalCredit`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ControlTotals {
    /// Number of lines.
    pub lines_count: u64,
    /// Sum of the debit amounts.
    pub total_debit: Decimal,
    /// Sum of the credit amounts.
    pub total_credit: Decimal,
}

/// VAT on a transaction line (`vat`). A line can carry several.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct VatLine {
    /// VAT code (`vatID`).
    pub vat_code: VatCodeId,
    /// Percentage as written, so `21` means 21% (`vatPerc`).
    pub percentage: Decimal,
    /// VAT amount (`vatAmnt` with `vatAmntTp`).
    pub amount: Amount,
}

/// Amount in a foreign currency (`currency`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ForeignAmount {
    /// Currency (`curCode`).
    pub currency: CurrencyCode,
    /// Amount in that currency (`curAmnt`).
    pub amount: Decimal,
}

/// A transaction line (`trLine`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct TransactionLine {
    /// Line number (`nr`).
    pub nr: String,
    /// Account (`accID`).
    pub account: AccountId,
    /// Document reference (`docRef`).
    pub document_ref: String,
    /// Effective date (`effDate`).
    pub effective_date: Date,
    /// Settlement date (`settDate`).
    pub settlement_date: Option<Date>,
    /// Description (`desc`).
    pub description: Option<String>,
    /// Amount (`amnt` with `amntTp`).
    pub amount: Amount,
    /// Customer or supplier (`custSupID`).
    pub relation: Option<RelationId>,
    /// Invoice reference (`invRef`).
    pub invoice_ref: Option<String>,
    /// Receiving document reference (`receivingDocRef`).
    pub receiving_doc_ref: Option<String>,
    /// Shipping document reference (`shipDocRef`).
    pub shipping_doc_ref: Option<String>,
    /// Cost centre (`cost`).
    pub cost: Option<String>,
    /// Product (`product`).
    pub product: Option<String>,
    /// Project (`project`).
    pub project: Option<String>,
    /// Work cost arrangement reference (`workCostArrRef`).
    pub work_cost_arrangement_ref: Option<String>,
    /// Bank account number (`bankAccNr`).
    pub bank_account_nr: Option<String>,
    /// Counter bank account number (`offsetBankAccNr`).
    pub offset_bank_account_nr: Option<String>,
    /// VAT postings (`vat`, at most 99).
    pub vat: Vec<VatLine>,
    /// Amount in a foreign currency (`currency`).
    pub foreign_currency: Option<ForeignAmount>,
}

/// A transaction (`transaction`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Transaction {
    /// Transaction number (`nr`).
    pub nr: String,
    /// Description (`desc`).
    pub description: Option<String>,
    /// Period number (`periodNumber`).
    pub period: u16,
    /// Transaction date (`trDt`).
    pub date: Date,
    /// Source of the transaction (`Source`).
    pub source: Option<String>,
    /// User that entered the transaction (`User`).
    pub user: Option<String>,
    /// Lines.
    pub lines: Vec<TransactionLine>,
}

/// A journal (`journal`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Journal {
    /// Identifier (`jrnID`).
    pub id: JournalId,
    /// Description (`desc`).
    pub description: String,
    /// Type of journal (`jrnTp`).
    pub kind: Option<JournalKind>,
    /// Fixed counter account of the journal (`offsetAccID`).
    pub offset_account: Option<AccountId>,
    /// Transactions.
    pub transactions: Vec<Transaction>,
}
