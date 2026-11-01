#![allow(clippy::unwrap_used)]

use rust_decimal::Decimal;
use xaf_auditfile::{AccountId, Auditfile, Finding, FindingKind, RelationId, VatCodeId};

const WEBSHOP: &str = include_str!("fixtures/webshop.xaf");

fn validate(xml: &str) -> Vec<Finding> {
    Auditfile::from_reader(xml.as_bytes()).unwrap().validate()
}

fn webshop_with(from: &str, to: &str) -> String {
    assert!(WEBSHOP.contains(from), "fixture does not contain {from}");
    WEBSHOP.replacen(from, to, 1)
}

fn only(findings: Vec<Finding>) -> (FindingKind, String) {
    assert_eq!(findings.len(), 1, "{findings:#?}");
    let finding = findings.into_iter().next().unwrap();
    (finding.kind, finding.location.to_string())
}

fn decimal(value: i64) -> Decimal {
    Decimal::new(value, 2)
}

#[test]
fn consistent_file_has_no_findings() {
    assert_eq!(validate(WEBSHOP), []);
}

#[test]
fn validating_twice_gives_the_same_result() {
    let file =
        Auditfile::from_reader(webshop_with("<amnt>100.00</amnt>", "<amnt>1.00</amnt>").as_bytes())
            .unwrap();
    assert_eq!(file.validate(), file.validate());
}

#[test]
fn unbalanced_transaction_is_reported_with_both_sums() {
    let xml = webshop_with(
        "<amnt>100.00</amnt>\n            <amntTp>C</amntTp>",
        "<amnt>99.00</amnt>\n            <amntTp>C</amntTp>",
    );
    let findings = validate(&xml);
    assert_eq!(findings.len(), 2, "{findings:#?}");
    assert_eq!(
        findings[0].kind,
        FindingKind::UnbalancedTransaction {
            debit: decimal(12_100),
            credit: decimal(12_000),
        }
    );
    assert_eq!(
        findings[0].location.to_string(),
        "transaction (journal VERK, transaction 1001)"
    );
    assert!(matches!(
        findings[1].kind,
        FindingKind::ControlTotalMismatch { .. }
    ));
}

#[test]
fn unknown_account_on_a_line() {
    let xml = webshop_with(
        "<accID>8000</accID>\n            <docRef>5001",
        "<accID>8999</accID>\n            <docRef>5001",
    );
    assert_eq!(
        only(validate(&xml)),
        (
            FindingKind::UnknownAccount(AccountId::from("8999")),
            "trLine/accID (journal VERK, transaction 1001, line 2)".into()
        )
    );
}

#[test]
fn unknown_vat_code_on_a_line() {
    let xml = webshop_with(
        "<vatID>LAAG</vatID>\n              <vatPerc>",
        "<vatID>ONBEKEND</vatID>\n              <vatPerc>",
    );
    assert_eq!(
        only(validate(&xml)),
        (
            FindingKind::UnknownVatCode(VatCodeId::from("ONBEKEND")),
            "trLine/vat/vatID (journal VERK, transaction 1002, line 2)".into()
        )
    );
}

#[test]
fn unknown_relation_on_a_line() {
    let xml = webshop_with(
        "<custSupID>C001</custSupID>\n            <invRef>",
        "<custSupID>C999</custSupID>\n            <invRef>",
    );
    assert_eq!(
        only(validate(&xml)),
        (
            FindingKind::UnknownRelation(RelationId::from("C999")),
            "trLine/custSupID (journal VERK, transaction 1001, line 1)".into()
        )
    );
}

#[test]
fn unknown_period_on_a_transaction() {
    let xml = webshop_with(
        "<periodNumber>2</periodNumber>\n          <trDt>2024-02-05",
        "<periodNumber>9</periodNumber>\n          <trDt>2024-02-05",
    );
    assert_eq!(
        only(validate(&xml)),
        (
            FindingKind::UnknownPeriod(9),
            "transaction/periodNumber (journal BANK, transaction 2001)".into()
        )
    );
}

#[test]
fn unknown_accounts_in_master_data() {
    let xml = webshop_with(
        "<vatToPayAccID>1800</vatToPayAccID>",
        "<vatToPayAccID>1810</vatToPayAccID>",
    )
    .replacen(
        "<offsetAccID>1100</offsetAccID>",
        "<offsetAccID>9999</offsetAccID>",
        1,
    );
    let findings = validate(&xml);
    let found: Vec<_> = findings
        .iter()
        .map(|f| (f.kind.clone(), f.location.to_string()))
        .collect();
    assert_eq!(
        found,
        [
            (
                FindingKind::UnknownAccount(AccountId::from("1810")),
                "vatCode/vatToPayAccID (record HOOG)".into()
            ),
            (
                FindingKind::UnknownAccount(AccountId::from("9999")),
                "journal/offsetAccID (journal BANK)".into()
            ),
        ]
    );
}

#[test]
fn unknown_account_in_the_opening_balance() {
    let xml = webshop_with(
        "<accID>0500</accID>\n        <amnt>1000.00</amnt>",
        "<accID>0501</accID>\n        <amnt>1000.00</amnt>",
    );
    assert_eq!(
        only(validate(&xml)),
        (
            FindingKind::UnknownAccount(AccountId::from("0501")),
            "obLine/accID (record 2)".into()
        )
    );
}

#[test]
fn duplicate_identifiers_are_reported_once_per_extra_occurrence() {
    let xml = webshop_with(
        "<accID>1100</accID>\n        <accDesc>Bank",
        "<accID>1300</accID>\n        <accDesc>Bank",
    )
    .replacen(
        "<vatID>LAAG</vatID>\n        <vatDesc>",
        "<vatID>HOOG</vatID>\n        <vatDesc>",
        1,
    )
    .replacen(
        "<custSupID>C002</custSupID>\n        <custSupName>",
        "<custSupID>C001</custSupID>\n        <custSupName>",
        1,
    );
    let kinds: Vec<_> = validate(&xml).into_iter().map(|f| f.kind).collect();
    assert!(kinds.contains(&FindingKind::DuplicateAccount(AccountId::from("1300"))));
    assert!(kinds.contains(&FindingKind::DuplicateVatCode(VatCodeId::from("HOOG"))));
    assert!(kinds.contains(&FindingKind::DuplicateRelation(RelationId::from("C001"))));
}

#[test]
fn duplicate_rgs_code() {
    let xml = webshop_with(
        "<accDesc>Debiteuren</accDesc>\n        <accTp>B</accTp>",
        "<accDesc>Debiteuren</accDesc>\n        <accTp>B</accTp>\n        <RGScode>BLimBan</RGScode>",
    );
    assert_eq!(
        only(validate(&xml)),
        (
            FindingKind::DuplicateRgsCode("BLimBan".into()),
            "ledgerAccount/RGScode (record 1300)".into()
        )
    );
}

#[test]
fn accounts_without_rgs_code_are_not_duplicates() {
    assert!(
        validate(WEBSHOP)
            .iter()
            .all(|f| !matches!(f.kind, FindingKind::DuplicateRgsCode(_)))
    );
}

#[test]
fn duplicate_transaction_number_within_a_journal() {
    let xml = webshop_with(
        "<nr>1002</nr>\n          <periodNumber>",
        "<nr>1001</nr>\n          <periodNumber>",
    );
    assert_eq!(
        only(validate(&xml)),
        (
            FindingKind::DuplicateTransactionNr("1001".into()),
            "transaction/nr (journal VERK, transaction 1001)".into()
        )
    );
}

#[test]
fn same_transaction_number_in_different_journals_is_fine() {
    let xml = webshop_with(
        "<nr>2001</nr>\n          <periodNumber>",
        "<nr>1001</nr>\n          <periodNumber>",
    );
    assert_eq!(validate(&xml), []);
}

#[test]
fn unbalanced_opening_balance() {
    let xml = webshop_with(
        "<amnt>1000.00</amnt>\n        <amntTp>C</amntTp>",
        "<amnt>900.00</amnt>\n        <amntTp>C</amntTp>",
    );
    let findings = validate(&xml);
    assert_eq!(findings.len(), 2, "{findings:#?}");
    assert_eq!(
        findings[0].kind,
        FindingKind::UnbalancedOpeningBalance {
            debit: decimal(100_000),
            credit: decimal(90_000),
        }
    );
    assert!(matches!(
        findings[1].kind,
        FindingKind::ControlTotalMismatch { .. }
    ));
}

#[test]
fn control_total_mismatch_names_stated_and_actual() {
    let xml = webshop_with("<linesCount>8</linesCount>", "<linesCount>9</linesCount>");
    let (kind, location) = only(validate(&xml));
    assert_eq!(location, "transactions");
    let FindingKind::ControlTotalMismatch { stated, actual } = kind else {
        panic!("unexpected kind");
    };
    assert_eq!((stated.lines_count, actual.lines_count), (9, 8));
    assert_eq!(stated.total_debit, actual.total_debit);
}

#[test]
fn control_totals_ignore_trailing_zeros() {
    let xml = webshop_with(
        "<totalDebit>396.50</totalDebit>",
        "<totalDebit>396.5000</totalDebit>",
    );
    assert_eq!(validate(&xml), []);
}

#[test]
fn findings_have_readable_messages() {
    let xml = webshop_with(
        "<accID>8000</accID>\n            <docRef>5001",
        "<accID>8999</accID>\n            <docRef>5001",
    );
    assert_eq!(
        validate(&xml)[0].to_string(),
        "trLine/accID (journal VERK, transaction 1001, line 2): unknown ledger account `8999`"
    );
}

#[test]
fn file_without_lists_reports_every_reference() {
    let xml = r#"<auditfile xmlns="http://www.odb.belastingdienst.nl/Belastingdienst/BCPP/1.1/structures/XmlauditfileXAF_4.0">
      <header><fiscalYear>2024</fiscalYear><startDate>2024-01-01</startDate><endDate>2024-12-31</endDate>
        <curCode>EUR</curCode><dateCreated>2025-01-01</dateCreated>
        <softwareDesc>x</softwareDesc><softwareVersion>1</softwareVersion></header>
      <company><companyName>x</companyName><taxRegistrationCountry>NL</taxRegistrationCountry>
        <taxRegIdent>x</taxRegIdent>
        <transactions><linesCount>1</linesCount><totalDebit>1.00</totalDebit><totalCredit>0.00</totalCredit>
          <journal><jrnID>J</jrnID><desc>j</desc>
            <transaction><nr>1</nr><periodNumber>1</periodNumber><trDt>2024-01-01</trDt>
              <trLine><nr>1</nr><accID>1000</accID><docRef>d</docRef><effDate>2024-01-01</effDate>
                <amnt>1.00</amnt><amntTp>D</amntTp></trLine>
            </transaction>
          </journal>
        </transactions></company></auditfile>"#;
    let kinds: Vec<_> = validate(xml).into_iter().map(|f| f.kind).collect();
    assert!(kinds.contains(&FindingKind::UnknownPeriod(1)));
    assert!(kinds.contains(&FindingKind::UnknownAccount(AccountId::from("1000"))));
    assert!(
        kinds
            .iter()
            .any(|k| matches!(k, FindingKind::UnbalancedTransaction { .. }))
    );
}
