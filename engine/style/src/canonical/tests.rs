//! Tests of the canonical form: what makes two items equal (`equality`), how
//! a style is stated on an item (`overlay`), how an item is written as XML
//! (`write`), the places of elements of another namespace and the helpers
//! the styles editor uses (`other_namespace`), and the spelling of the
//! enumerations (`tokens`). `reader` is the small XML reader the tests
//! build items with.

mod equality;
mod other_namespace;
mod overlay;
mod reader;
mod tokens;
mod write;

pub(super) use reader::node;
