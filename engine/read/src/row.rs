//! [`Row`]: one row of a sheet as [`SheetRows`](crate::SheetRows) reads
//! it, in a buffer the caller keeps and hands back for the next row.

use crate::Value;

/// One row of a sheet: its number and the value of each column from A to
/// the last cell written in it, a column with no cell written being
/// [`Value::Empty`].
///
/// It is a buffer: [`SheetRows::next_row`](crate::SheetRows::next_row)
/// fills it in place of the row it held, so reading a sheet allocates no
/// more than its longest row takes. The text of every string and error
/// value of the row is one run of text the values point into.
#[derive(Debug, Clone, Default)]
pub struct Row {
    number: u32,
    /// The text of the row's strings and error values, one after another.
    text: String,
    /// Each column's value, from A.
    cells: Vec<Slot>,
}

/// One column's value, its text, if any, in [`Row::text`].
#[derive(Debug, Clone, Copy)]
pub(crate) enum Slot {
    /// A value with no text.
    Value(Value<'static>),
    /// A string: `text[start..end]`.
    String(usize, usize),
    /// An error value: `text[start..end]`.
    Error(usize, usize),
}

impl Row {
    /// An empty row, to read rows into.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The row's number, from 1.
    #[must_use]
    pub fn number(&self) -> u32 {
        self.number
    }

    /// How many columns it has values for: up to its last cell written.
    #[must_use]
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// Whether no cell of it is written.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// The value of the column at `index`, from 0 (A); `None` past its last
    /// cell written.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<Value<'_>> {
        self.cells.get(index).map(|slot| self.value(*slot))
    }

    /// The value of each column, from A to its last cell written.
    pub fn values(&self) -> impl Iterator<Item = Value<'_>> {
        self.cells.iter().map(|slot| self.value(*slot))
    }

    /// The value `slot` stands for.
    fn value(&self, slot: Slot) -> Value<'_> {
        let text = |start: usize, end: usize| self.text.get(start..end).unwrap_or_default();
        match slot {
            Slot::Value(value) => value,
            Slot::String(start, end) => Value::String(text(start, end)),
            Slot::Error(start, end) => Value::Error(text(start, end)),
        }
    }

    /// Empties the row to be row `number`.
    pub(crate) fn begin(&mut self, number: u32) {
        self.number = number;
        self.text.clear();
        self.cells.clear();
    }

    /// The text the row's values point into, to add a cell's text to.
    pub(crate) fn text(&mut self) -> &mut String {
        &mut self.text
    }

    /// Sets the value of `column`, from 1, past the last column set: the
    /// columns between are empty.
    pub(crate) fn set(&mut self, column: u16, slot: Slot) {
        let before = usize::from(column).saturating_sub(1);
        self.cells.resize(before, Slot::Value(Value::Empty));
        self.cells.push(slot);
    }
}
