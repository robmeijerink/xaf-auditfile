use std::fs::File;
use std::io::BufReader;

use xaf_auditfile::{Auditfile, Error, FiscalYear, Side};

const WEBSHOP: &str = "tests/fixtures/webshop.xaf";

#[test]
fn reads_a_file_from_a_path() {
    let file = Auditfile::from_path(WEBSHOP).unwrap();
    assert_eq!(file.header.fiscal_year, FiscalYear::Single(2024));
    assert_eq!(file.company.name, "Voorbeeld Webshop B.V.");
    assert_eq!(file.relations.len(), 2);
    assert_eq!(file.accounts.len(), 5);
    assert_eq!(file.vat_codes.len(), 2);
    assert_eq!(file.journals.len(), 2);
}

#[test]
fn path_and_reader_give_the_same_result() {
    let from_path = Auditfile::from_path(WEBSHOP).unwrap();
    let from_reader = Auditfile::from_reader(BufReader::new(File::open(WEBSHOP).unwrap())).unwrap();
    assert_eq!(from_path, from_reader);
}

#[test]
fn transactions_iterates_over_all_journals_in_order() {
    let file = Auditfile::from_path(WEBSHOP).unwrap();
    let seen: Vec<_> = file
        .transactions()
        .map(|(journal, transaction)| (journal.id.as_str(), transaction.nr.as_str()))
        .collect();
    assert_eq!(seen, [("VERK", "1001"), ("VERK", "1002"), ("BANK", "2001")]);
}

#[test]
fn vat_entries_are_available_per_line() {
    let file = Auditfile::from_path(WEBSHOP).unwrap();
    let vat: Vec<_> = file
        .transactions()
        .flat_map(|(_, transaction)| &transaction.lines)
        .flat_map(|line| &line.vat)
        .map(|vat| (vat.vat_code.as_str(), vat.amount.side))
        .collect();
    assert_eq!(
        vat,
        [
            ("HOOG", Side::Credit),
            ("HOOG", Side::Credit),
            ("LAAG", Side::Credit)
        ]
    );
}

#[test]
fn missing_file_is_an_io_error() {
    let result = Auditfile::from_path("tests/fixtures/does-not-exist.xaf");
    assert!(matches!(result, Err(Error::Io(_))));
}
