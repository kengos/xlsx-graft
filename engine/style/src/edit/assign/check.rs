//! Checking the values of a [`Style`] before it is laid over a cell format:
//! the ones the XML writer would write into a styles part Excel refuses.

use crate::edit::StyleError;
use crate::{Alignment, Border, Color, Fill, Font, Style};
use xlsx_graft_xml::is_xml_char;

/// Checks `style`; `InvalidValue` names the attribute or element of the
/// first value that is out of range.
pub(super) fn check(style: &Style) -> Result<(), StyleError> {
    if let Some(font) = &style.font {
        font_ok(font)?;
    }
    if let Some(Fill::Pattern {
        foreground,
        background,
        ..
    }) = &style.fill
    {
        color_ok(foreground.as_ref())?;
        color_ok(background.as_ref())?;
    }
    if let Some(border) = &style.border {
        border_ok(border)?;
    }
    if let Some(code) = &style.number_format {
        text_ok(code, "formatCode")?;
    }
    if let Some(alignment) = &style.alignment {
        alignment_ok(alignment)?;
    }
    Ok(())
}

fn font_ok(font: &Font) -> Result<(), StyleError> {
    if let Some(name) = &font.name {
        text_ok(name, "name")?;
    }
    // 409: the largest size Excel accepts (ECMA-376-1:2016 §18.4.11 gives none).
    let size_ok = |size: f64| size.is_finite() && (1.0..=409.0).contains(&size);
    ok(font.size.is_none_or(size_ok), "sz")?;
    color_ok(font.color.as_ref())
}

fn border_ok(border: &Border) -> Result<(), StyleError> {
    let sides = [
        &border.left,
        &border.right,
        &border.top,
        &border.bottom,
        &border.diagonal,
    ];
    for side in sides.into_iter().flatten() {
        color_ok(side.color.as_ref())?;
    }
    Ok(())
}

fn alignment_ok(alignment: &Alignment) -> Result<(), StyleError> {
    // 0 to 180 degrees, and 255 for stacked text (ECMA-376-1:2016 §18.8.1).
    let rotation_ok = |degrees: u8| degrees <= 180 || degrees == 255;
    ok(
        alignment.text_rotation.is_none_or(rotation_ok),
        "textRotation",
    )
}

/// A theme color's tint is a finite number from -1 to 1.
fn color_ok(color: Option<&Color>) -> Result<(), StyleError> {
    let tint_ok = |tint: f64| tint.is_finite() && (-1.0..=1.0).contains(&tint);
    let Some(Color::Theme { tint, .. }) = color else {
        return Ok(());
    };
    ok(tint.is_none_or(tint_ok), "tint")
}

/// Text that is not empty and holds only characters XML allows.
fn text_ok(text: &str, field: &'static str) -> Result<(), StyleError> {
    ok(!text.is_empty() && text.chars().all(is_xml_char), field)
}

fn ok(valid: bool, field: &'static str) -> Result<(), StyleError> {
    if valid {
        Ok(())
    } else {
        Err(StyleError::InvalidValue { field })
    }
}
