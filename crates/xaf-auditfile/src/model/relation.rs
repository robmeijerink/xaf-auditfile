use super::amount::Amount;
use super::codes::RelationKind;
use super::company::Address;
use super::ids::{CountryCode, RelationId};

/// A customer or supplier (`customerSupplier`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Relation {
    /// Identifier (`custSupID`).
    pub id: RelationId,
    /// Name (`custSupName`).
    pub name: Option<String>,
    /// E-mail address (`eMail`).
    pub email: Option<String>,
    /// Chamber of Commerce number (`commerceNr`).
    pub chamber_of_commerce_nr: Option<String>,
    /// Country of the tax registration (`taxRegistrationCountry`).
    pub tax_registration_country: Option<CountryCode>,
    /// VAT identification number (`taxRegIdent`).
    pub tax_registration_id: Option<String>,
    /// Customer, supplier or both (`custSupTp`).
    pub kind: Option<RelationKind>,
    /// Outstanding amount at the start of the period (`opBalDesc` with `opBalTp`).
    pub opening_balance: Option<Amount>,
    /// Outstanding amount at the end of the period (`clBalDesc` with `clBalTp`).
    pub closing_balance: Option<Amount>,
    /// Addresses (`streetAddress`).
    pub addresses: Vec<Address>,
}
