#![allow(clippy::unwrap_used)]

use xaf_auditfile::{Auditfile, Error, Finding, StreamedTransaction};

const WEBSHOP: &str = include_str!("fixtures/webshop.xaf");
const NAMESPACE: &str =
    "http://www.odb.belastingdienst.nl/Belastingdienst/BCPP/1.1/structures/XmlauditfileXAF_4.0";

fn webshop_with(from: &str, to: &str) -> String {
    assert!(WEBSHOP.contains(from), "fixture does not contain {from}");
    WEBSHOP.replacen(from, to, 1)
}

fn stream(xml: &str) -> Result<xaf_auditfile::AuditfileStream<&[u8]>, Error> {
    Auditfile::stream_from_reader(xml.as_bytes())
}

fn collect(xml: &str) -> (Auditfile, Vec<Result<StreamedTransaction, Error>>) {
    let stream = stream(xml).unwrap();
    let master = stream.auditfile().clone();
    (master, stream.collect())
}

/// Findings from streaming: the master data first, then each transaction, then the totals.
fn streamed_findings(xml: &str) -> Vec<Finding> {
    let stream = stream(xml).unwrap();
    let mut findings = stream.auditfile().validate_master_data();
    let mut validator = stream.auditfile().validator();
    for item in stream {
        let item = item.unwrap();
        findings.extend(validator.check(&item.journal, &item.transaction));
    }
    findings.extend(validator.finish());
    findings
}

#[test]
fn master_data_is_the_file_without_journals() {
    let full = Auditfile::from_reader(WEBSHOP.as_bytes()).unwrap();
    let (master, _) = collect(WEBSHOP);

    let mut expected = full;
    expected.journals.clear();
    assert_eq!(master, expected);
    assert!(master.transaction_totals.is_some());
}

#[test]
fn transactions_are_the_same_as_in_a_whole_file_read() {
    let full = Auditfile::from_reader(WEBSHOP.as_bytes()).unwrap();
    let (_, items) = collect(WEBSHOP);

    let streamed: Vec<_> = items
        .into_iter()
        .map(|item| {
            let item = item.unwrap();
            (item.journal.id.clone(), item.transaction)
        })
        .collect();
    let whole: Vec<_> = full
        .transactions()
        .map(|(journal, transaction)| (journal.id.clone(), transaction.clone()))
        .collect();
    assert_eq!(streamed, whole);
}

#[test]
fn the_journal_is_handed_out_without_its_transactions() {
    let full = Auditfile::from_reader(WEBSHOP.as_bytes()).unwrap();
    let (_, items) = collect(WEBSHOP);
    let first = items.into_iter().next().unwrap().unwrap();

    let mut expected = full.journals[0].clone();
    expected.transactions.clear();
    assert_eq!(*first.journal, expected);
}

#[test]
fn transactions_of_one_journal_share_the_journal() {
    let (_, items) = collect(WEBSHOP);
    let items: Vec<_> = items.into_iter().map(Result::unwrap).collect();
    assert!(std::sync::Arc::ptr_eq(&items[0].journal, &items[1].journal));
    assert!(!std::sync::Arc::ptr_eq(
        &items[1].journal,
        &items[2].journal
    ));
}

#[test]
fn reads_from_a_path() {
    let stream = Auditfile::stream_from_path("tests/fixtures/webshop.xaf").unwrap();
    assert_eq!(stream.count(), 3);
}

#[test]
fn missing_file_is_an_io_error() {
    let result = Auditfile::stream_from_path("tests/fixtures/does-not-exist.xaf");
    assert!(matches!(result, Err(Error::Io(_))));
}

#[test]
fn the_whole_file_reader_is_unaffected_by_streaming() {
    let reordered = webshop_with("<header>", "<header><addedLater>1</addedLater>");
    let file = Auditfile::from_reader(reordered.as_bytes()).unwrap();
    assert_eq!(file.journals.len(), 2);
}

#[test]
fn another_version_is_rejected_up_front() {
    let result = Auditfile::stream_from_path("tests/fixtures/xaf32.xaf");
    match result {
        Err(Error::UnsupportedVersion(namespace)) => {
            assert_eq!(namespace, "http://www.auditfiles.nl/XAF/3.2");
        }
        other => panic!("unexpected result: {:?}", other.map(|_| ())),
    }
}

#[test]
fn input_that_is_not_an_auditfile_is_rejected_up_front() {
    for xml in ["", "hello world", "<invoice/>"] {
        assert!(
            matches!(stream(xml), Err(Error::NotAnAuditfile(_))),
            "{xml:?}"
        );
    }
}

#[test]
fn an_unusable_header_fails_up_front() {
    let xml = webshop_with(
        "<startDate>2024-01-01</startDate>",
        "<startDate>x</startDate>",
    );
    assert!(matches!(stream(&xml), Err(Error::InvalidValue { .. })));
}

#[test]
fn file_without_transactions_yields_nothing() {
    let xml = format!(
        "<auditfile xmlns=\"{NAMESPACE}\">\
           <header><fiscalYear>2024</fiscalYear><startDate>2024-01-01</startDate>\
             <endDate>2024-12-31</endDate><curCode>EUR</curCode>\
             <dateCreated>2025-01-01</dateCreated><softwareDesc>x</softwareDesc>\
             <softwareVersion>1</softwareVersion></header>\
           <company><companyName>x</companyName>\
             <taxRegistrationCountry>NL</taxRegistrationCountry><taxRegIdent>x</taxRegIdent>\
           </company></auditfile>"
    );
    let (master, items) = collect(&xml);
    assert_eq!(master.company.name, "x");
    assert!(items.is_empty());
}

#[test]
fn transactions_element_without_journals_yields_nothing() {
    let xml = WEBSHOP.split("<journal>").next().unwrap().to_owned()
        + "</transactions></company></auditfile>";
    let (master, items) = collect(&xml);
    assert_eq!(master.transaction_totals.unwrap().lines_count, 8);
    assert!(items.is_empty());
}

#[test]
fn journals_without_transactions_are_not_yielded() {
    let xml = webshop_with(
        "</transactions>",
        "<journal><jrnID>LEEG</jrnID><desc>Leeg</desc></journal></transactions>",
    );
    let (_, items) = collect(&xml);
    assert_eq!(items.len(), 3);
}

#[test]
fn a_file_cut_off_in_the_transactions_yields_the_complete_ones_then_an_error() {
    let cut = WEBSHOP.find("<nr>2001</nr>").unwrap();
    let (_, items) = collect(&WEBSHOP[..cut]);
    assert!(matches!(
        items.as_slice(),
        [Ok(_), Ok(_), Err(Error::Xml(_))]
    ));
}

#[test]
fn an_invalid_transaction_ends_the_stream_after_the_error() {
    let xml = webshop_with("<trDt>2024-01-20</trDt>", "<trDt>20-01-2024</trDt>");
    let mut stream = stream(&xml).unwrap();
    assert!(stream.next().unwrap().is_ok());
    match stream.next() {
        Some(Err(Error::InvalidValue { location, .. })) => {
            assert_eq!(
                location.to_string(),
                "transaction/trDt (journal VERK, transaction 1002)"
            );
        }
        other => panic!("unexpected item: {:?}", other.map(|r| r.map(|_| ()))),
    }
    assert!(stream.next().is_none());
}

#[test]
fn a_missing_element_in_a_transaction_is_a_deserialize_error() {
    let xml = webshop_with("<trDt>2024-02-05</trDt>", "");
    let (_, items) = collect(&xml);
    assert!(matches!(
        items.as_slice(),
        [Ok(_), Ok(_), Err(Error::Deserialize(_))]
    ));
}

#[test]
fn master_data_after_the_transactions_is_out_of_order() {
    let vat_codes_start = WEBSHOP.find("<vatCodes>").unwrap();
    let vat_codes_end = WEBSHOP.find("</vatCodes>").unwrap() + "</vatCodes>".len();
    let block = &WEBSHOP[vat_codes_start..vat_codes_end];
    let xml =
        WEBSHOP
            .replacen(block, "", 1)
            .replacen("</company>", &format!("{block}</company>"), 1);

    let (_, items) = collect(&xml);
    assert_eq!(items.len(), 4);
    assert!(items[..3].iter().all(Result::is_ok));
    match &items[3] {
        Err(Error::OutOfOrder(element)) => assert_eq!(element, "vatCodes"),
        other => panic!("unexpected item: {:?}", other.as_ref().map(|_| ())),
    }
}

#[test]
fn elements_unknown_to_the_schema_are_ignored() {
    let xml = webshop_with("<header>", "<header><addedLater>1</addedLater>").replacen(
        "<journal>",
        "<journal><note>x</note>",
        1,
    );
    let (_, items) = collect(&xml);
    assert_eq!(items.len(), 3);
    assert!(items.iter().all(Result::is_ok));
}

#[test]
fn prefixed_elements_are_supported() {
    let xml = WEBSHOP
        .replace("<auditfile xmlns=", "<x:auditfile xmlns:x=")
        .replace("</auditfile>", "</x:auditfile>")
        .replace("<header>", "<x:header>")
        .replace("</header>", "</x:header>");
    let (master, items) = collect(&xml);
    assert_eq!(master.header.software.name, "Example Books");
    assert_eq!(items.len(), 3);
}

#[test]
fn a_clean_file_has_no_findings_when_streamed() {
    assert_eq!(streamed_findings(WEBSHOP), []);
}

#[test]
fn streamed_findings_equal_those_of_a_whole_file_read() {
    let mutations = [
        (
            "<amnt>100.00</amnt>\n            <amntTp>C</amntTp>",
            "<amnt>99.00</amnt>\n            <amntTp>C</amntTp>",
        ),
        (
            "<accID>8000</accID>\n            <docRef>5001",
            "<accID>8999</accID>\n            <docRef>5001",
        ),
        (
            "<vatID>LAAG</vatID>\n              <vatPerc>",
            "<vatID>ONBEKEND</vatID>\n              <vatPerc>",
        ),
        (
            "<custSupID>C001</custSupID>\n            <invRef>",
            "<custSupID>C999</custSupID>\n            <invRef>",
        ),
        (
            "<periodNumber>2</periodNumber>\n          <trDt>2024-02-05",
            "<periodNumber>9</periodNumber>\n          <trDt>2024-02-05",
        ),
        (
            "<nr>1002</nr>\n          <periodNumber>",
            "<nr>1001</nr>\n          <periodNumber>",
        ),
        ("<linesCount>8</linesCount>", "<linesCount>9</linesCount>"),
        (
            "<offsetAccID>1100</offsetAccID>",
            "<offsetAccID>9999</offsetAccID>",
        ),
        (
            "<vatToPayAccID>1800</vatToPayAccID>",
            "<vatToPayAccID>1810</vatToPayAccID>",
        ),
        (
            "<amnt>1000.00</amnt>\n        <amntTp>C</amntTp>",
            "<amnt>900.00</amnt>\n        <amntTp>C</amntTp>",
        ),
        (
            "<accID>1100</accID>\n        <accDesc>Bank",
            "<accID>1300</accID>\n        <accDesc>Bank",
        ),
    ];
    for (from, to) in mutations {
        let xml = webshop_with(from, to);
        let whole = Auditfile::from_reader(xml.as_bytes()).unwrap().validate();
        assert!(!whole.is_empty(), "mutation produced no finding: {from}");
        assert_eq!(streamed_findings(&xml), whole, "mutation: {from}");
    }
}
