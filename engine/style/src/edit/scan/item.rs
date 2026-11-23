//! Reading one item of a list (a `font`, `fill`, `border` or `xf`) into its
//! canonical form. An element of the styles part that the schema table lists
//! is read into a node, and any other (an unknown name, or an element in
//! another namespace) is kept as the bytes it was written in. White space,
//! comments and processing instructions between elements are not part of the
//! form; text that is not white space is refused.

use super::events::{Ev, Tag, Walk};
use crate::canonical::Node;
use crate::edit::StyleError;

impl Walk<'_> {
    /// Reads the element `tag` (opened, or empty) into `node`, the empty
    /// node of its element, with the attributes and children it has.
    pub(super) fn build(
        &mut self,
        mut node: Node,
        tag: &Tag,
        opened: bool,
    ) -> Result<Node, StyleError> {
        for (name, value) in &tag.attributes {
            node.push_attribute(name, value);
        }
        let mut open = opened;
        while open && let Some(event) = self.next()? {
            match event {
                Ev::Start(child) => self.child(&mut node, child, true)?,
                Ev::Empty(child) => self.child(&mut node, child, false)?,
                Ev::Text { blank: false, .. } => return Err(self.unexpected_text(&tag.name)),
                Ev::End(_) => open = false,
                Ev::Text { .. } | Ev::Other(_) => {}
            }
        }
        Ok(node.finish())
    }

    /// Adds the child `tag` of `parent`: a node when the table lists it
    /// among the children of `parent`, else the bytes it was written in.
    fn child(&mut self, parent: &mut Node, tag: Tag, opened: bool) -> Result<(), StyleError> {
        let main = tag.ns == Some(self.namespace);
        match main.then(|| parent.child_element(&tag.local)).flatten() {
            Some(node) => {
                let child = self.build(node, &tag, opened)?;
                parent.push_child(child);
            }
            None => {
                let local = main.then(|| tag.local.clone());
                let bytes = self.raw(tag, opened)?;
                parent.push_raw(local.as_deref(), bytes);
            }
        }
        Ok(())
    }
}
