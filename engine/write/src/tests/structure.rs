//! Tests for the names a structure edit writes, driven through the public
//! `structure` module: `sheet_names` and `defined_names` cover what each
//! rule refuses and the order they are checked in, `prefix` the prefix a
//! sheet is written with, and `properties` that the prefix written for the
//! names accepted is read back by the formula reader. `model` covers what
//! `Workbook::read` reads of a workbook, and `model_unreadable` the
//! workbooks it gives `None` for, with the diagnostics each pushes.

mod defined_names;
mod model;
mod model_unreadable;
mod prefix;
mod properties;
mod sheet_names;
