//! The lists of the styles part an edit adds to: the items of a list as read
//! and as added (canonical forms, each with the first index it has), and the
//! number formats, which are text and an id and not canonical forms.

use crate::canonical::Node;
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// How a list element is written in the part it was read from.
#[derive(Debug, Default)]
pub(super) struct Shape {
    /// Whether the part has the list element at all.
    pub(super) present: bool,
    /// The prefix of the list element's name, without the colon; empty when
    /// the name has none.
    pub(super) prefix: Vec<u8>,
    /// Whether the list element has a `count` attribute.
    pub(super) count: bool,
}

/// What it takes to add `node` to a [`List`]: the items to push, in order,
/// and the index `node` has once they are pushed.
#[derive(Debug)]
pub(super) struct Plan {
    /// The items to add, in order; none when `node` is in the list already.
    pub(super) adds: Vec<Node>,
    /// The index of `node`.
    pub(super) index: u32,
}

/// One of the lists whose items are canonical forms (`fonts`, `fills`,
/// `borders`, `cellXfs`): the items read, then those added, and for each
/// canonical form the first index it has.
#[derive(Debug, Default)]
pub(super) struct List {
    pub(super) shape: Shape,
    /// The items, in order: those of the part, then those added.
    pub(super) items: Vec<Node>,
    first: HashMap<Node, u32>,
    /// How many of the last `items` were added.
    added: usize,
}

impl List {
    /// Pushes `node`, an item of the part or one added, at the end.
    pub(super) fn push(&mut self, node: Node) {
        let index = index(self.items.len());
        self.first.entry(node.clone()).or_insert(index);
        self.items.push(node);
    }

    /// How `node` is added: its index when the list has it already, and
    /// otherwise the items to push, `reserved` that the list lacks first
    /// (the first `reserved.len()` items of the list are the reserved ones).
    /// The list is not changed.
    pub(super) fn plan(&self, node: Node, reserved: &[Node]) -> Plan {
        if let Some(&index) = self.first.get(&node) {
            return Plan {
                adds: Vec::new(),
                index,
            };
        }
        let mut adds: Vec<Node> = reserved.iter().skip(self.items.len()).cloned().collect();
        let at = (adds.iter().position(|add| *add == node)).unwrap_or_else(|| {
            adds.push(node);
            adds.len() - 1
        });
        Plan {
            index: index(self.items.len() + at),
            adds,
        }
    }

    /// Pushes the items of a [`Plan`].
    pub(super) fn add(&mut self, adds: Vec<Node>) {
        self.added += adds.len();
        for node in adds {
            self.push(node);
        }
    }

    /// The items added.
    pub(super) fn added(&self) -> &[Node] {
        &self.items[self.items.len() - self.added..]
    }
}

/// The explicit number formats (`numFmts`): the ids and codes the part
/// defines, and those added.
#[derive(Debug, Default)]
pub(super) struct NumberFormats {
    pub(super) shape: Shape,
    /// The ids the part defines, each with the code of its first definition.
    ids: BTreeMap<u32, String>,
    /// The ids the part defines more than once with different codes: what
    /// they mean is not known, so they are not given to any code.
    conflicting: BTreeSet<u32>,
    /// For each format code the first id the part defines it under.
    first: HashMap<String, u32>,
    /// How many `numFmt` elements the part has.
    pub(super) read: usize,
    /// The definitions added: id and code, in order.
    pub(super) added: Vec<(u32, String)>,
    /// The greatest `numFmtId` attribute of any element in the whole part.
    pub(super) max_id: u32,
}

impl NumberFormats {
    /// Records a `numFmt` of the part that has both its id and its code.
    /// An id defined again with another code is conflicting, and no code is
    /// found under it.
    pub(super) fn define(&mut self, id: u32, code: String) {
        match self.ids.get(&id) {
            None => {
                self.ids.insert(id, code.clone());
                self.first.entry(code).or_insert(id);
            }
            Some(held) if *held != code && self.conflicting.insert(id) => {
                // Only the code of the first definition can have `id` as
                // its first id.
                if self.first.get(held) == Some(&id) {
                    self.first.remove(held);
                }
            }
            Some(_) => {}
        }
    }

    /// Whether the part defines `id` as a number format.
    pub(super) fn defines(&self, id: u32) -> bool {
        self.ids.contains_key(&id)
    }

    /// The first id the part defines `code` under.
    pub(super) fn id_of(&self, code: &str) -> Option<u32> {
        self.first.get(code).copied()
    }

    /// How many explicit number formats there are: those of the part and
    /// those added.
    pub(super) fn count(&self) -> usize {
        self.read + self.added.len()
    }

    /// Adds the definition of `id` as `code`.
    pub(super) fn add(&mut self, id: u32, code: String) {
        self.define(id, code.clone());
        self.max_id = self.max_id.max(id);
        self.added.push((id, code));
    }
}

/// The `u32` index of the item at `at`.
fn index(at: usize) -> u32 {
    u32::try_from(at).unwrap_or(u32::MAX)
}
