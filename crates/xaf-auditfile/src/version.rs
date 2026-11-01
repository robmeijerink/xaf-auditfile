use quick_xml::NsReader;
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::ResolveResult;

use crate::{Error, document};

pub(crate) const NAMESPACE_V4: &str =
    "http://www.odb.belastingdienst.nl/Belastingdienst/BCPP/1.1/structures/XmlauditfileXAF_4.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Version {
    V4,
}

/// Rejects the encodings the reader cannot parse: UTF-16 and UTF-32. They are recognised by a byte
/// order mark, or by a null byte among the first four bytes, which no ASCII-compatible encoding
/// has there. Other encodings are handled while reading.
pub(crate) fn check_encoding(head: &[u8]) -> Result<(), Error> {
    let byte_order_mark = head.starts_with(&[0xFF, 0xFE]) || head.starts_with(&[0xFE, 0xFF]);
    let null_byte = head.iter().take(4).any(|&byte| byte == 0);
    if byte_order_mark || null_byte {
        return Err(Error::Encoding(
            "UTF-16 and UTF-32 are not supported, convert the file to UTF-8".into(),
        ));
    }
    Ok(())
}

/// Determines the XAF version from the namespace of the root element, without reading further.
pub(crate) fn detect(xml: &[u8]) -> Result<Version, Error> {
    check_encoding(xml)?;
    let mut reader = NsReader::from_reader(xml);
    loop {
        let (namespace, event) = reader.read_resolved_event()?;
        match event {
            Event::Start(root) | Event::Empty(root) => return classify(&namespace, &root),
            Event::Text(text) if !document::is_blank(&text) => return Err(document::outside_root()),
            Event::Eof => {
                return Err(Error::NotAnAuditfile("the file has no root element".into()));
            }
            _ => {}
        }
    }
}

/// Decides from the root element and its namespace whether this is a supported auditfile.
pub(crate) fn classify(
    namespace: &ResolveResult<'_>,
    root: &BytesStart<'_>,
) -> Result<Version, Error> {
    let name = root.local_name();
    if name.as_ref() != b"auditfile" {
        return Err(Error::NotAnAuditfile(format!(
            "the root element is `{}`, expected `auditfile`",
            String::from_utf8_lossy(name.as_ref())
        )));
    }

    match namespace {
        ResolveResult::Bound(ns) if ns.as_ref() == NAMESPACE_V4.as_bytes() => Ok(Version::V4),
        ResolveResult::Bound(ns) => Err(Error::UnsupportedVersion(
            String::from_utf8_lossy(ns.as_ref()).into_owned(),
        )),
        ResolveResult::Unbound => Err(Error::NotAnAuditfile(
            "the root element has no namespace".into(),
        )),
        ResolveResult::Unknown(prefix) => Err(Error::NotAnAuditfile(format!(
            "the root element uses the undeclared prefix `{}`",
            String::from_utf8_lossy(prefix)
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(attributes: &str) -> String {
        format!("<auditfile {attributes}><header/></auditfile>")
    }

    fn v4() -> String {
        root(&format!("xmlns=\"{NAMESPACE_V4}\""))
    }

    #[test]
    fn detects_version_4_with_default_namespace() {
        assert_eq!(detect(v4().as_bytes()).unwrap(), Version::V4);
    }

    #[test]
    fn detects_version_4_with_prefixed_namespace() {
        let xml = format!("<x:auditfile xmlns:x=\"{NAMESPACE_V4}\"/>");
        assert_eq!(detect(xml.as_bytes()).unwrap(), Version::V4);
    }

    #[test]
    fn skips_declaration_comments_and_whitespace() {
        let xml = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!-- exported -->\n{}",
            v4()
        );
        assert_eq!(detect(xml.as_bytes()).unwrap(), Version::V4);
    }

    #[test]
    fn accepts_a_byte_order_mark() {
        let mut xml = b"\xEF\xBB\xBF".to_vec();
        xml.extend_from_slice(v4().as_bytes());
        assert_eq!(detect(&xml).unwrap(), Version::V4);
    }

    #[test]
    fn other_namespace_is_an_unsupported_version() {
        let xml = root("xmlns=\"http://www.auditfiles.nl/XAF/3.2\"");
        match detect(xml.as_bytes()) {
            Err(Error::UnsupportedVersion(ns)) => {
                assert_eq!(ns, "http://www.auditfiles.nl/XAF/3.2")
            }
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn other_version_is_reported_even_when_the_body_is_not_valid_for_4() {
        let xml = "<auditfile xmlns=\"urn:other\"><nonsense/></auditfile>";
        assert!(matches!(
            detect(xml.as_bytes()),
            Err(Error::UnsupportedVersion(_))
        ));
    }

    #[test]
    fn missing_namespace_is_not_an_auditfile() {
        assert!(matches!(
            detect(root("").as_bytes()),
            Err(Error::NotAnAuditfile(_))
        ));
    }

    #[test]
    fn undeclared_prefix_is_not_an_auditfile() {
        assert!(matches!(
            detect(b"<x:auditfile/>"),
            Err(Error::NotAnAuditfile(_))
        ));
    }

    #[test]
    fn other_root_element_is_not_an_auditfile() {
        let xml = format!("<invoice xmlns=\"{NAMESPACE_V4}\"/>");
        match detect(xml.as_bytes()) {
            Err(Error::NotAnAuditfile(message)) => assert!(message.contains("`invoice`")),
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn empty_input_has_no_root() {
        assert!(matches!(detect(b""), Err(Error::NotAnAuditfile(_))));
        assert!(matches!(detect(b"  \n"), Err(Error::NotAnAuditfile(_))));
    }

    #[test]
    fn broken_xml_before_the_root_is_an_xml_error() {
        assert!(matches!(detect(b"<!-- never closed"), Err(Error::Xml(_))));
    }
}
