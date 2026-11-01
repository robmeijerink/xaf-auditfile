#![allow(clippy::unwrap_used)]

use xaf_auditfile::{Auditfile, Error, StreamedTransaction, Transaction};

const WEBSHOP: &str = include_str!("fixtures/webshop.xaf");

fn streamed(bytes: &[u8]) -> (Result<Vec<StreamedTransaction>, Error>, Option<Auditfile>) {
    let stream = match Auditfile::stream_from_reader(bytes) {
        Ok(stream) => stream,
        Err(error) => return (Err(error), None),
    };
    let master = stream.auditfile().clone();
    (stream.collect(), Some(master))
}

fn transactions(file: &Auditfile) -> Vec<Transaction> {
    file.transactions().map(|(_, t)| t.clone()).collect()
}

/// Deterministic pseudo-random numbers, enough to pick positions and bytes.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next()).unwrap() % bound
    }
}

#[test]
fn a_file_cut_off_anywhere_never_panics_and_only_yields_complete_transactions() {
    let full = Auditfile::from_reader(WEBSHOP.as_bytes()).unwrap();
    let expected = transactions(&full);

    for cut in 0..WEBSHOP.trim_end().len() {
        let Some(prefix) = WEBSHOP.as_bytes().get(..cut) else {
            continue;
        };
        let (result, _) = streamed(prefix);
        let yielded: Vec<Transaction> = match result {
            Ok(items) => items.into_iter().map(|i| i.transaction).collect(),
            Err(_) => Vec::new(),
        };
        assert!(
            expected.starts_with(&yielded),
            "cut at {cut}: yielded transactions are not a prefix of the full file"
        );
        assert!(Auditfile::from_reader(prefix).is_err(), "cut at {cut}");
    }
}

#[test]
fn a_cut_off_file_always_ends_in_an_error_when_streamed() {
    for cut in (0..WEBSHOP.trim_end().len()).step_by(7) {
        let prefix = WEBSHOP.as_bytes().get(..cut).unwrap();
        let (result, _) = streamed(prefix);
        assert!(
            result.is_err(),
            "cut at {cut} was streamed without an error"
        );
    }
}

#[test]
fn single_byte_changes_give_the_same_outcome_whole_and_streamed() {
    let mut random = Lcg(0x5EED);
    let replacements = *b"<>&/\"' =x9";

    for round in 0..4000 {
        let mut bytes = WEBSHOP.as_bytes().to_vec();
        let at = random.below(bytes.len());
        let with = replacements[random.below(replacements.len())];
        bytes[at] = with;

        let whole = Auditfile::from_reader(bytes.as_slice());
        let (stream, master) = streamed(&bytes);

        match (&whole, &stream) {
            (Ok(file), Ok(items)) => {
                let mut without_journals = file.clone();
                without_journals.journals.clear();
                assert_eq!(master.as_ref(), Some(&without_journals), "round {round}");
                let from_stream: Vec<_> = items.iter().map(|i| i.transaction.clone()).collect();
                assert_eq!(from_stream, transactions(file), "round {round}");
            }
            (Err(_), Err(_)) => {}
            (Ok(_), Err(error)) => {
                panic!("round {round}: whole read succeeded but streaming failed: {error}")
            }
            (Err(error), Ok(_)) => {
                panic!("round {round}: streaming succeeded but whole read failed: {error}")
            }
        }
    }
}

#[test]
fn removing_a_slice_gives_the_same_outcome_whole_and_streamed() {
    let mut random = Lcg(0xFACE);
    for round in 0..2000 {
        let start = random.below(WEBSHOP.len());
        let length = 1 + random.below(40);
        let mut bytes = WEBSHOP.as_bytes().to_vec();
        bytes.drain(start..(start + length).min(WEBSHOP.len()));

        let whole = Auditfile::from_reader(bytes.as_slice());
        let (stream, _) = streamed(&bytes);
        assert_eq!(
            whole.is_ok(),
            stream.is_ok(),
            "round {round}: removed {length} bytes at {start}"
        );
    }
}
