use std::fmt;

macro_rules! string_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(String);

        impl $name {
            /// Wraps the identifier exactly as it appears in the file.
            pub fn new(id: impl Into<String>) -> Self {
                Self(id.into())
            }

            /// The identifier as written in the file.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $name {
            fn from(id: &str) -> Self {
                Self::new(id)
            }
        }

        impl From<String> for $name {
            fn from(id: String) -> Self {
                Self(id)
            }
        }
    };
}

string_id!(
    /// Ledger account identifier (`accID`).
    AccountId
);
string_id!(
    /// VAT code identifier (`vatID`).
    VatCodeId
);
string_id!(
    /// Customer or supplier identifier (`custSupID`).
    RelationId
);
string_id!(
    /// Journal identifier (`jrnID`).
    JournalId
);

/// ISO 4217 currency code as allowed by the schema: at most three characters, no digits.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CurrencyCode(String);

impl CurrencyCode {
    /// Returns `None` when the value is longer than three characters or contains a digit.
    pub fn new(code: &str) -> Option<Self> {
        let valid = code.chars().count() <= 3 && !code.chars().any(|c| c.is_ascii_digit());
        valid.then(|| Self(code.to_owned()))
    }

    /// The code as written in the file.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CurrencyCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// ISO 3166 country code as allowed by the schema: exactly two characters, no digits.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CountryCode(String);

impl CountryCode {
    /// Returns `None` when the value is not two characters long or contains a digit.
    pub fn new(code: &str) -> Option<Self> {
        let valid = code.chars().count() == 2 && !code.chars().any(|c| c.is_ascii_digit());
        valid.then(|| Self(code.to_owned()))
    }

    /// The code as written in the file.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CountryCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_ids_round_trip() {
        let id = AccountId::from("1000");
        assert_eq!(id.as_str(), "1000");
        assert_eq!(id.to_string(), "1000");
        assert_eq!(id, AccountId::new(String::from("1000")));
    }

    #[test]
    fn different_id_kinds_are_distinct_types() {
        fn takes_account(_: &AccountId) {}
        takes_account(&AccountId::from("1"));
    }

    #[test]
    fn currency_code_follows_schema_rules() {
        assert!(CurrencyCode::new("EUR").is_some());
        assert!(CurrencyCode::new("").is_some());
        assert!(CurrencyCode::new("EURO").is_none());
        assert!(CurrencyCode::new("E1R").is_none());
    }

    #[test]
    fn country_code_follows_schema_rules() {
        assert!(CountryCode::new("NL").is_some());
        assert!(CountryCode::new("N").is_none());
        assert!(CountryCode::new("NLD").is_none());
        assert!(CountryCode::new("N1").is_none());
    }
}
