use quick_xml::Reader;
use quick_xml::events::Event;

use crate::Error;

/// The deepest nesting accepted. The schema nests about eight levels deep. The reader keeps the
/// nesting level in a `u16`, which overflows on a file that nests tens of thousands of levels deep.
pub(crate) const MAX_DEPTH: usize = 128;

/// Whether text outside the root element is only whitespace, which is all XML allows there.
pub(crate) fn is_blank(text: &[u8]) -> bool {
    text.iter().all(u8::is_ascii_whitespace)
}

pub(crate) fn unclosed() -> Error {
    Error::Xml(quick_xml::Error::Syntax(
        quick_xml::errors::SyntaxError::UnclosedTag,
    ))
}

pub(crate) fn mixed_content() -> Error {
    Error::Deserialize(<quick_xml::DeError as serde::de::Error>::custom(
        "text mixed with elements",
    ))
}

pub(crate) fn outside_root() -> Error {
    Error::NotAnAuditfile("the file has content outside the root element".into())
}

/// Follows the elements being read and rejects text that is mixed with child elements, which
/// the schema never allows, and nesting deeper than [`MAX_DEPTH`].
#[derive(Debug, Default)]
pub(crate) struct Content {
    /// Per open element: whether it has child elements, and whether it has text.
    open: Vec<(bool, bool)>,
}

impl Content {
    pub(crate) fn depth(&self) -> usize {
        self.open.len()
    }

    /// An element, child of the current one, is opened or is empty.
    pub(crate) fn child(&mut self) -> Result<(), Error> {
        if let Some((has_children, has_text)) = self.open.last_mut() {
            if *has_text {
                return Err(mixed_content());
            }
            *has_children = true;
        }
        Ok(())
    }

    pub(crate) fn push(&mut self) -> Result<(), Error> {
        self.open.push((false, false));
        if self.open.len() > MAX_DEPTH {
            return Err(Error::NestingTooDeep(MAX_DEPTH));
        }
        Ok(())
    }

    pub(crate) fn pop(&mut self) {
        self.open.pop();
    }

    /// Text or CDATA in the current element. Whitespace alone is not content.
    pub(crate) fn text(&mut self, text: &[u8]) -> Result<(), Error> {
        if is_blank(text) {
            return Ok(());
        }
        if let Some((has_children, has_text)) = self.open.last_mut() {
            if *has_children {
                return Err(mixed_content());
            }
            *has_text = true;
        }
        Ok(())
    }
}

/// Checks the structure of the whole document before it is parsed: it is complete, has one root
/// element, has nothing but whitespace outside it, mixes no text with elements, and nests no
/// deeper than [`MAX_DEPTH`].
pub(crate) fn check(xml: &[u8]) -> Result<(), Error> {
    let mut reader = Reader::from_reader(xml);
    let mut content = Content::default();
    let mut roots = 0_usize;
    loop {
        match reader.read_event()? {
            Event::Start(_) => {
                if content.depth() == 0 {
                    roots += 1;
                }
                if roots > 1 {
                    return Err(outside_root());
                }
                content.child()?;
                content.push()?;
            }
            Event::Empty(_) => {
                if content.depth() == 0 {
                    roots += 1;
                }
                if roots > 1 {
                    return Err(outside_root());
                }
                content.child()?;
            }
            Event::End(_) => content.pop(),
            Event::Text(text) => {
                if content.depth() == 0 && !is_blank(&text) {
                    return Err(outside_root());
                }
                content.text(&text)?;
            }
            Event::CData(data) => {
                if content.depth() == 0 {
                    return Err(outside_root());
                }
                content.text(&data)?;
            }
            Event::Eof => {
                return if content.depth() == 0 {
                    Ok(())
                } else {
                    Err(unclosed())
                };
            }
            _ => {}
        }
    }
}
