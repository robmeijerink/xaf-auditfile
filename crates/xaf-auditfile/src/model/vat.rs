use super::ids::{AccountId, VatCodeId};

/// A VAT code (`vatCode`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct VatCode {
    /// Identifier (`vatID`).
    pub id: VatCodeId,
    /// Description (`vatDesc`).
    pub description: String,
    /// Account for VAT to pay (`vatToPayAccID`).
    pub payable_account: Option<AccountId>,
    /// Account for VAT to claim (`vatToClaimAccID`).
    pub claimable_account: Option<AccountId>,
}
