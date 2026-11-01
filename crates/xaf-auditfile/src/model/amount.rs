use rust_decimal::Decimal;

/// Debit or credit side of an amount.
///
/// The schema allows only `D` and `C`, and a balance check is meaningless without knowing the
/// side, so unlike the code lists this enum has no catch-all variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    /// Debit (`D`).
    Debit,
    /// Credit (`C`).
    Credit,
}

impl Side {
    /// Parses the single-letter code used in the file.
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "D" => Some(Self::Debit),
            "C" => Some(Self::Credit),
            _ => None,
        }
    }

    /// The single-letter code used in the file.
    pub fn as_code(self) -> &'static str {
        match self {
            Self::Debit => "D",
            Self::Credit => "C",
        }
    }
}

/// A monetary amount together with its debit/credit side, as stored in the file.
///
/// The value is kept exactly as written, so it may be negative.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Amount {
    /// The amount as written in the file.
    pub value: Decimal,
    /// Whether the amount is a debit or a credit.
    pub side: Side,
}

impl Amount {
    /// Combines a value, as written in the file, with its side.
    pub fn new(value: Decimal, side: Side) -> Self {
        Self { value, side }
    }

    /// The amount with debit as positive and credit as negative.
    pub fn signed(&self) -> Decimal {
        match self.side {
            Side::Debit => self.value,
            Side::Credit => -self.value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn side_codes_round_trip() {
        assert_eq!(Side::from_code("D"), Some(Side::Debit));
        assert_eq!(Side::from_code("C"), Some(Side::Credit));
        assert_eq!(Side::Debit.as_code(), "D");
        assert_eq!(Side::Credit.as_code(), "C");
    }

    #[test]
    fn side_rejects_other_codes() {
        assert_eq!(Side::from_code("X"), None);
        assert_eq!(Side::from_code("d"), None);
        assert_eq!(Side::from_code(""), None);
    }

    #[test]
    fn signed_debit_is_positive_credit_is_negative() {
        let value = Decimal::new(12345, 2);
        assert_eq!(Amount::new(value, Side::Debit).signed(), value);
        assert_eq!(Amount::new(value, Side::Credit).signed(), -value);
    }

    #[test]
    fn signed_keeps_negative_values_as_written() {
        let value = Decimal::new(-500, 2);
        assert_eq!(Amount::new(value, Side::Debit).signed(), value);
        assert_eq!(Amount::new(value, Side::Credit).signed(), -value);
    }
}
