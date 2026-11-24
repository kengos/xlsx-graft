//! `removals` on a long chain of parts: the work is linear and the chain is
//! removed whatever its length.

use crate::structure::edit::cleanup::removals;
use crate::structure::edit::snapshot::Node;
use xlsx_graft_package::PartName;

#[test]
fn a_chain_of_five_thousand_parts_is_removed_and_the_part_they_share_is_kept() {
    // Part 0 is the deleted sheet's, parts 1 to 5000 are a chain from it, each
    // also targeting the part 5001, which the part 5002 (not left out) targets
    // too.
    let chain = 5_000;
    let shared = chain + 1;
    let survivor = chain + 2;
    let name = |n: usize| PartName::parse(&format!("/xl/p{n}.xml")).unwrap();
    let mut graph: Vec<Node> = (0..=survivor)
        .map(|n| Node {
            name: name(n),
            targets: Vec::new(),
            incoming: Vec::new(),
        })
        .collect();
    let mut link = |from: usize, to: usize| {
        graph[from].targets.push(to);
        graph[to].incoming.push(Some(from));
    };
    for n in 0..chain {
        link(n, n + 1);
    }
    for n in 1..=chain {
        link(n, shared);
    }
    link(survivor, shared);
    let removed = removals(&graph, &[0], None);
    let expected: Vec<PartName> = (0..=chain).map(name).collect();
    assert_eq!(removed, expected.iter().collect::<Vec<_>>());
}
