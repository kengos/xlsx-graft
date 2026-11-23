//! Stating a [`Style`](crate::Style)'s values on the canonical form of an
//! item of the styles part. A value that is `None` leaves what the item has;
//! one that is `Some` replaces it. What the item has and the model does not
//! name (foreign attributes, raw children) stays. Which `fontId`, `fillId`
//! and so on an `xf` refers to, and its `apply*` attributes, are decided
//! by the caller.

use super::schema::{self, Element};
use super::{Node, Value};
use crate::{Alignment, Border, BorderSide, BorderStyle, Color, Fill, Font, Protection};

/// States `value` on `font`.
///
/// Stating a name removes the font's `scheme`: a font with a theme scheme is
/// shown in the theme's font, whatever its name says
/// (ECMA-376-1:2016 §18.8.35).
///
/// `Node::set_child` sorts the children after each change, so the font is in
/// the order `Node::finish` gives when this returns, and no call to `finish`
/// is needed.
pub(crate) fn apply_font(font: &mut Node, value: &Font) {
    if let Some(name) = &value.name {
        put_val(font, &schema::NAME, Value::Text(name.clone()));
        font.set_child(schema::SCHEME.name, None);
    }
    if let Some(size) = value.size {
        put_val(font, &schema::SZ, Value::double(size));
    }
    let flags = [
        (&schema::B, value.bold),
        (&schema::I, value.italic),
        (&schema::STRIKE, value.strike),
    ];
    for (element, flag) in flags {
        if let Some(on) = flag {
            put_val(font, element, Value::Bool(on));
        }
    }
    if let Some(underline) = value.underline {
        put_val(font, &schema::U, Value::Token(underline.as_str()));
    }
    if let Some(align) = value.vertical_align {
        put_val(font, &schema::VERT_ALIGN, Value::Token(align.as_str()));
    }
    if let Some(color) = &value.color {
        let node = color_node(&schema::COLOR, color);
        font.set_child(schema::COLOR.name, Some(node));
    }
}

/// The `fill` that `value` states. A fill is stated whole: the base is not
/// looked at.
pub(crate) fn fill_node(value: &Fill) -> Node {
    let Fill::Pattern {
        pattern,
        foreground,
        background,
    } = value;
    let mut pattern_fill = Node::new(&schema::PATTERN_FILL);
    pattern_fill.set_attribute("patternType", Some(Value::Token(pattern.as_str())));
    for (element, color) in [
        (&schema::FG_COLOR, foreground),
        (&schema::BG_COLOR, background),
    ] {
        if let Some(color) = color {
            pattern_fill.push_child(color_node(element, color));
        }
    }
    let mut fill = Node::new(&schema::FILL);
    fill.push_child(pattern_fill);
    fill
}

/// States `value` on `border`. A side is replaced whole; a side with the line
/// style none is removed.
pub(crate) fn apply_border(border: &mut Node, value: &Border) {
    let sides = [
        (&schema::LEFT, &value.left),
        (&schema::RIGHT, &value.right),
        (&schema::TOP, &value.top),
        (&schema::BOTTOM, &value.bottom),
        (&schema::DIAGONAL, &value.diagonal),
    ];
    for (element, side) in sides {
        if let Some(side) = side {
            border.set_child(element.name, Some(side_node(element, side)));
        }
    }
    put(border, "diagonalUp", value.diagonal_up.map(Value::Bool));
    put(border, "diagonalDown", value.diagonal_down.map(Value::Bool));
}

/// States `value` on the `alignment` of `xf`, which has none when it is
/// empty.
pub(crate) fn apply_alignment(xf: &mut Node, value: &Alignment) {
    let mut node = existing(xf, &schema::ALIGNMENT);
    put(
        &mut node,
        "horizontal",
        value.horizontal.map(|h| Value::Token(h.as_str())),
    );
    put(
        &mut node,
        "vertical",
        value.vertical.map(|v| Value::Token(v.as_str())),
    );
    put(&mut node, "wrapText", value.wrap_text.map(Value::Bool));
    put(
        &mut node,
        "shrinkToFit",
        value.shrink_to_fit.map(Value::Bool),
    );
    put(&mut node, "indent", value.indent.map(Value::Uint));
    let rotation = value
        .text_rotation
        .map(|degrees| Value::Uint(degrees.into()));
    put(&mut node, "textRotation", rotation);
    xf.set_child(schema::ALIGNMENT.name, Some(node));
}

/// States `value` on the `protection` of `xf`, which has none when it is
/// empty.
pub(crate) fn apply_protection(xf: &mut Node, value: &Protection) {
    let mut node = existing(xf, &schema::PROTECTION);
    put(&mut node, "locked", value.locked.map(Value::Bool));
    put(&mut node, "hidden", value.hidden.map(Value::Bool));
    xf.set_child(schema::PROTECTION.name, Some(node));
}

/// The color element `element` (`color`, `fgColor` or `bgColor`) that
/// `value` states.
pub(crate) fn color_node(element: &'static Element, value: &Color) -> Node {
    let mut node = Node::new(element);
    match value {
        Color::Rgb([alpha, red, green, blue]) => {
            let argb = u32::from_be_bytes([*alpha, *red, *green, *blue]);
            node.set_attribute("rgb", Some(Value::Hex(argb)));
        }
        Color::Theme { index, tint } => {
            node.set_attribute("theme", Some(Value::Uint(*index)));
            node.set_attribute("tint", tint.map(Value::double));
        }
    }
    node
}

/// A side `element` of a border that `side` states: without a line there is
/// nothing to color, so the color is not written and the side has no effect.
fn side_node(element: &'static Element, side: &BorderSide) -> Node {
    let mut node = Node::new(element);
    node.set_attribute("style", Some(Value::Token(side.style.as_str())));
    if let (Some(color), true) = (&side.color, side.style != BorderStyle::None) {
        node.push_child(color_node(&schema::COLOR, color));
    }
    node
}

/// Replaces the child `element` of `parent` with one whose attribute `val` is
/// `value`. The child is removed when that has no effect (`<b val="0"/>`).
fn put_val(parent: &mut Node, element: &'static Element, value: Value) {
    let mut node = Node::new(element);
    node.set_attribute("val", Some(value));
    parent.set_child(element.name, Some(node));
}

/// Sets the attribute `name` of `node` to `value` when there is one.
fn put(node: &mut Node, name: &'static str, value: Option<Value>) {
    if value.is_some() {
        node.set_attribute(name, value);
    }
}

/// The child `element` of `xf`, or an empty one.
fn existing(xf: &Node, element: &'static Element) -> Node {
    (xf.child(element.name).cloned()).unwrap_or_else(|| Node::new(element))
}
