//! Tests for `xml_text` and `xml_attribute`: each character replaced, and
//! the pieces handed to `out`.

use super::{xml_attribute, xml_text};
use std::convert::Infallible;

/// [`xml_text`] of `text`.
fn text(text: &str) -> String {
    let mut value = String::new();
    xml_text::<Infallible>(text, |piece| {
        value.push_str(piece);
        Ok(())
    })
    .unwrap();
    value
}

/// [`xml_attribute`] of `text`.
fn attribute(text: &str) -> String {
    let mut value = String::new();
    xml_attribute::<Infallible>(text, |piece| {
        value.push_str(piece);
        Ok(())
    })
    .unwrap();
    value
}

#[test]
fn text_replaces_the_markup_characters_and_the_carriage_return() {
    assert_eq!(
        text("a&b<c>d\re\n\t\"f'é"),
        "a&amp;b&lt;c&gt;d&#13;e\n\t\"f'é"
    );
    assert_eq!(text(""), "");
}

#[test]
fn an_attribute_also_replaces_the_quote_the_tab_and_the_line_feed() {
    assert_eq!(
        attribute("a&b<c>d\re\n\t\"f'é"),
        "a&amp;b&lt;c&gt;d&#13;e&#10;&#9;&quot;f'é"
    );
    assert_eq!(attribute(""), "");
}

#[test]
fn the_pieces_are_the_text_between_the_replacements_and_each_replacement() {
    let mut pieces = Vec::new();
    xml_text::<Infallible>("a&b", |piece| {
        pieces.push(piece.to_owned());
        Ok(())
    })
    .unwrap();
    assert_eq!(pieces, ["a", "&amp;", "b"]);
}

#[test]
fn an_error_of_out_stops_the_writing() {
    let mut calls = 0;
    let result = xml_attribute("a&b", |_| {
        calls += 1;
        Err("stop")
    });
    assert_eq!((result, calls), (Err("stop"), 1));
    let result = xml_text("&", |piece| {
        if piece.is_empty() {
            Ok(())
        } else {
            Err(piece.len())
        }
    });
    assert_eq!(result, Err(5));
}
