//! The sound graphs of a 2012 car bundle (kind `0x15` objects, docs/formats/chameleon-sound.md): which samples a graph
//! that reads a named live value plays.
//!
//! A graph is a tree of nodes whose expression strings read live values of the game (`...GetPopProbability`,
//! `...BackfireGain`). Its imports point at the samples it plays and at other graphs. This module only asks *which graphs,
//! named by the values they read, play a sample*; the nodes themselves (windows, fades and curves) are read by
//! `chameleon-sound-graph` for the engine (`graph_voices`) and are not read for the other effects.

use std::collections::{BTreeSet, HashMap, HashSet};

use chameleon_bundle::Bundle;

/// The resource kind of a sound graph.
const GRAPH_KIND: u32 = 0x15;
/// A graph that is only a bag of variants lists at most this many samples directly. The car's top graph, which lists
/// every sample the car has, is far above it.
const MAX_VARIANTS: usize = 12;

/// One graph: the live values its expressions read and what it imports.
struct Graph {
    names: Vec<String>,
    imports: Vec<u64>,
}

/// The graphs of a bundle.
pub struct Graphs {
    graphs: HashMap<u64, Graph>,
    /// The ids of the bundle's samples.
    samples: HashSet<u64>,
    /// The ids of every resource the bundle holds.
    held: HashSet<u64>,
}

impl Graphs {
    pub fn new(bundle: &Bundle) -> Self {
        let mut graphs = HashMap::new();
        let mut samples = HashSet::new();
        let held = bundle.entries().iter().map(|entry| entry.id).collect();
        for (at, entry) in bundle.entries().iter().enumerate() {
            if entry.kind == crate::sample::SAMPLE_KIND {
                samples.insert(entry.id);
            }
            if entry.kind != GRAPH_KIND {
                continue;
            }
            let Ok(resource) = bundle.resource(at) else { continue };
            let names = resource.blocks.first().map(|block| expression_names(block)).unwrap_or_default();
            let imports = resource.imports.iter().map(|import| import.id).collect();
            graphs.insert(entry.id, Graph { names, imports });
        }
        Self { graphs, samples, held }
    }

    /// The samples played by the graphs that read a value whose name contains `value`, directly or through the graphs
    /// they import.
    pub fn samples_reading(&self, value: &str) -> BTreeSet<u64> {
        let mut found = BTreeSet::new();
        let mut seen = HashSet::new();
        for (&id, graph) in &self.graphs {
            if graph.names.iter().any(|name| name.contains(value)) {
                self.collect(id, &mut seen, &mut found);
            }
        }
        found
    }

    /// `seeds` and the other variants of them: a graph with no expression of its own that lists a seed lists the sounds
    /// that are played in its place (the three backfires of a car are one such list).
    pub fn with_variants(&self, seeds: &BTreeSet<u64>) -> BTreeSet<u64> {
        let mut all = seeds.clone();
        for graph in self.graphs.values() {
            let listed: Vec<u64> = graph.imports.iter().copied().filter(|id| self.samples.contains(id)).collect();
            let is_bag = graph.names.is_empty() && listed.len() <= MAX_VARIANTS;
            if is_bag && listed.iter().any(|id| seeds.contains(id)) {
                all.extend(listed);
            }
        }
        all
    }

    /// The ids that graphs reading a value whose name contains `value` import and the bundle does not hold: samples
    /// the game keeps in a shared bundle.
    pub fn shared_imports_reading(&self, value: &str) -> Vec<u64> {
        let mut shared = BTreeSet::new();
        for graph in self.graphs.values().filter(|graph| graph.names.iter().any(|name| name.contains(value))) {
            shared.extend(graph.imports.iter().copied().filter(|id| !self.held.contains(id)));
        }
        shared.into_iter().collect()
    }

    fn collect(&self, id: u64, seen: &mut HashSet<u64>, found: &mut BTreeSet<u64>) {
        if !seen.insert(id) {
            return;
        }
        if self.samples.contains(&id) {
            found.insert(id);
            return;
        }
        let Some(graph) = self.graphs.get(&id) else { return };
        for &child in &graph.imports {
            self.collect(child, seen, found);
        }
    }
}

/// The expression names in a graph's main block: runs of printable ASCII with a dot, a letter and no space, such as
/// `RaceCars.EntityEngineSound[#].BackfireGain`.
fn expression_names(block: &[u8]) -> Vec<String> {
    let mut names = Vec::new();
    let mut run = String::new();
    for &byte in block.iter().chain(std::iter::once(&0)) {
        if (0x21..0x7f).contains(&byte) {
            run.push(byte as char);
            continue;
        }
        if is_expression(&run) {
            names.push(std::mem::take(&mut run));
        }
        run.clear();
    }
    names
}

fn is_expression(run: &str) -> bool {
    run.len() >= 6 && run.contains('.') && run.chars().any(|c| c.is_ascii_alphabetic())
}

#[cfg(test)]
mod tests;
