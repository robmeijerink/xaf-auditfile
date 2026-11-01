//! Incremental reading. The schema fixes the order of the elements and puts the transactions
//! last, so everything before the first journal is small enough to read as a whole, and the
//! transactions after it can be handed out one at a time.

use std::fmt;
use std::io::BufRead;
use std::sync::Arc;

use quick_xml::events::{BytesStart, Event};
use quick_xml::{NsReader, Writer};

use super::{map, raw};
use crate::document::{Content, is_blank, mixed_content, outside_root, unclosed};
use crate::version;
use crate::{Auditfile, Error, Journal, Transaction};

/// A transaction together with the journal it belongs to, as handed out by [`AuditfileStream`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct StreamedTransaction {
    /// The journal, without its transactions. All transactions of one journal share it.
    pub journal: Arc<Journal>,
    /// The transaction.
    pub transaction: Transaction,
}

/// An auditfile that is read one transaction at a time.
///
/// Created with [`Auditfile::stream_from_reader`] or [`Auditfile::stream_from_path`]. Iterating
/// yields every transaction in document order. After the first error nothing more is yielded.
pub struct AuditfileStream<R> {
    auditfile: Auditfile,
    reader: NsReader<R>,
    declaration: Vec<u8>,
    state: State,
    buf: Vec<u8>,
    scratch: Vec<u8>,
    fragment: Vec<u8>,
}

impl<R> fmt::Debug for AuditfileStream<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuditfileStream")
            .field("auditfile", &self.auditfile)
            .finish_non_exhaustive()
    }
}

enum State {
    AtJournalStart,
    InJournal(Arc<Journal>),
    BetweenJournals,
    Finished,
}

impl<R> AuditfileStream<R> {
    /// Everything before the first journal: header, company, master data, opening balance and the
    /// control totals of the transactions. `journals` is empty.
    pub fn auditfile(&self) -> &Auditfile {
        &self.auditfile
    }
}

impl<R: BufRead> Iterator for AuditfileStream<R> {
    type Item = Result<StreamedTransaction, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.advance().transpose()
    }
}

pub(crate) fn open<R: BufRead>(mut reader: R) -> Result<AuditfileStream<R>, Error> {
    version::check_encoding(reader.fill_buf()?)?;
    let mut reader = NsReader::from_reader(reader);
    let mut buf = Vec::new();
    let mut scratch = Vec::new();

    let root_is_empty = loop {
        buf.clear();
        let (namespace, event) = reader.read_resolved_event_into(&mut buf)?;
        match event {
            Event::Start(root) => {
                version::classify(&namespace, &root)?;
                break false;
            }
            Event::Empty(root) => {
                version::classify(&namespace, &root)?;
                break true;
            }
            Event::Decl(declaration) => {
                declaration.xml_version()?;
            }
            Event::Text(text) if !is_blank(&text) => return Err(outside_root()),
            Event::Eof => {
                return Err(Error::NotAnAuditfile("the file has no root element".into()));
            }
            _ => {}
        }
    };

    let declaration = format!(
        "<?xml version=\"1.0\" encoding=\"{}\"?>",
        reader.decoder().encoding().name()
    )
    .into_bytes();
    let mut master = declaration.clone();
    master.extend_from_slice(b"<auditfile>");

    let state = if root_is_empty {
        State::Finished
    } else {
        match read_master(&mut reader, &mut master, &mut buf, &mut scratch)? {
            Progress::JournalStart => {
                master.extend_from_slice(b"</transactions></company></auditfile>");
                State::AtJournalStart
            }
            Progress::Complete => {
                master.extend_from_slice(b"</auditfile>");
                State::Finished
            }
        }
    };
    if root_is_empty {
        master.extend_from_slice(b"</auditfile>");
    }

    let auditfile = map::map(raw::deserialize(&master)?)?;
    Ok(AuditfileStream {
        auditfile,
        reader,
        declaration,
        state,
        buf,
        scratch,
        fragment: Vec::new(),
    })
}

enum Progress {
    JournalStart,
    Complete,
}

/// Copies the elements before the first journal into `master`, in the shape of a file without
/// journals. Stops after the opening tag of the first journal, or reads to the end of the file.
fn read_master<R: BufRead>(
    reader: &mut NsReader<R>,
    master: &mut Vec<u8>,
    buf: &mut Vec<u8>,
    scratch: &mut Vec<u8>,
) -> Result<Progress, Error> {
    loop {
        buf.clear();
        match reader.read_event_into(buf)? {
            Event::Start(e) => match e.local_name().as_ref() {
                b"company" => {
                    master.extend_from_slice(b"<company>");
                    if let Progress::JournalStart = read_company(reader, master, scratch)? {
                        return Ok(Progress::JournalStart);
                    }
                }
                b"header" => copy_subtree(reader, &e, master, scratch)?,
                _ => skip_subtree(reader, scratch)?,
            },
            Event::Empty(e) => {
                if matches!(e.local_name().as_ref(), b"header" | b"company") {
                    write(master, Event::Empty(e))?;
                }
            }
            Event::End(_) => {
                drain(reader, buf, 0)?;
                return Ok(Progress::Complete);
            }
            Event::Text(text) if !is_blank(&text) => return Err(mixed_content()),
            Event::CData(_) => return Err(mixed_content()),
            Event::Eof => return Err(unclosed()),
            _ => {}
        }
    }
}

fn read_company<R: BufRead>(
    reader: &mut NsReader<R>,
    master: &mut Vec<u8>,
    scratch: &mut Vec<u8>,
) -> Result<Progress, Error> {
    let mut buf = Vec::new();
    let mut transactions_seen = false;
    loop {
        buf.clear();
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) | Event::Empty(e) if transactions_seen => {
                return Err(out_of_order(&e));
            }
            Event::Start(e) if e.local_name().as_ref() == b"transactions" => {
                transactions_seen = true;
                master.extend_from_slice(b"<transactions>");
                if let Progress::JournalStart = read_transactions(reader, master, scratch)? {
                    return Ok(Progress::JournalStart);
                }
            }
            Event::Start(e) => copy_subtree(reader, &e, master, scratch)?,
            Event::Empty(e) => write(master, Event::Empty(e))?,
            Event::End(_) => {
                master.extend_from_slice(b"</company>");
                return Ok(Progress::Complete);
            }
            Event::Text(text) if !is_blank(&text) => return Err(mixed_content()),
            Event::CData(_) => return Err(mixed_content()),
            Event::Eof => return Err(unclosed()),
            _ => {}
        }
    }
}

fn read_transactions<R: BufRead>(
    reader: &mut NsReader<R>,
    master: &mut Vec<u8>,
    scratch: &mut Vec<u8>,
) -> Result<Progress, Error> {
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) if e.local_name().as_ref() == b"journal" => {
                return Ok(Progress::JournalStart);
            }
            Event::Start(e) => copy_subtree(reader, &e, master, scratch)?,
            Event::Empty(e) => write(master, Event::Empty(e))?,
            Event::End(_) => {
                master.extend_from_slice(b"</transactions>");
                return Ok(Progress::Complete);
            }
            Event::Text(text) if !is_blank(&text) => return Err(mixed_content()),
            Event::CData(_) => return Err(mixed_content()),
            Event::Eof => return Err(unclosed()),
            _ => {}
        }
    }
}

impl<R: BufRead> AuditfileStream<R> {
    fn advance(&mut self) -> Result<Option<StreamedTransaction>, Error> {
        loop {
            match std::mem::replace(&mut self.state, State::Finished) {
                State::Finished => return Ok(None),
                State::AtJournalStart => {
                    let (journal, first) = self.read_journal_header()?;
                    match first {
                        Some(start) => return self.transaction_in(journal, &start).map(Some),
                        None => self.state = State::BetweenJournals,
                    }
                }
                State::InJournal(journal) => {
                    self.buf.clear();
                    match self.reader.read_event_into(&mut self.buf)? {
                        Event::Start(e) if e.local_name().as_ref() == b"transaction" => {
                            let start = e.into_owned();
                            return self.transaction_in(journal, &start).map(Some);
                        }
                        Event::Start(_) => {
                            skip_subtree(&mut self.reader, &mut self.scratch)?;
                            self.state = State::InJournal(journal);
                        }
                        Event::End(_) => self.state = State::BetweenJournals,
                        Event::Text(text) if !is_blank(&text) => return Err(mixed_content()),
                        Event::CData(_) => return Err(mixed_content()),
                        Event::Eof => return Err(unclosed()),
                        _ => self.state = State::InJournal(journal),
                    }
                }
                State::BetweenJournals => {
                    self.buf.clear();
                    match self.reader.read_event_into(&mut self.buf)? {
                        Event::Start(e) if e.local_name().as_ref() == b"journal" => {
                            self.state = State::AtJournalStart;
                        }
                        Event::Start(_) => {
                            skip_subtree(&mut self.reader, &mut self.scratch)?;
                            self.state = State::BetweenJournals;
                        }
                        Event::End(_) => {
                            drain(&mut self.reader, &mut self.buf, 2)?;
                            return Ok(None);
                        }
                        Event::Text(text) if !is_blank(&text) => return Err(mixed_content()),
                        Event::CData(_) => return Err(mixed_content()),
                        Event::Eof => return Err(unclosed()),
                        _ => self.state = State::BetweenJournals,
                    }
                }
            }
        }
    }

    /// Reads the elements of a journal that precede its first transaction. Returns the journal and
    /// the opening tag of the first transaction, if there is one.
    fn read_journal_header(
        &mut self,
    ) -> Result<(Arc<Journal>, Option<BytesStart<'static>>), Error> {
        self.fragment.clear();
        self.fragment.extend_from_slice(&self.declaration);
        self.fragment.extend_from_slice(b"<journal>");
        let first = loop {
            self.buf.clear();
            match self.reader.read_event_into(&mut self.buf)? {
                Event::Start(e) if e.local_name().as_ref() == b"transaction" => {
                    break Some(e.into_owned());
                }
                Event::Start(e) => {
                    copy_subtree(&mut self.reader, &e, &mut self.fragment, &mut self.scratch)?;
                }
                Event::Empty(e) => write(&mut self.fragment, Event::Empty(e))?,
                Event::End(_) => break None,
                Event::Text(text) if !is_blank(&text) => return Err(mixed_content()),
                Event::CData(_) => return Err(mixed_content()),
                Event::Eof => return Err(unclosed()),
                _ => {}
            }
        };
        self.fragment.extend_from_slice(b"</journal>");
        let journal = map::journal(raw::deserialize(&self.fragment)?)?;
        Ok((Arc::new(journal), first))
    }

    fn transaction_in(
        &mut self,
        journal: Arc<Journal>,
        start: &BytesStart<'_>,
    ) -> Result<StreamedTransaction, Error> {
        self.fragment.clear();
        self.fragment.extend_from_slice(&self.declaration);
        copy_subtree(
            &mut self.reader,
            start,
            &mut self.fragment,
            &mut self.scratch,
        )?;
        let transaction = map::transaction(journal.id.as_str(), raw::deserialize(&self.fragment)?)?;
        self.state = State::InJournal(Arc::clone(&journal));
        Ok(StreamedTransaction {
            journal,
            transaction,
        })
    }
}

/// Copies an element, whose opening tag has just been read, with all it contains.
fn copy_subtree<R: BufRead>(
    reader: &mut NsReader<R>,
    start: &BytesStart<'_>,
    out: &mut Vec<u8>,
    scratch: &mut Vec<u8>,
) -> Result<(), Error> {
    let mut writer = Writer::new(out);
    writer.write_event(Event::Start(start.borrow()))?;
    let mut content = Content::default();
    content.push()?;
    while content.depth() > 0 {
        scratch.clear();
        let event = reader.read_event_into(scratch)?;
        match &event {
            Event::Start(_) => {
                content.child()?;
                content.push()?;
            }
            Event::Empty(_) => content.child()?,
            Event::End(_) => content.pop(),
            Event::Text(text) => content.text(text)?,
            Event::CData(data) => content.text(data)?,
            Event::Eof => return Err(unclosed()),
            Event::Decl(_) | Event::DocType(_) | Event::PI(_) | Event::Comment(_) => continue,
            _ => {}
        }
        writer.write_event(event)?;
    }
    Ok(())
}

/// Skips an element, whose opening tag has just been read, with all it contains.
fn skip_subtree<R: BufRead>(reader: &mut NsReader<R>, scratch: &mut Vec<u8>) -> Result<(), Error> {
    let mut content = Content::default();
    content.push()?;
    while content.depth() > 0 {
        scratch.clear();
        match reader.read_event_into(scratch)? {
            Event::Start(_) => {
                content.child()?;
                content.push()?;
            }
            Event::Empty(_) => content.child()?,
            Event::End(_) => content.pop(),
            Event::Text(text) => content.text(&text)?,
            Event::CData(data) => content.text(&data)?,
            Event::Eof => return Err(unclosed()),
            _ => {}
        }
    }
    Ok(())
}

fn write(out: &mut Vec<u8>, event: Event<'_>) -> Result<(), Error> {
    Writer::new(out).write_event(event)?;
    Ok(())
}

/// Reads the rest of the file, where `open` elements are still to be closed and nothing else may
/// follow.
fn drain<R: BufRead>(
    reader: &mut NsReader<R>,
    buf: &mut Vec<u8>,
    mut open: usize,
) -> Result<(), Error> {
    loop {
        buf.clear();
        match reader.read_event_into(buf)? {
            Event::Start(e) | Event::Empty(e) if open > 0 => return Err(out_of_order(&e)),
            Event::Start(_) | Event::Empty(_) => return Err(outside_root()),
            Event::End(_) => open = open.saturating_sub(1),
            Event::Text(text) if !is_blank(&text) => return Err(stray_text(open)),
            Event::CData(_) => return Err(stray_text(open)),
            Event::Eof => return if open == 0 { Ok(()) } else { Err(unclosed()) },
            _ => {}
        }
    }
}

/// Text after the transactions: inside an element that is still open it is mixed with elements,
/// otherwise it is outside the root element.
fn stray_text(open: usize) -> Error {
    if open > 0 {
        mixed_content()
    } else {
        outside_root()
    }
}

fn out_of_order(element: &BytesStart<'_>) -> Error {
    Error::OutOfOrder(String::from_utf8_lossy(element.local_name().as_ref()).into_owned())
}
