//! Finding the shared recordings in the 2012 install by the names the sound graphs carry
//! (docs/formats/chameleon-shared-sounds.md).
//!
//! A graph that a sound designer named holds its name as text in its main block (`Player Vehicle Collision Big`). The
//! graph's imports, in order, are what it plays: a selector node (one of the variants plays) followed by the samples it
//! chooses from, then the next selector.

use std::collections::BTreeSet;

use chameleon_bundle::{Bundle, Resource};

use crate::sample::{self, SAMPLE_KIND, Sample};

/// The resource kind of a sound graph.
const GRAPH_KIND: u32 = 0x15;

/// The graphs of `bundle` whose main block holds the name `label`.
pub(super) fn named_graphs(bundle: &Bundle, label: &str) -> Vec<Resource> {
    let mut found = Vec::new();
    for (at, entry) in bundle.entries().iter().enumerate() {
        if entry.kind != GRAPH_KIND {
            continue;
        }
        let Ok(resource) = bundle.resource(at) else { continue };
        if contains(&resource.blocks[0], label.as_bytes()) {
            found.push(resource);
        }
    }
    found
}

fn contains(block: &[u8], text: &[u8]) -> bool {
    !text.is_empty() && block.windows(text.len()).any(|window| window == text)
}

/// Whether the resource `id` of `bundle` is a sample.
pub(super) fn is_sample(bundle: &Bundle, id: u64) -> bool {
    bundle.find(id).is_some_and(|at| bundle.entries()[at].kind == SAMPLE_KIND)
}

/// The ids of the first run of consecutive samples among `imports` (`true` for a sample): the variants of the first
/// selector of a graph. Each id once, in ascending order.
pub(super) fn first_run(imports: &[(u64, bool)]) -> Vec<u64> {
    let run: BTreeSet<u64> =
        imports.iter().skip_while(|(_, sample)| !sample).take_while(|(_, sample)| *sample).map(|(id, _)| *id).collect();
    run.into_iter().collect()
}

/// The first selector's samples of every graph of `bundle` named `label`, each once.
pub(super) fn first_selector_samples(bundle: &Bundle, label: &str) -> Vec<u64> {
    let mut ids = BTreeSet::new();
    for graph in named_graphs(bundle, label) {
        let imports: Vec<(u64, bool)> =
            graph.imports.iter().map(|import| (import.id, is_sample(bundle, import.id))).collect();
        ids.extend(first_run(&imports));
    }
    ids.into_iter().collect()
}

/// The samples of `target` that the graphs of `graphs_in` named `label` import, each once.
pub(super) fn samples_imported(graphs_in: &Bundle, label: &str, target: &Bundle) -> Vec<u64> {
    let mut ids = BTreeSet::new();
    for graph in named_graphs(graphs_in, label) {
        ids.extend(graph.imports.iter().map(|import| import.id).filter(|&id| is_sample(target, id)));
    }
    ids.into_iter().collect()
}

/// The decoded samples `ids` of `bundle`; one that does not decode is left out.
pub(super) fn decode(bundle: &Bundle, ids: &[u64]) -> Vec<(u64, Sample)> {
    ids.iter().filter_map(|&id| Some((id, sample::read(bundle, bundle.find(id)?)?))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_found_inside_a_block_of_other_bytes() {
        let block = b"\x00\x01Player Vehicle Scraping\x00tail";
        assert!(contains(block, b"Player Vehicle Scraping"));
        assert!(!contains(block, b"Player Vehicle Collision Big"));
        assert!(!contains(b"", b"x") && !contains(block, b""));
    }

    #[test]
    fn the_first_selector_is_the_first_run_of_samples_and_nodes_end_it() {
        // node, node, samples 9 7 8, node, samples 1 2
        let imports = [(100, false), (101, false), (9, true), (7, true), (8, true), (102, false), (1, true), (2, true)];
        assert_eq!(first_run(&imports), [7, 8, 9]);
        assert_eq!(first_run(&[(100, false)]), Vec::<u64>::new());
        assert_eq!(first_run(&[(5, true), (5, true), (3, true)]), [3, 5], "each once");
    }
}
