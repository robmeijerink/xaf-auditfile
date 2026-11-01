//! Mapping from the schema-shaped structures in [`super::raw`] to the public model.

use std::str::FromStr;

use rust_decimal::Decimal;
use time::{Date, Month};

use super::raw;
use crate::{
    AccountId, AccountKind, Address, Amount, Auditfile, Company, ControlTotals, CountryCode,
    CurrencyCode, Error, FiscalYear, ForeignAmount, Header, Journal, JournalId, JournalKind,
    LedgerAccount, Location, OpeningBalance, OpeningBalanceLine, Period, Relation, RelationId,
    RelationKind, Side, Software, Transaction, TransactionLine, VatCode, VatCodeId, VatLine,
};

pub(super) fn map(raw: raw::Auditfile) -> Result<Auditfile, Error> {
    let ctx = Ctx::default();
    let company = raw.company;

    let transactions = company.transactions;
    let transaction_totals = transactions
        .as_ref()
        .map(|t| {
            ctx.totals(
                [
                    "transactions/linesCount",
                    "transactions/totalDebit",
                    "transactions/totalCredit",
                ],
                [&t.lines_count, &t.total_debit, &t.total_credit],
            )
        })
        .transpose()?;
    let journals = transactions
        .map(|t| {
            t.journal
                .into_iter()
                .map(journal)
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();

    Ok(Auditfile {
        header: header(raw.header)?,
        relations: company
            .customers_suppliers
            .map(|c| c.customer_supplier.into_iter().map(relation).collect())
            .transpose()?
            .unwrap_or_default(),
        accounts: company
            .general_ledger
            .map(|g| g.ledger_account.into_iter().map(account).collect())
            .unwrap_or_default(),
        vat_codes: company
            .vat_codes
            .map(|v| v.vat_code.into_iter().map(vat_code).collect())
            .unwrap_or_default(),
        periods: company
            .periods
            .map(|p| p.period.into_iter().map(period).collect())
            .transpose()?
            .unwrap_or_default(),
        opening_balance: company.opening_balance.map(opening_balance).transpose()?,
        transaction_totals,
        journals,
        company: Company {
            chamber_of_commerce_nr: non_empty(company.commerce_nr),
            name: company.company_name,
            tax_registration_country: ctx.country(
                "company/taxRegistrationCountry",
                &company.tax_registration_country,
            )?,
            tax_registration_id: company.tax_reg_ident,
            addresses: addresses(company.street_address)?,
        },
    })
}

fn header(raw: raw::Header) -> Result<Header, Error> {
    let ctx = Ctx::default();
    Ok(Header {
        fiscal_year: ctx.fiscal_year("header/fiscalYear", &raw.fiscal_year)?,
        start_date: ctx.date("header/startDate", &raw.start_date)?,
        end_date: ctx.date("header/endDate", &raw.end_date)?,
        currency: ctx.currency("header/curCode", &raw.cur_code)?,
        date_created: ctx.date("header/dateCreated", &raw.date_created)?,
        software: Software {
            name: raw.software_desc,
            version: raw.software_version,
        },
        rgs_version: non_empty(raw.rgs_version),
    })
}

fn addresses(raw: Vec<raw::StreetAddress>) -> Result<Vec<Address>, Error> {
    let ctx = Ctx::default();
    raw.into_iter()
        .map(|a| {
            Ok(Address {
                street: non_empty(a.streetname),
                number: non_empty(a.number),
                number_extension: non_empty(a.number_extension),
                city: non_empty(a.city),
                postal_code: non_empty(a.postal_code),
                country: non_empty(a.country)
                    .map(|c| ctx.country("streetAddress/country", &c))
                    .transpose()?,
            })
        })
        .collect()
}

fn relation(raw: raw::CustomerSupplier) -> Result<Relation, Error> {
    let ctx = Ctx {
        record: Some(&raw.cust_sup_id),
        ..Ctx::default()
    };
    Ok(Relation {
        opening_balance: ctx.optional_amount(
            ["customerSupplier/opBalDesc", "customerSupplier/opBalTp"],
            raw.op_bal_desc,
            raw.op_bal_tp,
        )?,
        closing_balance: ctx.optional_amount(
            ["customerSupplier/clBalDesc", "customerSupplier/clBalTp"],
            raw.cl_bal_desc,
            raw.cl_bal_tp,
        )?,
        tax_registration_country: non_empty(raw.tax_registration_country)
            .map(|c| ctx.country("customerSupplier/taxRegistrationCountry", &c))
            .transpose()?,
        addresses: addresses(raw.street_address)?,
        name: non_empty(raw.cust_sup_name),
        email: non_empty(raw.e_mail),
        chamber_of_commerce_nr: non_empty(raw.commerce_nr),
        tax_registration_id: non_empty(raw.tax_reg_ident),
        kind: non_empty(raw.cust_sup_tp).map(|c| RelationKind::from_code(&c)),
        id: RelationId::from(raw.cust_sup_id),
    })
}

fn account(raw: raw::LedgerAccount) -> LedgerAccount {
    LedgerAccount {
        id: AccountId::from(raw.acc_id),
        description: raw.acc_desc,
        kind: AccountKind::from_code(&raw.acc_tp),
        rgs_code: non_empty(raw.rgs_code),
    }
}

fn vat_code(raw: raw::VatCode) -> VatCode {
    VatCode {
        id: VatCodeId::from(raw.vat_id),
        description: raw.vat_desc,
        payable_account: non_empty(raw.vat_to_pay_acc_id).map(AccountId::from),
        claimable_account: non_empty(raw.vat_to_claim_acc_id).map(AccountId::from),
    }
}

fn period(raw: raw::Period) -> Result<Period, Error> {
    let ctx = Ctx {
        record: Some(&raw.period_number),
        ..Ctx::default()
    };
    Ok(Period {
        number: ctx.u16("period/periodNumber", &raw.period_number)?,
        start: ctx.date("period/startDatePeriod", &raw.start_date_period)?,
        end: ctx.date("period/endDatePeriod", &raw.end_date_period)?,
    })
}

fn opening_balance(raw: raw::OpeningBalance) -> Result<OpeningBalance, Error> {
    let totals = Ctx::default().totals(
        [
            "openingBalance/linesCount",
            "openingBalance/totalDebit",
            "openingBalance/totalCredit",
        ],
        [&raw.lines_count, &raw.total_debit, &raw.total_credit],
    )?;
    let lines = raw
        .ob_line
        .into_iter()
        .map(|line| {
            let ctx = Ctx {
                record: Some(&line.nr),
                ..Ctx::default()
            };
            Ok(OpeningBalanceLine {
                amount: ctx.amount("obLine/amnt", &line.amnt, "obLine/amntTp", &line.amnt_tp)?,
                account: AccountId::from(line.acc_id),
                nr: line.nr,
            })
        })
        .collect::<Result<_, Error>>()?;
    Ok(OpeningBalance { totals, lines })
}

pub(super) fn journal(raw: raw::Journal) -> Result<Journal, Error> {
    let transactions = raw
        .transaction
        .into_iter()
        .map(|t| transaction(&raw.jrn_id, t))
        .collect::<Result<_, _>>()?;
    Ok(Journal {
        id: JournalId::from(raw.jrn_id),
        description: raw.desc,
        kind: non_empty(raw.jrn_tp).map(|c| JournalKind::from_code(&c)),
        offset_account: non_empty(raw.offset_acc_id).map(AccountId::from),
        transactions,
    })
}

pub(super) fn transaction(journal: &str, raw: raw::Transaction) -> Result<Transaction, Error> {
    let ctx = Ctx {
        journal: Some(journal),
        transaction: Some(&raw.nr),
        ..Ctx::default()
    };
    let lines = raw
        .tr_line
        .into_iter()
        .map(|line| transaction_line(journal, &raw.nr, line))
        .collect::<Result<_, _>>()?;
    Ok(Transaction {
        period: ctx.u16("transaction/periodNumber", &raw.period_number)?,
        date: ctx.date("transaction/trDt", &raw.tr_dt)?,
        description: non_empty(raw.desc),
        source: non_empty(raw.source),
        user: non_empty(raw.user),
        lines,
        nr: raw.nr,
    })
}

fn transaction_line(
    journal: &str,
    transaction: &str,
    raw: raw::TrLine,
) -> Result<TransactionLine, Error> {
    let ctx = Ctx {
        record: None,
        journal: Some(journal),
        transaction: Some(transaction),
        line: Some(&raw.nr),
    };
    let vat = raw
        .vat
        .into_iter()
        .map(|v| {
            Ok(VatLine {
                percentage: ctx.decimal("trLine/vat/vatPerc", &v.vat_perc)?,
                amount: ctx.amount(
                    "trLine/vat/vatAmnt",
                    &v.vat_amnt,
                    "trLine/vat/vatAmntTp",
                    &v.vat_amnt_tp,
                )?,
                vat_code: VatCodeId::from(v.vat_id),
            })
        })
        .collect::<Result<_, Error>>()?;
    let foreign_currency = raw
        .currency
        .map(|c| {
            Ok::<_, Error>(ForeignAmount {
                currency: ctx.currency("trLine/currency/curCode", &c.cur_code)?,
                amount: ctx.decimal("trLine/currency/curAmnt", &c.cur_amnt)?,
            })
        })
        .transpose()?;
    Ok(TransactionLine {
        effective_date: ctx.date("trLine/effDate", &raw.eff_date)?,
        settlement_date: non_empty(raw.sett_date)
            .map(|d| ctx.date("trLine/settDate", &d))
            .transpose()?,
        amount: ctx.amount("trLine/amnt", &raw.amnt, "trLine/amntTp", &raw.amnt_tp)?,
        account: AccountId::from(raw.acc_id),
        document_ref: raw.doc_ref,
        description: non_empty(raw.desc),
        relation: non_empty(raw.cust_sup_id).map(RelationId::from),
        invoice_ref: non_empty(raw.inv_ref),
        receiving_doc_ref: non_empty(raw.receiving_doc_ref),
        shipping_doc_ref: non_empty(raw.ship_doc_ref),
        cost: non_empty(raw.cost),
        product: non_empty(raw.product),
        project: non_empty(raw.project),
        work_cost_arrangement_ref: non_empty(raw.work_cost_arr_ref),
        bank_account_nr: non_empty(raw.bank_acc_nr),
        offset_bank_account_nr: non_empty(raw.offset_bank_acc_nr),
        vat,
        foreign_currency,
        nr: raw.nr,
    })
}

/// The schema allows at most 20 digits in an amount. Rejecting anything of 21 digits or more
/// keeps sums of amounts far from the limits of [`Decimal`].
const MAX_MAGNITUDE: i128 = 10_i128.pow(20);

/// Longest value, in characters, that an error repeats.
const MAX_ECHOED_CHARS: usize = 64;

/// Keeps an error message small however long the offending value is.
fn shorten(value: &str) -> String {
    match value.char_indices().nth(MAX_ECHOED_CHARS) {
        Some((end, _)) => format!("{}...", value.get(..end).unwrap_or_default()),
        None => value.to_owned(),
    }
}

/// An empty element such as `<bankAccNr/>` carries no information.
fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|v| !v.is_empty())
}

/// Where in the file a value is, so that an error can say so. A [`Location`] is only built when
/// something goes wrong.
#[derive(Debug, Clone, Copy, Default)]
struct Ctx<'a> {
    record: Option<&'a str>,
    journal: Option<&'a str>,
    transaction: Option<&'a str>,
    line: Option<&'a str>,
}

impl Ctx<'_> {
    fn invalid(&self, element: &'static str, value: &str, expected: &'static str) -> Error {
        let mut location = Location::new(element);
        if let Some(record) = self.record {
            location = location.in_record(record);
        }
        if let Some(journal) = self.journal {
            location = location.in_journal(journal);
        }
        if let Some(transaction) = self.transaction {
            location = location.in_transaction(transaction);
        }
        if let Some(line) = self.line {
            location = location.in_line(line);
        }
        Error::InvalidValue {
            location: Box::new(location),
            value: shorten(value),
            expected,
        }
    }

    fn date(&self, element: &'static str, value: &str) -> Result<Date, Error> {
        parse_date(value).ok_or_else(|| self.invalid(element, value, "a date (YYYY-MM-DD)"))
    }

    fn decimal(&self, element: &'static str, value: &str) -> Result<Decimal, Error> {
        Decimal::from_str(value.trim())
            .ok()
            .filter(|decimal| decimal.abs() < Decimal::from_i128_with_scale(MAX_MAGNITUDE, 0))
            .ok_or_else(|| self.invalid(element, value, "a decimal number below 10^20"))
    }

    fn u16(&self, element: &'static str, value: &str) -> Result<u16, Error> {
        value
            .trim()
            .parse()
            .map_err(|_| self.invalid(element, value, "a non-negative whole number"))
    }

    fn u64(&self, element: &'static str, value: &str) -> Result<u64, Error> {
        value
            .trim()
            .parse()
            .map_err(|_| self.invalid(element, value, "a non-negative whole number"))
    }

    fn side(&self, element: &'static str, value: &str) -> Result<Side, Error> {
        Side::from_code(value.trim()).ok_or_else(|| self.invalid(element, value, "`D` or `C`"))
    }

    fn country(&self, element: &'static str, value: &str) -> Result<CountryCode, Error> {
        CountryCode::new(value)
            .ok_or_else(|| self.invalid(element, value, "a two-letter country code"))
    }

    fn currency(&self, element: &'static str, value: &str) -> Result<CurrencyCode, Error> {
        CurrencyCode::new(value)
            .ok_or_else(|| self.invalid(element, value, "a currency code of at most three letters"))
    }

    fn fiscal_year(&self, element: &'static str, value: &str) -> Result<FiscalYear, Error> {
        parse_fiscal_year(value.trim())
            .ok_or_else(|| self.invalid(element, value, "`YYYY` or `YYYY-YYYY`"))
    }

    fn amount(
        &self,
        value_element: &'static str,
        value: &str,
        side_element: &'static str,
        side: &str,
    ) -> Result<Amount, Error> {
        Ok(Amount::new(
            self.decimal(value_element, value)?,
            self.side(side_element, side)?,
        ))
    }

    /// An amount whose value and side are both optional in the schema. A side without a value
    /// says nothing; a value without a side cannot be interpreted.
    fn optional_amount(
        &self,
        [value_element, side_element]: [&'static str; 2],
        value: Option<String>,
        side: Option<String>,
    ) -> Result<Option<Amount>, Error> {
        let Some(value) = non_empty(value) else {
            return Ok(None);
        };
        let side = non_empty(side).unwrap_or_default();
        self.amount(value_element, &value, side_element, &side)
            .map(Some)
    }

    fn totals(
        &self,
        elements: [&'static str; 3],
        values: [&String; 3],
    ) -> Result<ControlTotals, Error> {
        let [count, debit, credit] = elements;
        let [count_value, debit_value, credit_value] = values;
        Ok(ControlTotals {
            lines_count: self.u64(count, count_value)?,
            total_debit: self.decimal(debit, debit_value)?,
            total_credit: self.decimal(credit, credit_value)?,
        })
    }
}

fn parse_fiscal_year(value: &str) -> Option<FiscalYear> {
    match value.split_once('-') {
        None => Some(FiscalYear::Single(digits(value, 4)?)),
        Some((from, to)) => Some(FiscalYear::Span {
            from: digits(from, 4)?,
            to: digits(to, 4)?,
        }),
    }
}

/// Parses an `xsd:date`. The optional time zone is accepted and ignored: the calendar date is
/// what matters.
fn parse_date(value: &str) -> Option<Date> {
    let value = strip_time_zone(value.trim());
    let mut parts = value.split('-');
    let (year, month, day) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() {
        return None;
    }
    let month = Month::try_from(digits::<u8>(month, 2)?).ok()?;
    Date::from_calendar_date(i32::from(digits::<u16>(year, 4)?), month, digits(day, 2)?).ok()
}

fn strip_time_zone(value: &str) -> &str {
    value
        .strip_suffix('Z')
        .or_else(|| strip_offset(value))
        .unwrap_or(value)
}

fn strip_offset(value: &str) -> Option<&str> {
    let (head, tail) = value.split_at_checked(value.len().checked_sub(6)?)?;
    let (hours, minutes) = tail.strip_prefix(['+', '-'])?.split_once(':')?;
    digits::<u8>(hours, 2)?;
    digits::<u8>(minutes, 2)?;
    Some(head)
}

/// Exactly `len` ASCII digits.
fn digits<T: FromStr>(value: &str, len: usize) -> Option<T> {
    if value.len() == len && value.bytes().all(|b| b.is_ascii_digit()) {
        value.parse().ok()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WEBSHOP: &str = include_str!("../../tests/fixtures/webshop.xaf");

    fn map_str(xml: &str) -> Result<Auditfile, Error> {
        map(raw::deserialize(xml.as_bytes())?)
    }

    fn webshop() -> Auditfile {
        map_str(WEBSHOP).unwrap()
    }

    fn webshop_with(from: &str, to: &str) -> String {
        assert!(WEBSHOP.contains(from), "fixture does not contain {from}");
        WEBSHOP.replacen(from, to, 1)
    }

    fn invalid(result: Result<Auditfile, Error>) -> (String, String) {
        match result {
            Err(Error::InvalidValue {
                location, value, ..
            }) => (location.to_string(), value),
            other => panic!("expected an invalid value, got {other:?}"),
        }
    }

    fn date(year: i32, month: Month, day: u8) -> Date {
        Date::from_calendar_date(year, month, day).unwrap()
    }

    #[test]
    fn maps_header() {
        let header = webshop().header;
        assert_eq!(header.fiscal_year, FiscalYear::Single(2024));
        assert_eq!(header.start_date, date(2024, Month::January, 1));
        assert_eq!(header.end_date, date(2024, Month::December, 31));
        assert_eq!(header.currency.as_str(), "EUR");
        assert_eq!(header.date_created, date(2025, Month::February, 1));
        assert_eq!(header.software.name, "Example Books");
        assert_eq!(header.software.version, "1.2.3");
        assert_eq!(header.rgs_version, None);
    }

    #[test]
    fn maps_company() {
        let company = webshop().company;
        assert_eq!(company.name, "Voorbeeld Webshop B.V.");
        assert_eq!(company.chamber_of_commerce_nr.as_deref(), Some("12345678"));
        assert_eq!(company.tax_registration_country.as_str(), "NL");
        assert_eq!(company.tax_registration_id, "NL000000000B01");
        assert_eq!(company.addresses.len(), 1);
        let address = &company.addresses[0];
        assert_eq!(address.street.as_deref(), Some("Voorbeeldstraat"));
        assert_eq!(address.number_extension, None);
        assert_eq!(
            address.country.as_ref().map(CountryCode::as_str),
            Some("NL")
        );
    }

    #[test]
    fn maps_relations() {
        let relations = webshop().relations;
        assert_eq!(relations.len(), 2);
        assert_eq!(relations[0].id.as_str(), "C001");
        assert_eq!(relations[0].email.as_deref(), Some("klant1@example.org"));
        assert_eq!(relations[0].kind, Some(RelationKind::Customer));
        assert_eq!(relations[0].opening_balance, None);
        assert_eq!(relations[0].addresses.len(), 1);
        assert_eq!(
            relations[1].opening_balance,
            Some(Amount::new(Decimal::new(5000, 2), Side::Debit))
        );
        assert_eq!(
            relations[1].closing_balance,
            Some(Amount::new(Decimal::new(0, 2), Side::Debit))
        );
    }

    #[test]
    fn maps_accounts_vat_codes_and_periods() {
        let file = webshop();
        assert_eq!(file.accounts.len(), 5);
        assert_eq!(file.accounts[1].id.as_str(), "1100");
        assert_eq!(file.accounts[1].kind, AccountKind::BalanceSheet);
        assert_eq!(file.accounts[1].rgs_code.as_deref(), Some("BLimBan"));
        assert_eq!(file.accounts[4].kind, AccountKind::ProfitAndLoss);
        assert_eq!(file.vat_codes.len(), 2);
        assert_eq!(
            file.vat_codes[0].payable_account,
            Some(AccountId::from("1800"))
        );
        assert_eq!(file.vat_codes[0].claimable_account, None);
        assert_eq!(file.periods.len(), 2);
        assert_eq!(file.periods[1].number, 2);
        assert_eq!(file.periods[1].end, date(2024, Month::February, 29));
    }

    #[test]
    fn maps_opening_balance() {
        let balance = webshop().opening_balance.unwrap();
        assert_eq!(balance.totals.lines_count, 2);
        assert_eq!(balance.totals.total_debit, Decimal::new(100_000, 2));
        assert_eq!(balance.lines.len(), 2);
        assert_eq!(balance.lines[1].account.as_str(), "0500");
        assert_eq!(balance.lines[1].amount.signed(), Decimal::new(-100_000, 2));
    }

    #[test]
    fn maps_journals_and_transactions() {
        let file = webshop();
        let totals = file.transaction_totals.unwrap();
        assert_eq!(totals.lines_count, 8);
        assert_eq!(totals.total_credit, Decimal::new(39_650, 2));
        assert_eq!(file.journals.len(), 2);
        assert_eq!(file.journals[0].kind, Some(JournalKind::Sales));
        assert_eq!(file.journals[1].kind, Some(JournalKind::Bank));
        assert_eq!(
            file.journals[1].offset_account,
            Some(AccountId::from("1100"))
        );
        let transaction = &file.journals[0].transactions[0];
        assert_eq!(transaction.nr, "1001");
        assert_eq!(transaction.period, 1);
        assert_eq!(transaction.date, date(2024, Month::January, 10));
        assert_eq!(transaction.source.as_deref(), Some("webshop"));
        assert_eq!(transaction.user, None);
        assert_eq!(transaction.lines.len(), 3);
    }

    #[test]
    fn maps_transaction_line() {
        let file = webshop();
        let line = &file.journals[0].transactions[1].lines[0];
        assert_eq!(line.nr, "1");
        assert_eq!(line.account.as_str(), "1300");
        assert_eq!(line.document_ref, "5002");
        assert_eq!(line.effective_date, date(2024, Month::January, 20));
        assert_eq!(line.settlement_date, Some(date(2024, Month::February, 3)));
        assert_eq!(
            line.amount,
            Amount::new(Decimal::new(17_550, 2), Side::Debit)
        );
        assert_eq!(line.relation, Some(RelationId::from("C002")));
        assert_eq!(line.invoice_ref, None);
        assert!(line.vat.is_empty());
        assert_eq!(line.foreign_currency, None);
    }

    #[test]
    fn maps_several_vat_entries_on_one_line() {
        let file = webshop();
        let line = &file.journals[0].transactions[1].lines[1];
        assert_eq!(line.vat.len(), 2);
        assert_eq!(line.vat[0].vat_code.as_str(), "HOOG");
        assert_eq!(line.vat[0].percentage, Decimal::from(21));
        assert_eq!(line.vat[1].vat_code.as_str(), "LAAG");
        assert_eq!(line.vat[1].percentage, Decimal::from(9));
        assert_eq!(
            line.vat[1].amount,
            Amount::new(Decimal::new(450, 2), Side::Credit)
        );
    }

    #[test]
    fn maps_foreign_currency_and_drops_empty_elements() {
        let file = webshop();
        let line = &file.journals[1].transactions[0].lines[0];
        let foreign = line.foreign_currency.as_ref().unwrap();
        assert_eq!(foreign.currency.as_str(), "USD");
        assert_eq!(foreign.amount, Decimal::new(10_800, 2));
        assert_eq!(line.bank_account_nr, None);
    }

    #[test]
    fn keeps_codes_outside_the_lists() {
        let xml = webshop_with("<jrnTp>S</jrnTp>", "<jrnTp>Q</jrnTp>").replacen(
            "<accTp>P</accTp>",
            "<accTp>X</accTp>",
            1,
        );
        let file = map_str(&xml).unwrap();
        assert_eq!(
            file.journals[0].kind,
            Some(JournalKind::Unknown("Q".into()))
        );
        assert_eq!(file.accounts[4].kind, AccountKind::Unknown("X".into()));
    }

    #[test]
    fn empty_optional_date_is_absent() {
        let xml = webshop_with("<settDate>2024-02-03</settDate>", "<settDate/>");
        let file = map_str(&xml).unwrap();
        assert_eq!(
            file.journals[0].transactions[1].lines[0].settlement_date,
            None
        );
    }

    #[test]
    fn invalid_transaction_date_names_journal_and_transaction() {
        let xml = webshop_with("<trDt>2024-01-10</trDt>", "<trDt>2024-13-10</trDt>");
        assert_eq!(
            invalid(map_str(&xml)),
            (
                "transaction/trDt (journal VERK, transaction 1001)".into(),
                "2024-13-10".into()
            )
        );
    }

    #[test]
    fn invalid_amount_names_the_line() {
        let xml = webshop_with("<amnt>121.00</amnt>", "<amnt>12,1</amnt>");
        assert_eq!(
            invalid(map_str(&xml)),
            (
                "trLine/amnt (journal VERK, transaction 1001, line 1)".into(),
                "12,1".into()
            )
        );
    }

    #[test]
    fn invalid_side_names_the_line() {
        let xml = webshop_with("<amntTp>D</amntTp>", "<amntTp>X</amntTp>");
        let (location, value) = invalid(map_str(&xml));
        assert!(
            location.starts_with("obLine/amntTp (record 1)"),
            "{location}"
        );
        assert_eq!(value, "X");
    }

    #[test]
    fn invalid_vat_percentage_names_the_line() {
        let xml = webshop_with("<vatPerc>21</vatPerc>", "<vatPerc>abc</vatPerc>");
        assert_eq!(
            invalid(map_str(&xml)).0,
            "trLine/vat/vatPerc (journal VERK, transaction 1001, line 2)"
        );
    }

    #[test]
    fn invalid_period_number_names_the_record() {
        let xml = webshop_with(
            "<periodNumber>2</periodNumber>",
            "<periodNumber>-2</periodNumber>",
        );
        assert_eq!(invalid(map_str(&xml)).0, "period/periodNumber (record -2)");
    }

    #[test]
    fn amount_of_more_than_twenty_digits_is_rejected() {
        let xml = webshop_with("<amnt>121.00</amnt>", "<amnt>100000000000000000000</amnt>");
        assert_eq!(
            invalid(map_str(&xml)).0,
            "trLine/amnt (journal VERK, transaction 1001, line 1)"
        );
        let xml = webshop_with(
            "<amnt>121.00</amnt>",
            "<amnt>99999999999999999999.99</amnt>",
        );
        assert!(map_str(&xml).is_ok());
    }

    #[test]
    fn echoed_value_is_shortened() {
        let long = "9".repeat(10_000);
        let xml = webshop_with("<amnt>121.00</amnt>", &format!("<amnt>{long}</amnt>"));
        let (_, value) = invalid(map_str(&xml));
        assert_eq!(value.chars().count(), 67);
        assert!(value.ends_with("..."));
        assert_eq!(shorten("short"), "short");
        assert_eq!(shorten(&"é".repeat(64)), "é".repeat(64));
    }

    #[test]
    fn invalid_control_total_is_reported() {
        let xml = webshop_with(
            "<linesCount>8</linesCount>",
            "<linesCount>eight</linesCount>",
        );
        assert_eq!(invalid(map_str(&xml)).0, "transactions/linesCount");
    }

    #[test]
    fn invalid_country_code_is_reported() {
        let xml = webshop_with(
            "<taxRegistrationCountry>NL</taxRegistrationCountry>\n    <taxRegIdent>NL000000000B01",
            "<taxRegistrationCountry>NLD</taxRegistrationCountry>\n    <taxRegIdent>NL000000000B01",
        );
        assert_eq!(
            invalid(map_str(&xml)),
            ("company/taxRegistrationCountry".into(), "NLD".into())
        );
    }

    #[test]
    fn relation_balance_without_side_is_reported() {
        let xml = webshop_with("<opBalTp>D</opBalTp>", "");
        assert_eq!(
            invalid(map_str(&xml)).0,
            "customerSupplier/opBalTp (record C002)"
        );
    }

    #[test]
    fn relation_balance_side_without_value_is_ignored() {
        let xml = webshop_with("<opBalDesc>50.00</opBalDesc>", "");
        assert_eq!(map_str(&xml).unwrap().relations[1].opening_balance, None);
    }

    #[test]
    fn parses_fiscal_years() {
        assert_eq!(parse_fiscal_year("2024"), Some(FiscalYear::Single(2024)));
        assert_eq!(
            parse_fiscal_year("2024-2025"),
            Some(FiscalYear::Span {
                from: 2024,
                to: 2025
            })
        );
        for bad in ["", "24", "2024-25", "2024-", "2024-2025-2026", "20x4"] {
            assert_eq!(parse_fiscal_year(bad), None, "{bad}");
        }
    }

    #[test]
    fn parses_dates_with_and_without_time_zone() {
        let expected = Some(date(2024, Month::February, 29));
        for good in [
            "2024-02-29",
            "2024-02-29Z",
            "2024-02-29+01:00",
            "2024-02-29-05:30",
            " 2024-02-29 ",
        ] {
            assert_eq!(parse_date(good), expected, "{good}");
        }
    }

    #[test]
    fn rejects_malformed_dates() {
        for bad in [
            "",
            "2024",
            "2024-1-1",
            "24-01-01",
            "2024-13-01",
            "2023-02-29",
            "2024-01-32",
            "2024/01/01",
            "2024-01-01T10:00:00",
            "2024-01-01+1:00",
            "-2024-01-01",
            "2024-01-01-",
        ] {
            assert_eq!(parse_date(bad), None, "{bad}");
        }
    }
}
