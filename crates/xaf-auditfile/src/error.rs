use std::fmt;

/// Where in the file a problem was found.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Location {
    /// Path of the element, for example `trLine/effDate`.
    pub element: &'static str,
    /// Identifier of the record the element belongs to: a relation, period or opening balance
    /// line.
    pub record: Option<String>,
    /// Identifier of the journal (`jrnID`), if the element is inside one.
    pub journal: Option<String>,
    /// Number of the transaction (`nr`), if the element is inside one.
    pub transaction: Option<String>,
    /// Number of the transaction line (`nr`), if the element is inside one.
    pub line: Option<String>,
}

impl Location {
    /// A location that only names the element.
    pub fn new(element: &'static str) -> Self {
        Self {
            element,
            record: None,
            journal: None,
            transaction: None,
            line: None,
        }
    }

    /// Adds the record the element belongs to.
    #[must_use]
    pub fn in_record(mut self, record: impl Into<String>) -> Self {
        self.record = Some(record.into());
        self
    }

    /// Adds the journal the element is in.
    #[must_use]
    pub fn in_journal(mut self, journal: impl Into<String>) -> Self {
        self.journal = Some(journal.into());
        self
    }

    /// Adds the transaction the element is in.
    #[must_use]
    pub fn in_transaction(mut self, transaction: impl Into<String>) -> Self {
        self.transaction = Some(transaction.into());
        self
    }

    /// Adds the transaction line the element is in.
    #[must_use]
    pub fn in_line(mut self, line: impl Into<String>) -> Self {
        self.line = Some(line.into());
        self
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.element)?;
        let context = [
            ("record", &self.record),
            ("journal", &self.journal),
            ("transaction", &self.transaction),
            ("line", &self.line),
        ];
        let mut separator = " (";
        for (label, value) in context {
            if let Some(value) = value {
                write!(f, "{separator}{label} {value}")?;
                separator = ", ";
            }
        }
        if separator == ", " {
            f.write_str(")")?;
        }
        Ok(())
    }
}

/// Errors that can occur while reading an auditfile.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The file could not be read.
    #[error("could not read the file")]
    Io(#[from] std::io::Error),

    /// The file is XAF, but not version 4.0. Holds the namespace of the root element.
    #[error("unsupported XAF version (namespace `{0}`)")]
    UnsupportedVersion(String),

    /// The file is well-formed XML but not an auditfile.
    #[error("not an XAF auditfile: {0}")]
    NotAnAuditfile(String),

    /// The file is not well-formed XML, for example because it is truncated.
    #[error("malformed XML")]
    Xml(quick_xml::Error),

    /// The text encoding of the file is not supported, or the file is not valid in the encoding
    /// it declares.
    #[error("unsupported or invalid text encoding: {0}")]
    Encoding(String),

    /// The XML is well-formed but does not have the structure the XAF 4.0 schema prescribes,
    /// for example because a required element is missing. The message names the element but not
    /// where in the file it was expected.
    #[error("XML does not match the XAF 4.0 schema")]
    Deserialize(#[from] quick_xml::DeError),

    /// Elements are nested deeper than the reader accepts. Holds the limit.
    #[error("elements are nested more than {0} levels deep")]
    NestingTooDeep(usize),

    /// While streaming, an element appeared out of the order the schema prescribes. Holds the
    /// name of the element.
    #[error("element `{0}` is out of order; streaming requires the order of the schema")]
    OutOfOrder(String),

    /// A value could not be interpreted.
    #[error("invalid value {value:?} in {location}: expected {expected}")]
    InvalidValue {
        /// Where the value was found.
        location: Box<Location>,
        /// The value as written in the file.
        value: String,
        /// What the value should have been.
        expected: &'static str,
    },
}

impl From<quick_xml::Error> for Error {
    fn from(error: quick_xml::Error) -> Self {
        match error {
            quick_xml::Error::Encoding(error) => Self::Encoding(error.to_string()),
            error => Self::Xml(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error as _;

    use super::*;

    #[test]
    fn location_without_context_shows_only_the_element() {
        assert_eq!(Location::new("accID").to_string(), "accID");
    }

    #[test]
    fn location_shows_all_context() {
        let location = Location::new("trLine/effDate")
            .in_journal("MEMO")
            .in_transaction("20240001")
            .in_line("2");
        assert_eq!(
            location.to_string(),
            "trLine/effDate (journal MEMO, transaction 20240001, line 2)"
        );
    }

    #[test]
    fn location_shows_the_record() {
        let location = Location::new("obLine/amnt").in_record("2");
        assert_eq!(location.to_string(), "obLine/amnt (record 2)");
    }

    #[test]
    fn location_skips_missing_context() {
        let location = Location::new("trDt").in_journal("MEMO").in_line("3");
        assert_eq!(location.to_string(), "trDt (journal MEMO, line 3)");
    }

    #[test]
    fn invalid_value_names_value_location_and_expectation() {
        let error = Error::InvalidValue {
            location: Box::new(Location::new("trLine/amntTp").in_journal("MEMO")),
            value: "X".into(),
            expected: "`D` or `C`",
        };
        assert_eq!(
            error.to_string(),
            "invalid value \"X\" in trLine/amntTp (journal MEMO): expected `D` or `C`"
        );
    }

    #[test]
    fn unsupported_version_names_the_namespace() {
        let error = Error::UnsupportedVersion("http://example.org/xaf/3.2".into());
        assert_eq!(
            error.to_string(),
            "unsupported XAF version (namespace `http://example.org/xaf/3.2`)"
        );
    }

    #[test]
    fn encoding_error_names_the_problem() {
        let error = Error::Encoding("UTF-16 and UTF-32 are not supported".into());
        assert_eq!(
            error.to_string(),
            "unsupported or invalid text encoding: UTF-16 and UTF-32 are not supported"
        );
    }

    #[test]
    fn io_error_keeps_its_source() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        let error = Error::from(io);
        assert_eq!(error.to_string(), "could not read the file");
        assert_eq!(
            error.source().map(ToString::to_string).as_deref(),
            Some("gone")
        );
    }
}
