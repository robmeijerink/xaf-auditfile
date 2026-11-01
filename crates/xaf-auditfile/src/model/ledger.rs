use super::amount::Amount;
use super::codes::AccountKind;
use super::ids::AccountId;
use super::journal::ControlTotals;

/// A ledger account (`ledgerAccount`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LedgerAccount {
    /// Identifier (`accID`).
    pub id: AccountId,
    /// Description (`accDesc`).
    pub description: String,
    /// Balance sheet or profit and loss (`accTp`).
    pub kind: AccountKind,
    /// RGS code linked to the account (`RGScode`).
    pub rgs_code: Option<String>,
}

/// A line of the opening balance (`obLine`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct OpeningBalanceLine {
    /// Line number (`nr`).
    pub nr: String,
    /// Account (`accID`).
    pub account: AccountId,
    /// Amount (`amnt` with `amntTp`).
    pub amount: Amount,
}

/// The `openingBalance` element.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct OpeningBalance {
    /// Control totals as stated in the file.
    pub totals: ControlTotals,
    /// Lines.
    pub lines: Vec<OpeningBalanceLine>,
}
