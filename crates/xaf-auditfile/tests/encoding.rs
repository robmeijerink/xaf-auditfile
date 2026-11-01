#![allow(clippy::unwrap_used)]

use xaf_auditfile::{Auditfile, Error};

const WEBSHOP: &str = include_str!("fixtures/webshop.xaf");

fn with_accents(declared: &str) -> String {
    WEBSHOP
        .replace("encoding=\"UTF-8\"", &format!("encoding=\"{declared}\""))
        .replace("Voorbeeld Webshop B.V.", "Café Zoë B.V.")
        .replace("Webshoporder 5001", "Bestelling café €")
}

fn latin1(text: &str) -> Vec<u8> {
    text.replace('€', "EUR")
        .chars()
        .map(|c| u8::try_from(u32::from(c)).unwrap())
        .collect()
}

fn windows_1252(text: &str) -> Vec<u8> {
    text.chars()
        .map(|c| match c {
            '€' => 0x80,
            c => u8::try_from(u32::from(c)).unwrap(),
        })
        .collect()
}

fn utf16(text: &str, bom: &[u8], to_bytes: fn(u16) -> [u8; 2]) -> Vec<u8> {
    let mut bytes = bom.to_vec();
    bytes.extend(text.encode_utf16().flat_map(to_bytes));
    bytes
}

fn whole(bytes: &[u8]) -> Result<Auditfile, Error> {
    Auditfile::from_reader(bytes)
}

fn streamed(bytes: &[u8]) -> Result<(Auditfile, Vec<xaf_auditfile::StreamedTransaction>), Error> {
    let stream = Auditfile::stream_from_reader(bytes)?;
    let master = stream.auditfile().clone();
    let transactions = stream.collect::<Result<Vec<_>, _>>()?;
    Ok((master, transactions))
}

fn first_description(file: &Auditfile) -> Option<&str> {
    file.transactions().next()?.1.description.as_deref()
}

#[test]
fn iso_8859_1_is_decoded() {
    let file = whole(&latin1(&with_accents("ISO-8859-1"))).unwrap();
    assert_eq!(file.company.name, "Café Zoë B.V.");
    assert_eq!(first_description(&file), Some("Bestelling café EUR"));
}

#[test]
fn windows_1252_is_decoded_including_the_euro_sign() {
    let file = whole(&windows_1252(&with_accents("windows-1252"))).unwrap();
    assert_eq!(file.company.name, "Café Zoë B.V.");
    assert_eq!(first_description(&file), Some("Bestelling café €"));
}

#[test]
fn streaming_decodes_the_same_way() {
    for (declared, bytes) in [
        ("ISO-8859-1", latin1(&with_accents("ISO-8859-1"))),
        ("windows-1252", windows_1252(&with_accents("windows-1252"))),
    ] {
        let expected = whole(&bytes).unwrap();
        let (master, transactions) = streamed(&bytes).unwrap();

        assert_eq!(master.company.name, "Café Zoë B.V.", "{declared}");
        let streamed: Vec<_> = transactions.into_iter().map(|t| t.transaction).collect();
        let whole: Vec<_> = expected.transactions().map(|(_, t)| t.clone()).collect();
        assert_eq!(streamed, whole, "{declared}");
        assert_eq!(
            streamed[0].description.as_deref(),
            first_description(&expected),
            "{declared}"
        );
    }
}

#[test]
fn utf_8_with_a_byte_order_mark_is_accepted() {
    let mut bytes = b"\xEF\xBB\xBF".to_vec();
    bytes.extend_from_slice(with_accents("UTF-8").as_bytes());
    assert_eq!(whole(&bytes).unwrap().company.name, "Café Zoë B.V.");
    assert_eq!(streamed(&bytes).unwrap().0.company.name, "Café Zoë B.V.");
}

#[test]
fn bytes_that_do_not_match_the_declared_encoding_are_an_encoding_error() {
    let bytes = latin1(&with_accents("UTF-8"));
    assert!(matches!(whole(&bytes), Err(Error::Encoding(_))));
    assert!(matches!(streamed(&bytes), Err(Error::Encoding(_))));
}

#[test]
fn utf_16_is_an_encoding_error() {
    let text = with_accents("UTF-16");
    let files = [
        utf16(&text, &[0xFF, 0xFE], u16::to_le_bytes),
        utf16(&text, &[0xFE, 0xFF], u16::to_be_bytes),
        utf16(&text, &[], u16::to_le_bytes),
        utf16(&text, &[], u16::to_be_bytes),
    ];
    for bytes in files {
        assert!(matches!(whole(&bytes), Err(Error::Encoding(_))), "whole");
        assert!(
            matches!(streamed(&bytes), Err(Error::Encoding(_))),
            "stream"
        );
    }
}

#[test]
fn the_utf_16_error_says_what_to_do() {
    let bytes = utf16(&with_accents("UTF-16"), &[0xFF, 0xFE], u16::to_le_bytes);
    let message = whole(&bytes).unwrap_err().to_string();
    assert!(message.contains("convert the file to UTF-8"), "{message}");
}
