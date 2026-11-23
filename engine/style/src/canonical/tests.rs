//! Tests of the canonical form: what makes two items equal (`equality`), how
//! a style is stated on an item (`overlay`), how an item is written as XML
//! (`write`), and the spelling of the enumerations (`tokens`). `reader` is
//! the small XML reader the tests build items with.

mod equality;
mod overlay;
mod reader;
mod tokens;
mod write;

pub(super) use reader::node;
