//! Writing a [`Node`] as XML, and the `numFmt` element.

use super::{Child, Node, Value};

impl Node {
    /// Writes the node and what is in it to `out`. The element names have
    /// the prefix `prefix` and a colon, or none when `prefix` is empty. No
    /// white space is written between elements. The attributes are written
    /// in the table's order, then the foreign ones; the children in the order
    /// of the table, a child of the `write_empty` list that is missing as an
    /// empty element at the rank of that name.
    pub(crate) fn write(&self, prefix: &[u8], out: &mut Vec<u8>) {
        let name = self.element.name.as_bytes();
        open(prefix, name, out);
        for (name, value) in &self.attributes {
            attribute(name.as_bytes(), &value.text(), out);
        }
        for (name, value) in &self.foreign {
            attribute(name, value, out);
        }
        if self.children.is_empty() && self.element.write_empty.is_empty() {
            out.extend_from_slice(b"/>");
            return;
        }
        out.push(b'>');
        self.write_children(prefix, out);
        out.extend_from_slice(b"</");
        qualify(prefix, name, out);
        out.push(b'>');
    }

    fn write_children(&self, prefix: &[u8], out: &mut Vec<u8>) {
        let mut next = 0;
        for (rank, name) in self.element.children.iter().enumerate() {
            let before = next;
            while let Some(child) = self.children.get(next) {
                if self.rank_of_child(child) != rank {
                    break;
                }
                child.write(prefix, out);
                next += 1;
            }
            if next == before && self.element.write_empty.contains(name) {
                open(prefix, name.as_bytes(), out);
                out.extend_from_slice(b"/>");
            }
        }
        for child in self.children.iter().skip(next) {
            child.write(prefix, out);
        }
    }

    fn rank_of_child(&self, child: &Child) -> usize {
        match child {
            Child::Node(node) => self.rank_of(node.element.name),
            Child::Raw { rank, .. } => *rank,
        }
    }
}

impl Child {
    fn write(&self, prefix: &[u8], out: &mut Vec<u8>) {
        match self {
            Self::Node(node) => node.write(prefix, out),
            Self::Raw { bytes, .. } => out.extend_from_slice(bytes),
        }
    }
}

impl Value {
    /// The text of the value as an attribute holds it, before escaping.
    fn text(&self) -> String {
        match self {
            Self::Bool(true) => "1".to_owned(),
            Self::Bool(false) => "0".to_owned(),
            Self::Uint(number) => number.to_string(),
            Self::Int(number) => number.to_string(),
            Self::Double(bits) => f64::from_bits(*bits).to_string(),
            Self::Hex(number) => format!("{number:08X}"),
            Self::Token(text) => (*text).to_owned(),
            Self::Text(text) | Self::Raw(text) => text.clone(),
        }
    }
}

/// Writes the `numFmt` element (ECMA-376-1:2016 §18.8.30) that defines the
/// number format `id` as `code`, its names prefixed as `Node::write`'s are.
pub(crate) fn write_number_format(prefix: &[u8], id: u32, code: &str, out: &mut Vec<u8>) {
    open(prefix, b"numFmt", out);
    attribute(b"numFmtId", &id.to_string(), out);
    attribute(b"formatCode", code, out);
    out.extend_from_slice(b"/>");
}

/// Writes `<` and the qualified `name`.
fn open(prefix: &[u8], name: &[u8], out: &mut Vec<u8>) {
    out.push(b'<');
    qualify(prefix, name, out);
}

/// Writes `name` after `prefix` and a colon, or alone when there is no
/// prefix.
fn qualify(prefix: &[u8], name: &[u8], out: &mut Vec<u8>) {
    if !prefix.is_empty() {
        out.extend_from_slice(prefix);
        out.push(b':');
    }
    out.extend_from_slice(name);
}

/// Writes ` name="value"`.
fn attribute(name: &[u8], value: &str, out: &mut Vec<u8>) {
    out.push(b' ');
    out.extend_from_slice(name);
    out.extend_from_slice(b"=\"");
    escape(value, out);
    out.push(b'"');
}

/// Writes `value` as the text of an attribute: `&`, `<`, `>` and `"` as
/// entities, and tab, line feed and carriage return as character references,
/// which a reader would otherwise normalize to a space.
fn escape(value: &str, out: &mut Vec<u8>) {
    for &byte in value.as_bytes() {
        match byte {
            b'&' => out.extend_from_slice(b"&amp;"),
            b'<' => out.extend_from_slice(b"&lt;"),
            b'>' => out.extend_from_slice(b"&gt;"),
            b'"' => out.extend_from_slice(b"&quot;"),
            b'\t' => out.extend_from_slice(b"&#9;"),
            b'\n' => out.extend_from_slice(b"&#10;"),
            b'\r' => out.extend_from_slice(b"&#13;"),
            _ => out.push(byte),
        }
    }
}
