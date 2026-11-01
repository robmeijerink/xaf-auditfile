use super::ids::CountryCode;

/// A `streetAddress` of the company or of a relation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Address {
    /// Street name (`streetname`).
    pub street: Option<String>,
    /// House number (`number`).
    pub number: Option<String>,
    /// House number extension (`numberExtension`).
    pub number_extension: Option<String>,
    /// City (`city`).
    pub city: Option<String>,
    /// Postal code (`postalCode`).
    pub postal_code: Option<String>,
    /// Country (`country`).
    pub country: Option<CountryCode>,
}

/// The company the file belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Company {
    /// Chamber of Commerce number (`Commercenr`).
    pub chamber_of_commerce_nr: Option<String>,
    /// Company name (`companyName`).
    pub name: String,
    /// Country of the tax registration (`taxRegistrationCountry`).
    pub tax_registration_country: CountryCode,
    /// VAT identification number (`taxRegIdent`).
    pub tax_registration_id: String,
    /// Addresses (`streetAddress`).
    pub addresses: Vec<Address>,
}
