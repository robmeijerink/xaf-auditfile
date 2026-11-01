//! Code lists of the schema. The meanings come from the functional description
//! (`XMLAuditfileFinancieel_4.0_FunHie`) that accompanies the XSD; the XSD itself only lists the
//! letters. Values outside the list are preserved in `Unknown`.

/// Type of a ledger account (`accTp`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AccountKind {
    /// Balance sheet account (`B`).
    BalanceSheet,
    /// Profit and loss account (`P`).
    ProfitAndLoss,
    /// A code not in the list.
    Unknown(String),
}

impl AccountKind {
    /// Parses the code used in the file.
    pub fn from_code(code: &str) -> Self {
        match code {
            "B" => Self::BalanceSheet,
            "P" => Self::ProfitAndLoss,
            other => Self::Unknown(other.to_owned()),
        }
    }

    /// The code used in the file.
    pub fn as_code(&self) -> &str {
        match self {
            Self::BalanceSheet => "B",
            Self::ProfitAndLoss => "P",
            Self::Unknown(code) => code,
        }
    }
}

/// Whether a relation is a customer, a supplier or both (`custSupTp`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RelationKind {
    /// Both customer and supplier (`B`).
    Both,
    /// Customer (`C`).
    Customer,
    /// Supplier (`S`).
    Supplier,
    /// A code not in the list.
    Unknown(String),
}

impl RelationKind {
    /// Parses the code used in the file.
    pub fn from_code(code: &str) -> Self {
        match code {
            "B" => Self::Both,
            "C" => Self::Customer,
            "S" => Self::Supplier,
            other => Self::Unknown(other.to_owned()),
        }
    }

    /// The code used in the file.
    pub fn as_code(&self) -> &str {
        match self {
            Self::Both => "B",
            Self::Customer => "C",
            Self::Supplier => "S",
            Self::Unknown(code) => code,
        }
    }
}

/// Type of a journal (`jrnTp`).
///
/// The XSD also accepts `O`, which the functional description does not list; it is preserved as
/// `Unknown("O")`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum JournalKind {
    /// Bank (`B`).
    Bank,
    /// Cash (`C`).
    Cash,
    /// Goods received or sent (`G`).
    Goods,
    /// Memorial journal or daybook (`M`).
    Memo,
    /// Purchases (`P`).
    Purchases,
    /// Sales (`S`).
    Sales,
    /// Production (`T`).
    Production,
    /// Payroll (`Y`).
    Payroll,
    /// Other (`Z`).
    Other,
    /// A code not in the list.
    Unknown(String),
}

impl JournalKind {
    /// Parses the code used in the file.
    pub fn from_code(code: &str) -> Self {
        match code {
            "B" => Self::Bank,
            "C" => Self::Cash,
            "G" => Self::Goods,
            "M" => Self::Memo,
            "P" => Self::Purchases,
            "S" => Self::Sales,
            "T" => Self::Production,
            "Y" => Self::Payroll,
            "Z" => Self::Other,
            other => Self::Unknown(other.to_owned()),
        }
    }

    /// The code used in the file.
    pub fn as_code(&self) -> &str {
        match self {
            Self::Bank => "B",
            Self::Cash => "C",
            Self::Goods => "G",
            Self::Memo => "M",
            Self::Purchases => "P",
            Self::Sales => "S",
            Self::Production => "T",
            Self::Payroll => "Y",
            Self::Other => "Z",
            Self::Unknown(code) => code,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_kind_codes() {
        assert_eq!(AccountKind::from_code("B"), AccountKind::BalanceSheet);
        assert_eq!(AccountKind::from_code("P"), AccountKind::ProfitAndLoss);
        assert_eq!(
            AccountKind::from_code("X"),
            AccountKind::Unknown("X".into())
        );
        assert_eq!(AccountKind::Unknown("X".into()).as_code(), "X");
    }

    #[test]
    fn relation_kind_codes() {
        for code in ["B", "C", "S"] {
            assert_eq!(RelationKind::from_code(code).as_code(), code);
        }
        assert_eq!(RelationKind::from_code("C"), RelationKind::Customer);
        assert_eq!(
            RelationKind::from_code("?"),
            RelationKind::Unknown("?".into())
        );
    }

    #[test]
    fn journal_kind_codes_round_trip() {
        for code in ["B", "C", "G", "M", "P", "S", "T", "Y", "Z"] {
            let kind = JournalKind::from_code(code);
            assert!(!matches!(kind, JournalKind::Unknown(_)), "{code}");
            assert_eq!(kind.as_code(), code);
        }
    }

    #[test]
    fn journal_kind_o_is_preserved_as_unknown() {
        let kind = JournalKind::from_code("O");
        assert_eq!(kind, JournalKind::Unknown("O".into()));
        assert_eq!(kind.as_code(), "O");
    }
}
