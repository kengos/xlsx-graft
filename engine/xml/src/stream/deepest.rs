//! [`XmlStreamReader::deepest`]: reading on past a too-deep element to
//! count how deep the part nests, which only the in-memory reader does.

use super::XmlStreamReader;
use quick_xml::events::Event;
use std::io::BufRead;

impl<R: BufRead> XmlStreamReader<R> {
    /// How deep the part nests, reading on from the element one past
    /// `max_depth` that stopped the reader. Only counts: it stops at the end
    /// or at the first error, an event past `max_event_bytes` included, so a
    /// malformed rest reports the deepest before it.
    pub(crate) fn deepest(&mut self) -> u64 {
        let mut deepest = self.guard.limits().max_depth.value.saturating_add(1);
        // Past `max_depth` when the element refused is a start tag, left open.
        let mut depth = self.guard.depth();
        let mut buf = Vec::new();
        loop {
            buf.clear();
            self.reader.get_mut().begin();
            let event = self.reader.read_event_into(&mut buf);
            if self.reader.get_ref().too_large().is_some() {
                return deepest;
            }
            match event {
                Ok(Event::Start(_)) => {
                    depth = depth.saturating_add(1);
                    deepest = deepest.max(depth);
                }
                Ok(Event::Empty(_)) => deepest = deepest.max(depth.saturating_add(1)),
                Ok(Event::End(_)) => depth = depth.saturating_sub(1),
                Ok(Event::Eof) | Err(_) => return deepest,
                Ok(_) => {}
            }
        }
    }
}
