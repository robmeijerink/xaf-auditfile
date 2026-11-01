use time::Date;

use super::ids::CurrencyCode;

/// Fiscal year (`fiscalYear`): `2024` or a broken year such as `2024-2025`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FiscalYear {
    /// A single calendar year.
    Single(u16),
    /// A fiscal year that spans two calendar years.
    Span {
        /// First calendar year.
        from: u16,
        /// Last calendar year.
        to: u16,
    },
}

/// Software that produced the file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Software {
    /// Name of the software (`softwareDesc`).
    pub name: String,
    /// Version of the software (`softwareVersion`).
    pub version: String,
}

/// The `header` element.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Header {
    /// Fiscal year (`fiscalYear`).
    pub fiscal_year: FiscalYear,
    /// First day of the reporting period (`startDate`).
    pub start_date: Date,
    /// Last day of the reporting period (`endDate`).
    pub end_date: Date,
    /// Currency of the administration (`curCode`).
    pub currency: CurrencyCode,
    /// Date the file was created (`dateCreated`).
    pub date_created: Date,
    /// Software that produced the file.
    pub software: Software,
    /// Version of the RGS reference scheme in use (`RGSVersion`).
    pub rgs_version: Option<String>,
}

/// A booking period (`period`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Period {
    /// Period number (`periodNumber`).
    pub number: u16,
    /// First day of the period (`startDatePeriod`).
    pub start: Date,
    /// Last day of the period (`endDatePeriod`).
    pub end: Date,
}
