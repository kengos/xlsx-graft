//! Tests of the crate's public values and of the number formats it writes:
//! `read_back` reads the formats back with `xlsx-graft-read`; `fixtures`
//! edits the styles part of every workbook in `fixtures/`, and `properties`
//! edits a small part with random styles.

mod fixtures;
mod properties;
mod read_back;

use crate::Style;

#[test]
fn a_number_format_style_states_the_number_format_alone() {
    let style = Style::number_format("0.00");
    assert_eq!(style.number_format.as_deref(), Some("0.00"));
    assert_eq!(
        style,
        Style {
            number_format: Some("0.00".to_owned()),
            ..Style::default()
        }
    );
    assert_eq!(
        Style::number_format(String::from("@"))
            .number_format
            .as_deref(),
        Some("@")
    );
}
