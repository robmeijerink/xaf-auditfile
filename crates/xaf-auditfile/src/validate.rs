use std::collections::HashSet;
use std::fmt;

use rust_decimal::Decimal;

use crate::{
    AccountId, Auditfile, ControlTotals, Journal, JournalId, Location, RelationId, Side,
    Transaction, TransactionLine, VatCodeId,
};

/// A problem found by [`Auditfile::validate`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Finding {
    /// What is wrong.
    pub kind: FindingKind,
    /// Where it was found.
    pub location: Location,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.location, self.kind)
    }
}

/// The kinds of problems [`Auditfile::validate`] reports.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FindingKind {
    /// The debit and credit amounts of a transaction differ.
    UnbalancedTransaction {
        /// Sum of the debit amounts.
        debit: Decimal,
        /// Sum of the credit amounts.
        credit: Decimal,
    },
    /// The debit and credit amounts of the opening balance differ.
    UnbalancedOpeningBalance {
        /// Sum of the debit amounts.
        debit: Decimal,
        /// Sum of the credit amounts.
        credit: Decimal,
    },
    /// A reference to a ledger account that is not in the general ledger.
    UnknownAccount(AccountId),
    /// A reference to a VAT code that is not in the list of VAT codes.
    UnknownVatCode(VatCodeId),
    /// A reference to a customer or supplier that is not in the list of relations.
    UnknownRelation(RelationId),
    /// A transaction in a period that is not in the list of periods.
    UnknownPeriod(u16),
    /// A ledger account identifier that occurs more than once.
    DuplicateAccount(AccountId),
    /// A VAT code identifier that occurs more than once.
    DuplicateVatCode(VatCodeId),
    /// A customer or supplier identifier that occurs more than once.
    DuplicateRelation(RelationId),
    /// An RGS code that is linked to more than one ledger account.
    DuplicateRgsCode(String),
    /// A transaction number that occurs more than once in the same journal.
    DuplicateTransactionNr(String),
    /// The control totals in the file differ from the totals of the lines.
    ControlTotalMismatch {
        /// The totals as stated in the file.
        stated: ControlTotals,
        /// The totals of the lines.
        actual: ControlTotals,
    },
}

impl fmt::Display for FindingKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnbalancedTransaction { debit, credit } => {
                write!(
                    f,
                    "transaction is not in balance (debit {debit}, credit {credit})"
                )
            }
            Self::UnbalancedOpeningBalance { debit, credit } => {
                write!(
                    f,
                    "opening balance is not in balance (debit {debit}, credit {credit})"
                )
            }
            Self::UnknownAccount(id) => write!(f, "unknown ledger account `{id}`"),
            Self::UnknownVatCode(id) => write!(f, "unknown VAT code `{id}`"),
            Self::UnknownRelation(id) => write!(f, "unknown customer or supplier `{id}`"),
            Self::UnknownPeriod(number) => write!(f, "unknown period {number}"),
            Self::DuplicateAccount(id) => write!(f, "ledger account `{id}` occurs more than once"),
            Self::DuplicateVatCode(id) => write!(f, "VAT code `{id}` occurs more than once"),
            Self::DuplicateRelation(id) => {
                write!(f, "customer or supplier `{id}` occurs more than once")
            }
            Self::DuplicateRgsCode(code) => {
                write!(f, "RGS code `{code}` is linked to more than one account")
            }
            Self::DuplicateTransactionNr(nr) => {
                write!(
                    f,
                    "transaction number `{nr}` occurs more than once in the journal"
                )
            }
            Self::ControlTotalMismatch { stated, actual } => write!(
                f,
                "control totals do not match the lines (stated {} lines, debit {}, credit {}; \
                 actual {} lines, debit {}, credit {})",
                stated.lines_count,
                stated.total_debit,
                stated.total_credit,
                actual.lines_count,
                actual.total_debit,
                actual.total_credit,
            ),
        }
    }
}

impl Auditfile {
    /// Checks the file for inconsistencies that the schema does not prevent.
    ///
    /// Reports duplicate identifiers, references to accounts, VAT codes, customers, suppliers and
    /// periods that do not exist, transactions and the opening balance that are not in balance,
    /// and control totals that do not match the lines. References are checked against the lists
    /// in the file, so a file without such a list has every reference to it reported.
    ///
    /// An inconsistent file is not an error: the result is a list of findings, and is empty when
    /// nothing was found. The findings about the master data come first, followed by those about
    /// the journals in document order.
    pub fn validate(&self) -> Vec<Finding> {
        let mut findings = self.validate_master_data();
        let mut validator = self.validator();
        for journal in &self.journals {
            findings.extend(validator.check_journal(journal));
            for transaction in &journal.transactions {
                findings.extend(validator.check(journal, transaction));
            }
        }
        findings.extend(validator.finish());
        findings
    }

    /// Checks everything except the journals and their transactions: duplicate accounts, RGS
    /// codes, VAT codes and customers or suppliers, the accounts that VAT codes refer to, and the
    /// opening balance.
    ///
    /// [`Auditfile::validate`] includes this. It is separate for a file that is read with
    /// [`Auditfile::stream_from_reader`], where it is combined with a [`Validator`].
    pub fn validate_master_data(&self) -> Vec<Finding> {
        let mut findings = Vec::new();
        self.check_duplicates(&mut findings);
        self.check_vat_code_accounts(&mut findings);
        self.check_opening_balance(&mut findings);
        findings
    }

    /// Creates a [`Validator`] for the journals of this file, using its master data.
    pub fn validator(&self) -> Validator {
        Validator {
            accounts: self
                .accounts
                .iter()
                .map(|a| a.id.as_str().to_owned())
                .collect(),
            vat_codes: self
                .vat_codes
                .iter()
                .map(|v| v.id.as_str().to_owned())
                .collect(),
            relations: self
                .relations
                .iter()
                .map(|r| r.id.as_str().to_owned())
                .collect(),
            periods: self.periods.iter().map(|p| p.number).collect(),
            stated_totals: self.transaction_totals.clone(),
            lines_count: 0,
            total_debit: Decimal::ZERO,
            total_credit: Decimal::ZERO,
            journal: None,
            transaction_nrs: HashSet::new(),
        }
    }

    fn check_duplicates(&self, findings: &mut Vec<Finding>) {
        for account in repeated(&self.accounts, |a| Some(a.id.as_str())) {
            findings.push(Finding {
                kind: FindingKind::DuplicateAccount(account.id.clone()),
                location: Location::new("ledgerAccount/accID").in_record(account.id.as_str()),
            });
        }
        for account in repeated(&self.accounts, |a| a.rgs_code.as_deref()) {
            let code = account.rgs_code.clone().unwrap_or_default();
            findings.push(Finding {
                kind: FindingKind::DuplicateRgsCode(code),
                location: Location::new("ledgerAccount/RGScode").in_record(account.id.as_str()),
            });
        }
        for vat_code in repeated(&self.vat_codes, |v| Some(v.id.as_str())) {
            findings.push(Finding {
                kind: FindingKind::DuplicateVatCode(vat_code.id.clone()),
                location: Location::new("vatCode/vatID").in_record(vat_code.id.as_str()),
            });
        }
        for relation in repeated(&self.relations, |r| Some(r.id.as_str())) {
            findings.push(Finding {
                kind: FindingKind::DuplicateRelation(relation.id.clone()),
                location: Location::new("customerSupplier/custSupID")
                    .in_record(relation.id.as_str()),
            });
        }
    }

    fn check_vat_code_accounts(&self, findings: &mut Vec<Finding>) {
        let accounts = self.account_ids();
        for vat_code in &self.vat_codes {
            let references = [
                ("vatCode/vatToPayAccID", &vat_code.payable_account),
                ("vatCode/vatToClaimAccID", &vat_code.claimable_account),
            ];
            for (element, account) in references {
                if let Some(account) = account.as_ref().filter(|a| !accounts.contains(a.as_str())) {
                    findings.push(Finding {
                        kind: FindingKind::UnknownAccount(account.clone()),
                        location: Location::new(element).in_record(vat_code.id.as_str()),
                    });
                }
            }
        }
    }

    fn check_opening_balance(&self, findings: &mut Vec<Finding>) {
        let Some(balance) = &self.opening_balance else {
            return;
        };
        let accounts = self.account_ids();
        for line in &balance.lines {
            if !accounts.contains(line.account.as_str()) {
                findings.push(Finding {
                    kind: FindingKind::UnknownAccount(line.account.clone()),
                    location: Location::new("obLine/accID").in_record(line.nr.as_str()),
                });
            }
        }

        let (debit, credit) = sums(balance.lines.iter().map(|l| &l.amount));
        if debit != credit {
            findings.push(Finding {
                kind: FindingKind::UnbalancedOpeningBalance { debit, credit },
                location: Location::new("openingBalance"),
            });
        }
        let actual = ControlTotals {
            lines_count: balance.lines.len() as u64,
            total_debit: debit,
            total_credit: credit,
        };
        if balance.totals != actual {
            findings.push(Finding {
                kind: FindingKind::ControlTotalMismatch {
                    stated: balance.totals.clone(),
                    actual,
                },
                location: Location::new("openingBalance"),
            });
        }
    }

    fn account_ids(&self) -> HashSet<&str> {
        self.accounts.iter().map(|a| a.id.as_str()).collect()
    }
}

/// Checks the journals and transactions of a file one at a time.
///
/// [`Auditfile::validate`] uses this on a complete model. It exists separately for a file that is
/// streamed, where the transactions are not all in memory: create it from the master data with
/// [`Auditfile::validator`], pass every transaction to [`Validator::check`] and call
/// [`Validator::finish`] at the end. The findings are then the same as those of
/// [`Auditfile::validate`] together with [`Auditfile::validate_master_data`].
///
/// It keeps the master data it checks against, the running control totals, and the transaction
/// numbers of the current journal.
#[derive(Debug, Clone)]
pub struct Validator {
    accounts: HashSet<String>,
    vat_codes: HashSet<String>,
    relations: HashSet<String>,
    periods: HashSet<u16>,
    stated_totals: Option<ControlTotals>,
    lines_count: u64,
    total_debit: Decimal,
    total_credit: Decimal,
    journal: Option<JournalId>,
    transaction_nrs: HashSet<String>,
}

impl Validator {
    /// Checks the journal itself and starts a new journal for the duplicate transaction numbers.
    ///
    /// [`Validator::check`] does this when the journal changes, so calling it is only needed for
    /// a journal without transactions.
    pub fn check_journal(&mut self, journal: &Journal) -> Vec<Finding> {
        self.journal = Some(journal.id.clone());
        self.transaction_nrs.clear();
        let mut findings = Vec::new();
        if let Some(account) = journal
            .offset_account
            .as_ref()
            .filter(|a| !self.accounts.contains(a.as_str()))
        {
            findings.push(Finding {
                kind: FindingKind::UnknownAccount(account.clone()),
                location: Location::new("journal/offsetAccID").in_journal(journal.id.as_str()),
            });
        }
        findings
    }

    /// Checks a transaction of the given journal. Transactions must be passed in document order,
    /// those of one journal together.
    pub fn check(&mut self, journal: &Journal, transaction: &Transaction) -> Vec<Finding> {
        let mut findings = if self.journal.as_ref() == Some(&journal.id) {
            Vec::new()
        } else {
            self.check_journal(journal)
        };

        if !self.transaction_nrs.insert(transaction.nr.clone()) {
            findings.push(Finding {
                kind: FindingKind::DuplicateTransactionNr(transaction.nr.clone()),
                location: transaction_location("transaction/nr", journal, transaction),
            });
        }
        if !self.periods.contains(&transaction.period) {
            findings.push(Finding {
                kind: FindingKind::UnknownPeriod(transaction.period),
                location: transaction_location("transaction/periodNumber", journal, transaction),
            });
        }
        for line in &transaction.lines {
            self.check_line(journal, transaction, line, &mut findings);
        }

        let (debit, credit) = sums(transaction.lines.iter().map(|l| &l.amount));
        if debit != credit {
            findings.push(Finding {
                kind: FindingKind::UnbalancedTransaction { debit, credit },
                location: transaction_location("transaction", journal, transaction),
            });
        }
        self.lines_count = self
            .lines_count
            .saturating_add(transaction.lines.len() as u64);
        self.total_debit = add(self.total_debit, debit);
        self.total_credit = add(self.total_credit, credit);
        findings
    }

    /// Compares the control totals stated in the file with the totals of the transactions that
    /// were checked.
    pub fn finish(self) -> Vec<Finding> {
        let Some(stated) = self.stated_totals else {
            return Vec::new();
        };
        let actual = ControlTotals {
            lines_count: self.lines_count,
            total_debit: self.total_debit,
            total_credit: self.total_credit,
        };
        if stated == actual {
            return Vec::new();
        }
        vec![Finding {
            kind: FindingKind::ControlTotalMismatch { stated, actual },
            location: Location::new("transactions"),
        }]
    }

    fn check_line(
        &self,
        journal: &Journal,
        transaction: &Transaction,
        line: &TransactionLine,
        findings: &mut Vec<Finding>,
    ) {
        let at = |element| line_location(element, journal, transaction, line);
        if !self.accounts.contains(line.account.as_str()) {
            findings.push(Finding {
                kind: FindingKind::UnknownAccount(line.account.clone()),
                location: at("trLine/accID"),
            });
        }
        if let Some(relation) = line
            .relation
            .as_ref()
            .filter(|r| !self.relations.contains(r.as_str()))
        {
            findings.push(Finding {
                kind: FindingKind::UnknownRelation(relation.clone()),
                location: at("trLine/custSupID"),
            });
        }
        for vat in line
            .vat
            .iter()
            .filter(|v| !self.vat_codes.contains(v.vat_code.as_str()))
        {
            findings.push(Finding {
                kind: FindingKind::UnknownVatCode(vat.vat_code.clone()),
                location: at("trLine/vat/vatID"),
            });
        }
    }
}

/// The items whose key was already used by an earlier item.
fn repeated<'a, T>(
    items: &'a [T],
    key: impl Fn(&'a T) -> Option<&'a str> + 'a,
) -> impl Iterator<Item = &'a T> {
    let mut seen = HashSet::new();
    items
        .iter()
        .filter(move |item| key(item).is_some_and(|key| !seen.insert(key)))
}

/// Total debit and total credit of the amounts.
fn sums<'a>(amounts: impl Iterator<Item = &'a crate::Amount>) -> (Decimal, Decimal) {
    amounts.fold(
        (Decimal::ZERO, Decimal::ZERO),
        |(debit, credit), amount| match amount.side {
            Side::Debit => (add(debit, amount.value), credit),
            Side::Credit => (debit, add(credit, amount.value)),
        },
    )
}

/// Addition that stops at the limits of [`Decimal`] instead of panicking. Amounts are limited
/// when a file is read, so this is only reached by a file with hundreds of millions of lines.
fn add(a: Decimal, b: Decimal) -> Decimal {
    a.checked_add(b).unwrap_or(if b.is_sign_negative() {
        Decimal::MIN
    } else {
        Decimal::MAX
    })
}

fn transaction_location(
    element: &'static str,
    journal: &Journal,
    transaction: &Transaction,
) -> Location {
    Location::new(element)
        .in_journal(journal.id.as_str())
        .in_transaction(transaction.nr.as_str())
}

fn line_location(
    element: &'static str,
    journal: &Journal,
    transaction: &Transaction,
    line: &TransactionLine,
) -> Location {
    transaction_location(element, journal, transaction).in_line(line.nr.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addition_stops_at_the_limits_instead_of_panicking() {
        assert_eq!(add(Decimal::MAX, Decimal::ONE), Decimal::MAX);
        assert_eq!(add(Decimal::MIN, -Decimal::ONE), Decimal::MIN);
        assert_eq!(add(Decimal::ONE, Decimal::ONE), Decimal::TWO);
    }
}
