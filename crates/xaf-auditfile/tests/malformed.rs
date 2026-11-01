use xaf_auditfile::{Auditfile, Error, JournalKind};

const WEBSHOP: &str = include_str!("fixtures/webshop.xaf");
const NAMESPACE: &str =
    "http://www.odb.belastingdienst.nl/Belastingdienst/BCPP/1.1/structures/XmlauditfileXAF_4.0";

fn read(xml: &str) -> Result<Auditfile, Error> {
    Auditfile::from_reader(xml.as_bytes())
}

fn webshop_with(from: &str, to: &str) -> String {
    assert!(WEBSHOP.contains(from), "fixture does not contain {from}");
    WEBSHOP.replacen(from, to, 1)
}

fn deserialize_message(result: Result<Auditfile, Error>) -> String {
    match result {
        Err(Error::Deserialize(error)) => error.to_string(),
        other => format!("not a deserialize error: {other:?}"),
    }
}

#[test]
fn empty_input_is_not_an_auditfile() {
    assert!(matches!(read(""), Err(Error::NotAnAuditfile(_))));
}

#[test]
fn plain_text_is_not_an_auditfile() {
    assert!(matches!(read("hello world"), Err(Error::NotAnAuditfile(_))));
}

#[test]
fn file_cut_off_halfway_is_malformed_xml() {
    let truncated = &WEBSHOP[..WEBSHOP.len() / 2];
    assert!(matches!(read(truncated), Err(Error::Xml(_))));
}

#[test]
fn missing_closing_root_tag_is_malformed_xml() {
    let truncated = WEBSHOP.trim_end().trim_end_matches("</auditfile>");
    assert!(matches!(read(truncated), Err(Error::Xml(_))));
}

#[test]
fn mismatched_tags_are_malformed_xml() {
    let xml = webshop_with("</header>", "</heder>");
    assert!(matches!(read(&xml), Err(Error::Xml(_))));
}

#[test]
fn unknown_entity_is_malformed_xml() {
    let xml = webshop_with("Voorbeeld Webshop B.V.", "A &bogus; B");
    assert!(matches!(read(&xml), Err(Error::Xml(_))));
}

#[test]
fn missing_header_names_the_element() {
    let xml = webshop_with("<header>", "<other>").replacen("</header>", "</other>", 1);
    assert_eq!(deserialize_message(read(&xml)), "missing field `header`");
}

#[test]
fn missing_required_element_in_a_transaction_names_the_element() {
    let xml = webshop_with("<trDt>2024-01-10</trDt>", "");
    assert_eq!(deserialize_message(read(&xml)), "missing field `trDt`");
}

#[test]
fn general_ledger_without_accounts_is_rejected() {
    let xml = webshop_with(
        "<generalLedger>",
        "<generalLedger></generalLedger><ignored>",
    )
    .replacen("</generalLedger>\n", "</ignored>\n", 1);
    assert_eq!(
        deserialize_message(read(&xml)),
        "missing field `ledgerAccount`"
    );
}

#[test]
fn duplicate_header_is_rejected() {
    let xml = webshop_with(
        "<company>",
        "<header><fiscalYear>2024</fiscalYear></header><company>",
    );
    assert_eq!(deserialize_message(read(&xml)), "duplicate field `header`");
}

#[test]
fn invalid_date_names_the_element() {
    let xml = webshop_with(
        "<startDate>2024-01-01</startDate>",
        "<startDate>01-01-2024</startDate>",
    );
    match read(&xml) {
        Err(Error::InvalidValue {
            location, value, ..
        }) => {
            assert_eq!(location.to_string(), "header/startDate");
            assert_eq!(value, "01-01-2024");
        }
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn empty_required_amount_names_the_line() {
    let xml = webshop_with("<amnt>121.00</amnt>", "<amnt/>");
    let error = read(&xml).unwrap_err();
    assert_eq!(
        error.to_string(),
        "invalid value \"\" in trLine/amnt (journal VERK, transaction 1001, line 1): \
         expected a decimal number below 10^20"
    );
}

#[test]
fn invalid_debit_credit_indicator_is_reported() {
    let xml = webshop_with("<amntTp>C</amntTp>", "<amntTp>K</amntTp>");
    let message = read(&xml).unwrap_err().to_string();
    assert!(message.contains("expected `D` or `C`"), "{message}");
}

#[test]
fn unknown_code_does_not_fail_the_parse() {
    let xml = webshop_with("<jrnTp>S</jrnTp>", "<jrnTp>Q</jrnTp>");
    let file = read(&xml).unwrap();
    assert_eq!(
        file.journals[0].kind,
        Some(JournalKind::Unknown("Q".into()))
    );
}

#[test]
fn unknown_elements_are_ignored() {
    let xml = webshop_with("<header>", "<header><addedLater>1</addedLater>");
    assert_eq!(read(&xml).unwrap(), read(WEBSHOP).unwrap());
}

#[test]
fn element_order_within_a_record_does_not_matter() {
    let xml = webshop_with("<fiscalYear>2024</fiscalYear>", "").replacen(
        "</header>",
        "<fiscalYear>2024</fiscalYear></header>",
        1,
    );
    assert_eq!(read(&xml).unwrap(), read(WEBSHOP).unwrap());
}

#[test]
fn cdata_and_entities_are_decoded() {
    let xml = webshop_with(
        "<companyName>Voorbeeld Webshop B.V.</companyName>",
        "<companyName><![CDATA[A & B]]></companyName>",
    );
    assert_eq!(read(&xml).unwrap().company.name, "A & B");
    let xml = webshop_with("Voorbeeld Webshop B.V.", "A &amp; B");
    assert_eq!(read(&xml).unwrap().company.name, "A & B");
}

#[test]
fn a_prefixed_namespace_is_supported() {
    let xml = format!(
        "<x:auditfile xmlns:x=\"{NAMESPACE}\">\
           <x:header>\
             <x:fiscalYear>2024-2025</x:fiscalYear>\
             <x:startDate>2024-07-01</x:startDate>\
             <x:endDate>2025-06-30</x:endDate>\
             <x:curCode>EUR</x:curCode>\
             <x:dateCreated>2025-07-15</x:dateCreated>\
             <x:softwareDesc>Example</x:softwareDesc>\
             <x:softwareVersion>1</x:softwareVersion>\
           </x:header>\
           <x:company>\
             <x:companyName>Prefixed B.V.</x:companyName>\
             <x:taxRegistrationCountry>NL</x:taxRegistrationCountry>\
             <x:taxRegIdent>NL000000000B01</x:taxRegIdent>\
           </x:company>\
         </x:auditfile>"
    );
    let file = read(&xml).unwrap();
    assert_eq!(file.company.name, "Prefixed B.V.");
    assert!(file.journals.is_empty());
}

fn streamed_error(xml: &str) -> Option<Error> {
    let stream = match Auditfile::stream_from_reader(xml.as_bytes()) {
        Ok(stream) => stream,
        Err(error) => return Some(error),
    };
    stream.filter_map(Result::err).next()
}

fn both_fail_with(xml: &str, expected: fn(&Error) -> bool) {
    let whole = read(xml).err();
    assert!(whole.as_ref().is_some_and(expected), "whole: {whole:?}");
    let streamed = streamed_error(xml);
    assert!(
        streamed.as_ref().is_some_and(expected),
        "stream: {streamed:?}"
    );
}

#[test]
fn text_before_the_root_element_is_rejected() {
    let xml = format!("junk{WEBSHOP}");
    both_fail_with(&xml, |e| matches!(e, Error::NotAnAuditfile(_)));
}

#[test]
fn content_after_the_root_element_is_rejected() {
    for extra in ["junk", "<extra/>", "<other>x</other>", "<![CDATA[x]]>"] {
        let xml = format!("{WEBSHOP}{extra}");
        both_fail_with(&xml, |e| matches!(e, Error::NotAnAuditfile(_)));
    }
}

#[test]
fn whitespace_and_comments_around_the_root_element_are_fine() {
    let xml = format!("\n<!-- before -->\n{WEBSHOP}\n<!-- after -->\n");
    assert!(read(&xml).is_ok());
    assert!(streamed_error(&xml).is_none());
}

#[test]
fn a_file_missing_its_closing_tags_is_rejected() {
    let cut = WEBSHOP.find("</company>").unwrap();
    let xml = &WEBSHOP[..cut];
    both_fail_with(xml, |e| matches!(e, Error::Xml(_)));
}

#[test]
fn text_mixed_with_elements_is_rejected() {
    for (from, to) in [
        ("</header>", "junk</header>"),
        ("<journal>", "junk<journal>"),
        ("<trLine>", "junk<trLine>"),
        ("</transactions>", "junk</transactions>"),
        ("</company>", "junk</company>"),
    ] {
        let xml = webshop_with(from, to);
        both_fail_with(&xml, |e| matches!(e, Error::Deserialize(_)));
    }
}

#[test]
fn a_damaged_xml_declaration_is_rejected() {
    let xml = webshop_with("version=\"1.0\"", "version=\"9.9\"");
    both_fail_with(&xml, |e| matches!(e, Error::Xml(_)));
}
