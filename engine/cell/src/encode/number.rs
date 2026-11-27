//! A cell's number as the text of its `<v>`: the shortest that reads back
//! to the same `f64`, without a locale. The `<v>` is an ST_Xstring
//! (ECMA-376-1:2016 §22.9.2.19); the number in it is written in the lexical
//! form of `xsd:double`.

use crate::CellError;

/// The greatest magnitude an `f64` holds every integer up to: 2^53.
const EXACT: u64 = 1 << 53;

/// The text of `x`, written the same way a cell's value is written (the
/// `<v>` of [`write_cell`](crate::write_cell)): `0` for either zero, plain
/// decimal digits from 1e-7 up to 1e21, and otherwise a mantissa and an
/// exponent, `1.5E-8`, `1E+21` (upper-case `E`, the sign of a positive
/// exponent written).
///
/// ```
/// use xlsx_graft_cell::number_text;
///
/// assert_eq!(number_text(12.5)?, "12.5");
/// assert_eq!(number_text(-0.0)?, "0");
/// assert_eq!(number_text(1e21)?, "1E+21");
/// assert!(number_text(f64::NAN).is_err());
/// # Ok::<(), xlsx_graft_cell::CellError>(())
/// ```
///
/// # Errors
///
/// [`CellError::NotFinite`] for NaN and the infinities.
pub fn number(x: f64) -> Result<String, CellError> {
    if !x.is_finite() {
        return Err(CellError::NotFinite);
    }
    if x == 0.0 {
        return Ok("0".to_owned());
    }
    if (1e-7..1e21).contains(&x.abs()) {
        return Ok(x.to_string());
    }
    // `{:e}` writes `1e21` and `1.5e-8`: `e` is replaced by `E+`, then the
    // `E+-` of a negative exponent is replaced by `E-`.
    Ok(format!("{x:e}").replacen('e', "E+", 1).replace("E+-", "E-"))
}

/// The text of `n`.
///
/// # Errors
///
/// [`CellError::InexactInteger`] when its magnitude is past 2^53.
pub(crate) fn integer(n: i64) -> Result<String, CellError> {
    if n.unsigned_abs() > EXACT {
        return Err(CellError::InexactInteger);
    }
    Ok(n.to_string())
}

#[cfg(test)]
mod tests;
