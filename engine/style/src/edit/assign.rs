//! `StylesEdit::cell_format`: lays a [`Style`] over a base cell format and
//! gives the index of the `xf` that has the result, adding the `xf` and the
//! font, fill, border and number format it needs unless the part has them
//! already (`list.rs` finds an equal item by its canonical form).
//!
//! Nothing is changed unless the whole call succeeds: what to add is worked
//! out first, every limit is checked, and only then are the lists changed.

mod check;

use super::list::{List, Plan};
use super::scan::Scan;
use super::{StyleError, StyleKind, StyleLimits, limit};
use crate::canonical::overlay::{
    apply_alignment, apply_border, apply_font, apply_protection, fill_node,
};
use crate::canonical::{Node, Value};
use crate::number_format::builtin_id;
use crate::{Fill, PatternType, Style};

impl Scan {
    /// The index of the cell format that is `base` with `style` laid over
    /// it, adding what is missing.
    pub(super) fn cell_format(
        &mut self,
        limits: &StyleLimits,
        base: u32,
        style: &Style,
    ) -> Result<u32, StyleError> {
        check::check(style)?;
        let (mut xf, empty) = self.base_xf(base)?;
        let mut number = None;
        if let Some(code) = &style.number_format {
            let (id, new) = self.number_id(code, limits)?;
            number = new;
            put(&mut xf, "numFmtId", "applyNumberFormat", id);
        }
        let mut fonts = Vec::new();
        if let Some(font) = &style.font {
            let mut node = base_item(&self.fonts, &xf, "fontId", Node::font);
            apply_font(&mut node, font);
            let plan = self.fonts.plan(node, &[]);
            grow(StyleKind::Font, &self.fonts, &plan, limits.max_fonts)?;
            put(&mut xf, "fontId", "applyFont", plan.index);
            fonts = plan.adds;
        }
        let mut fills = Vec::new();
        if let Some(fill) = &style.fill {
            let plan = self.fills.plan(fill_node(fill), &reserved_fills());
            put(&mut xf, "fillId", "applyFill", plan.index);
            fills = plan.adds;
        }
        let mut borders = Vec::new();
        if let Some(border) = &style.border {
            let mut node = base_item(&self.borders, &xf, "borderId", Node::border);
            apply_border(&mut node, border);
            let plan = self.borders.plan(node, &[]);
            put(&mut xf, "borderId", "applyBorder", plan.index);
            borders = plan.adds;
        }
        if let Some(alignment) = &style.alignment {
            apply_alignment(&mut xf, alignment);
            xf.set_attribute("applyAlignment", Some(Value::Bool(true)));
        }
        if let Some(protection) = &style.protection {
            apply_protection(&mut xf, protection);
            xf.set_attribute("applyProtection", Some(Value::Bool(true)));
        }
        if empty && self.cell_style_xfs {
            xf.set_attribute("xfId", Some(Value::Uint(0)));
        }
        let plan = self.cell_xfs.plan(xf.finish(), &[]);
        let kind = StyleKind::CellFormat;
        grow(kind, &self.cell_xfs, &plan, limits.max_cell_formats)?;
        if let Some((id, code)) = number {
            self.numbers.add(id, code);
        }
        self.fonts.add(fonts);
        self.fills.add(fills);
        self.borders.add(borders);
        self.cell_xfs.add(plan.adds);
        Ok(plan.index)
    }

    /// The cell format `base` as a node, and whether it is a new, empty one:
    /// that is only for `base` 0 of a part that has no cell format.
    fn base_xf(&self, base: u32) -> Result<(Node, bool), StyleError> {
        let items = &self.cell_xfs.items;
        let found = usize::try_from(base).ok().and_then(|at| items.get(at));
        match found {
            Some(xf) => Ok((xf.clone(), false)),
            None if base == 0 && items.is_empty() => Ok((Node::xf(), true)),
            None => Err(StyleError::NoCellFormat { base }),
        }
    }

    /// The id the number format `code` has: a built-in id when `code` is
    /// one the table knows and the part does not define that id itself, else
    /// the first id the part defines `code` under, else a new one (given with
    /// the definition to add). The part is not changed.
    fn number_id(
        &self,
        code: &str,
        limits: &StyleLimits,
    ) -> Result<(u32, Option<(u32, String)>), StyleError> {
        let numbers = &self.numbers;
        if let Some(id) = builtin_id(code).filter(|&id| !numbers.defines(id)) {
            return Ok((id, None));
        }
        if let Some(id) = numbers.id_of(code) {
            return Ok((id, None));
        }
        let kind = StyleKind::NumberFormat;
        limit(kind, numbers.count() + 1, limits.max_number_formats)?;
        // 163: the ids up to 163 are reserved for the built-in formats.
        let id = numbers.max_id.max(163).checked_add(1);
        let limit = limits.max_number_formats;
        let id = id.ok_or(StyleError::TooMany { kind, limit })?;
        Ok((id, Some((id, code.to_owned()))))
    }
}

/// Sets the attribute `name` of `xf` to `id` and `apply` to true.
fn put(xf: &mut Node, name: &'static str, apply: &'static str, id: u32) {
    xf.set_attribute(name, Some(Value::Uint(id)));
    xf.set_attribute(apply, Some(Value::Bool(true)));
}

/// The item of `list` that the attribute `name` of `xf` refers to: the first
/// item when `xf` has no such attribute, as a cell format without one has.
/// `empty` when the attribute is not a number or `list` has no such item.
fn base_item(list: &List, xf: &Node, name: &str, empty: fn() -> Node) -> Node {
    let at = match xf.attribute(name) {
        None => Some(0),
        Some(Value::Uint(id)) => usize::try_from(*id).ok(),
        Some(_) => None,
    };
    let found = at.and_then(|at| list.items.get(at));
    found.cloned().unwrap_or_else(empty)
}

/// `TooMany` when `plan` adds to `list` and the list would be longer than
/// `max`.
fn grow(kind: StyleKind, list: &List, plan: &Plan, max: u32) -> Result<(), StyleError> {
    if plan.adds.is_empty() {
        return Ok(());
    }
    limit(kind, list.items.len() + plan.adds.len(), max)
}

/// The two fills Excel reserves as the first two of `fills`: none and
/// gray125, of the `patternFill` (ECMA-376-1:2016 §18.8.32).
fn reserved_fills() -> [Node; 2] {
    [PatternType::None, PatternType::Gray125].map(|pattern| {
        fill_node(&Fill::Pattern {
            pattern,
            foreground: None,
            background: None,
        })
    })
}
