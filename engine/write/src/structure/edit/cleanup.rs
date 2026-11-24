//! [`removals`]: the parts to leave out of the package when sheets are
//! deleted: the deleted sheets' parts, each part that only the parts left out
//! target through relationships, and the calcChain part.

use super::snapshot::Node;
use std::collections::BTreeSet;
use xlsx_graft_package::PartName;

/// The names of the parts to remove when the sheets whose parts are at
/// `roots` (indexes of `graph`) are deleted, in the order of `graph`:
///
/// - the roots;
/// - each part that a relationship of a part left out targets, when every
///   relationship that targets it is declared by a part left out (a
///   relationship of the package never is); parts that target each other stay
///   when a part that stays targets one of them, or when only the parts of the
///   cycle target them;
/// - the calcChain part at `calc_chain`, last, when there is a root and it is
///   not among those.
///
/// The work is one pass over the relationships: each part counts the
/// relationships that target it and are not declared by a part left out, and a
/// part with no such relationship left is left out in its turn.
pub(super) fn removals<'g>(
    graph: &'g [Node],
    roots: &[usize],
    calc_chain: Option<usize>,
) -> Vec<&'g PartName> {
    let mut pending: Vec<usize> = graph.iter().map(|node| node.incoming.len()).collect();
    let mut left: BTreeSet<usize> = roots.iter().copied().collect();
    let mut work: Vec<usize> = left.iter().copied().collect();
    while let Some(at) = work.pop() {
        let targets = graph
            .get(at)
            .into_iter()
            .flat_map(|node| node.targets.iter());
        for &target in targets.collect::<Vec<_>>() {
            for count in pending.get_mut(target).into_iter() {
                *count = count.saturating_sub(1);
                if *count == 0 && left.insert(target) {
                    work.push(target);
                }
            }
        }
    }
    let chain = calc_chain
        .filter(|chain| !roots.is_empty() && !left.contains(chain))
        .into_iter();
    left.iter()
        .copied()
        .chain(chain)
        .filter_map(|at| graph.get(at).map(|node| &node.name))
        .collect()
}
