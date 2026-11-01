mod map;
mod raw;
mod stream;

pub(crate) use stream::open as stream;
pub use stream::{AuditfileStream, StreamedTransaction};

use crate::{Auditfile, Error};

pub(crate) fn parse(xml: &[u8]) -> Result<Auditfile, Error> {
    map::map(raw::deserialize(xml)?)
}
