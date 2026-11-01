use std::fs::File;
use std::io::BufReader;

use xaf_auditfile::{
    Auditfile, AuditfileStream, Error, Finding, FindingKind, Location, StreamedTransaction,
    Validator,
};

fn send_sync<T: Send + Sync>() {}

fn send<T: Send>() {}

#[test]
fn public_types_can_be_shared_between_threads() {
    send_sync::<Auditfile>();
    send_sync::<Error>();
    send_sync::<Finding>();
    send_sync::<FindingKind>();
    send_sync::<Location>();
    send_sync::<StreamedTransaction>();
    send_sync::<Validator>();
}

#[test]
fn a_stream_can_be_moved_to_another_thread() {
    send::<AuditfileStream<BufReader<File>>>();
}

#[test]
fn the_error_fits_in_a_boxed_error() {
    let error: Box<dyn std::error::Error + Send + Sync> = Box::new(Error::Encoding("x".into()));
    assert!(error.to_string().contains("encoding"));
}
