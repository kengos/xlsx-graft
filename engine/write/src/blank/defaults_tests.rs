//! Tests for `DefaultFont` (what `new` refuses, and in which order) and for
//! `Defaults::new` (the `sheetFormatPr` and the font each combination of a
//! font and a row height gives).

use super::{DefaultFont, DefaultFontError, Defaults};
use crate::sheet::{SheetError, SheetFormat};

#[test]
fn a_default_font_takes_a_name_of_xml_characters_and_a_size_from_1_to_409() {
    for (name, size) in [
        ("Calibri", 1.0),
        ("Yu Gothic", 409.0),
        ("a", 10.5),
        ("中", 11.0),
    ] {
        let font = DefaultFont::new(name, size).unwrap();
        assert_eq!((font.name(), font.size()), (name, size));
    }
    let long = "x".repeat(300);
    assert!(DefaultFont::new(long, 11.0).is_ok());
    for name in ["", "a\u{1}b", "a\u{fffe}", "\u{ffff}"] {
        let got = DefaultFont::new(name, 11.0);
        assert_eq!(
            got,
            Err(DefaultFontError::Name {
                name: name.to_owned()
            })
        );
    }
    for size in [0.99, 409.01, 0.0, -1.0, f64::INFINITY, f64::NEG_INFINITY] {
        let got = DefaultFont::new("Calibri", size);
        assert_eq!(got, Err(DefaultFontError::Size { size }), "{size}");
    }
    assert!(matches!(
        DefaultFont::new("Calibri", f64::NAN),
        Err(DefaultFontError::Size { size }) if size.is_nan()
    ));
    // The name is checked first.
    let got = DefaultFont::new("", 0.0);
    assert_eq!(
        got,
        Err(DefaultFontError::Name {
            name: String::new()
        })
    );
}

#[test]
fn the_default_font_is_calibri_11_and_the_errors_say_what_they_refuse() {
    let font = DefaultFont::default();
    assert_eq!((font.name(), font.size()), ("Calibri", 11.0));
    let name = DefaultFontError::Name {
        name: "a".to_owned(),
    };
    assert_eq!(
        name.to_string(),
        r#"the font name "a" is empty or has a character XML does not allow"#
    );
    let size = DefaultFontError::Size { size: 0.5 };
    assert_eq!(
        size.to_string(),
        "the font size 0.5 is not a finite number from 1 to 409"
    );
}

#[test]
fn defaults_follow_the_three_rules_for_the_font_and_the_row_height_given() {
    let gothic = || DefaultFont::new("Yu Gothic", 11.0).unwrap();
    let none = Defaults::new(None, None).unwrap();
    assert_eq!(none, Defaults::default());
    assert_eq!(none.font(), &DefaultFont::default());
    assert_eq!(none.sheet_format(), SheetFormat::new(15.0).unwrap());
    assert!(!none.sheet_format().custom_height());

    let font_only = Defaults::new(Some(gothic()), None).unwrap();
    assert_eq!(font_only.font(), &gothic());
    assert_eq!(font_only.sheet_format(), SheetFormat::custom(15.0).unwrap());

    let calibri_only = Defaults::new(Some(DefaultFont::default()), None).unwrap();
    assert_eq!(calibri_only.font(), &DefaultFont::default());
    assert_eq!(
        calibri_only.sheet_format(),
        SheetFormat::custom(15.0).unwrap()
    );

    let height_only = Defaults::new(None, Some(20.0)).unwrap();
    assert_eq!(height_only.font(), &DefaultFont::default());
    assert_eq!(
        height_only.sheet_format(),
        SheetFormat::custom(20.0).unwrap()
    );

    let both = Defaults::new(Some(gothic()), Some(20.0)).unwrap();
    assert_eq!(both.font(), &gothic());
    assert_eq!(both.sheet_format(), SheetFormat::custom(20.0).unwrap());
}

#[test]
fn defaults_refuse_a_row_height_outside_0_to_409_with_or_without_a_font() {
    for height in [-0.1, 409.1, f64::INFINITY] {
        let error = Err(SheetError::RowHeight { height });
        assert_eq!(Defaults::new(None, Some(height)), error);
        let font = Some(DefaultFont::default());
        assert_eq!(Defaults::new(font, Some(height)), error);
    }
}
