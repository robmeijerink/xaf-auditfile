#![allow(clippy::unwrap_used)]

use xaf_auditfile::{Auditfile, Error};

const NAMESPACE: &str =
    "http://www.odb.belastingdienst.nl/Belastingdienst/BCPP/1.1/structures/XmlauditfileXAF_4.0";
const HEADER: &str = "<header><fiscalYear>2024</fiscalYear><startDate>2024-01-01</startDate>\
    <endDate>2024-12-31</endDate><curCode>EUR</curCode><dateCreated>2025-01-01</dateCreated>\
    <softwareDesc>x</softwareDesc><softwareVersion>1</softwareVersion>";

fn document(doctype: &str, header_extra: &str, company_name: &str, transactions: &str) -> String {
    format!(
        "{doctype}<auditfile xmlns=\"{NAMESPACE}\">{HEADER}{header_extra}</header>\
         <company><companyName>{company_name}</companyName>\
         <taxRegistrationCountry>NL</taxRegistrationCountry><taxRegIdent>x</taxRegIdent>\
         {transactions}</company></auditfile>"
    )
}

fn transactions(lines: &str) -> String {
    format!(
        "<transactions><linesCount>2</linesCount><totalDebit>1.00</totalDebit>\
         <totalCredit>1.00</totalCredit><journal><jrnID>J</jrnID><desc>j</desc>\
         <transaction><nr>1</nr><periodNumber>1</periodNumber><trDt>2024-01-01</trDt>\
         {lines}</transaction></journal></transactions>"
    )
}

fn line(nr: u32, amount: &str, side: &str) -> String {
    format!(
        "<trLine><nr>{nr}</nr><accID>1</accID><docRef>d</docRef><effDate>2024-01-01</effDate>\
         <amnt>{amount}</amnt><amntTp>{side}</amntTp></trLine>"
    )
}

fn whole(xml: &str) -> Result<Auditfile, Error> {
    Auditfile::from_reader(xml.as_bytes())
}

/// Reads everything through the streaming interface and returns the first error, if any.
fn streamed(xml: &str) -> Result<(), Error> {
    let stream = Auditfile::stream_from_reader(xml.as_bytes())?;
    for item in stream {
        item?;
    }
    Ok(())
}

fn nested(depth: usize) -> String {
    format!("{}{}", "<a>".repeat(depth), "</a>".repeat(depth))
}

#[test]
fn nesting_beyond_the_limit_is_rejected_in_the_header() {
    let xml = document("", &nested(100_000), "x", "");
    assert!(matches!(whole(&xml), Err(Error::NestingTooDeep(_))));
    assert!(matches!(streamed(&xml), Err(Error::NestingTooDeep(_))));
}

#[test]
fn nesting_beyond_the_limit_is_rejected_in_a_transaction() {
    let lines = format!("{}{}", line(1, "1.00", "D"), nested(100_000));
    let xml = document("", "", "x", &transactions(&lines));
    assert!(matches!(whole(&xml), Err(Error::NestingTooDeep(_))));
    assert!(matches!(streamed(&xml), Err(Error::NestingTooDeep(_))));
}

#[test]
fn nesting_beyond_the_limit_is_rejected_in_an_unknown_journal_child() {
    let xml = document("", "", "x", &transactions("")).replacen(
        "<transaction>",
        &format!("{}<transaction>", nested(100_000)),
        1,
    );
    assert!(matches!(whole(&xml), Err(Error::NestingTooDeep(_))));
    assert!(matches!(streamed(&xml), Err(Error::NestingTooDeep(_))));
}

#[test]
fn nesting_just_under_the_limit_is_fine() {
    let xml = document("", &nested(100), "x", &transactions(&nested(100)));
    assert!(whole(&xml).is_ok());
    assert!(streamed(&xml).is_ok());
}

#[test]
fn the_nesting_error_names_the_limit() {
    let xml = document("", &nested(1_000), "x", "");
    let message = whole(&xml).unwrap_err().to_string();
    assert_eq!(message, "elements are nested more than 128 levels deep");
}

#[test]
fn entity_expansion_is_rejected() {
    let mut doctype = String::from("<!DOCTYPE auditfile [<!ENTITY a0 \"aaaaaaaaaa\">");
    for level in 1..10 {
        doctype.push_str(&format!(
            "<!ENTITY a{level} \"{}\">",
            format!("&a{};", level - 1).repeat(10)
        ));
    }
    doctype.push_str("]>");
    let xml = document(&doctype, "", "&a9;", "");
    assert!(matches!(whole(&xml), Err(Error::Xml(_))));
    assert!(matches!(streamed(&xml), Err(Error::Xml(_))));
}

#[test]
fn external_entities_are_not_fetched() {
    let doctype = "<!DOCTYPE auditfile [<!ENTITY e SYSTEM \"file:///etc/passwd\">]>";
    let xml = document(doctype, "", "&e;", "");
    assert!(matches!(whole(&xml), Err(Error::Xml(_))));
    assert!(matches!(streamed(&xml), Err(Error::Xml(_))));
}

#[test]
fn amounts_at_the_limit_of_decimal_are_rejected_and_do_not_panic() {
    let max = "79228162514264337593543950335";
    let lines = format!(
        "{}{}{}",
        line(1, max, "D"),
        line(2, max, "D"),
        line(3, max, "C")
    );
    let xml = document("", "", "x", &transactions(&lines));
    assert!(matches!(whole(&xml), Err(Error::InvalidValue { .. })));
    assert!(matches!(streamed(&xml), Err(Error::InvalidValue { .. })));
}

#[test]
fn many_large_amounts_can_be_summed_and_validated() {
    let largest = "99999999999999999999.99";
    let lines: String = (1..=1000)
        .map(|nr| line(nr, largest, if nr % 2 == 0 { "C" } else { "D" }))
        .collect();
    let xml = document("", "", "x", &transactions(&lines));
    let file = whole(&xml).unwrap();
    assert!(!file.validate().is_empty());
}

#[test]
fn an_enormous_number_is_rejected_and_the_message_stays_short() {
    let xml = document(
        "",
        "",
        "x",
        &transactions(&line(1, &"9".repeat(1_000_000), "D")),
    );
    for error in [whole(&xml).unwrap_err(), streamed(&xml).unwrap_err()] {
        assert!(matches!(error, Error::InvalidValue { .. }));
        assert!(error.to_string().len() < 300, "{}", error.to_string().len());
    }
}

#[test]
fn a_very_long_text_value_is_read_without_trouble() {
    let name = "A".repeat(5_000_000);
    let xml = document("", "", &name, "");
    assert_eq!(whole(&xml).unwrap().company.name.len(), name.len());
    assert!(streamed(&xml).is_ok());
}

#[test]
fn a_huge_number_of_unknown_elements_is_skipped() {
    let extra = "<x/>".repeat(500_000);
    let xml = document("", &extra, "x", "");
    assert!(whole(&xml).is_ok());
    assert!(streamed(&xml).is_ok());
}

#[test]
fn garbage_bytes_are_an_error_and_not_a_panic() {
    let garbage: Vec<u8> = (0..4096_u32)
        .map(|n| (n.wrapping_mul(2_654_435_761) >> 13) as u8)
        .collect();
    assert!(Auditfile::from_reader(garbage.as_slice()).is_err());
    assert!(Auditfile::stream_from_reader(garbage.as_slice()).is_err());
}
