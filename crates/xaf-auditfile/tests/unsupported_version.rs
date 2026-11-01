use xaf_auditfile::{Auditfile, Error};

#[test]
fn a_3_2_file_is_an_unsupported_version() {
    match Auditfile::from_path("tests/fixtures/xaf32.xaf") {
        Err(Error::UnsupportedVersion(namespace)) => {
            assert_eq!(namespace, "http://www.auditfiles.nl/XAF/3.2");
        }
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn the_version_is_reported_before_the_contents_are_read() {
    let result = Auditfile::from_path("tests/fixtures/xaf32.xaf");
    assert!(!matches!(result, Err(Error::Deserialize(_))));
}
