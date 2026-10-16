//! Dependency and load-order resolution (ADR 0005 section 3).
//!
//! Three steps, repeated until nothing changes:
//!
//! 1. a mod whose dependency is missing, rejected, or in a version its requirement does not accept is rejected;
//! 2. every mod in a dependency or load-order cycle is rejected, with the members of the cycle as the reason.
//!
//! What is left is sorted topologically. Edges are `dependencies` and `load.after`/`load.before`; ties break by id,
//! ascending, so the same set of mods always loads in the same order.

use std::collections::{BTreeMap, BTreeSet};

use super::discovery::{Candidate, Discovery};
use super::reject::{RejectReason, Rejected};

/// The mods that will load, in load order, and the ones refused while ordering.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Resolution {
    /// Mods in load order. Each one's dependencies come before it.
    pub ordered: Vec<Candidate>,
    /// Everything refused, by discovery or here, in the order it was refused.
    pub rejected: Vec<Rejected>,
}

/// Order the discovered mods. The rejects from discovery are carried through unchanged.
pub fn resolve(discovery: Discovery) -> Resolution {
    let mut rejected = discovery.rejected;
    let mut alive: BTreeMap<String, Candidate> = BTreeMap::new();
    for candidate in discovery.candidates {
        let id = candidate.manifest.id.clone();
        if let Some(previous) = alive.insert(id, candidate) {
            rejected.push(Rejected::of_candidate(
                previous,
                RejectReason::Duplicate { kept: String::from("a later folder") },
            ));
        }
    }
    loop {
        if drop_unsatisfied(&mut alive, &mut rejected) {
            continue;
        }
        if drop_cycles(&mut alive, &mut rejected) {
            continue;
        }
        break;
    }
    let graph = load_graph(&alive);
    let mut ordered = Vec::with_capacity(alive.len());
    for id in topological_order(&graph) {
        if let Some(candidate) = alive.remove(&id) {
            ordered.push(candidate);
        }
    }
    Resolution { ordered, rejected }
}

/// Reject, in one pass, every mod with an unsatisfied dependency. Returns whether anything was rejected.
fn drop_unsatisfied(alive: &mut BTreeMap<String, Candidate>, rejected: &mut Vec<Rejected>) -> bool {
    let rejected_ids: BTreeSet<String> = rejected.iter().filter_map(|r| r.id.clone()).collect();
    let mut doomed: Vec<(String, RejectReason)> = Vec::new();
    for (id, candidate) in alive.iter() {
        for dependency in &candidate.manifest.dependencies {
            let reason = match alive.get(&dependency.id) {
                Some(found) if dependency.requirement.matches(&found.manifest.version) => continue,
                Some(found) => RejectReason::DependencyVersion {
                    id: dependency.id.clone(),
                    requirement: dependency.requirement.to_string(),
                    found: found.manifest.version.to_string(),
                },
                None if rejected_ids.contains(&dependency.id) => {
                    RejectReason::DependencyRejected { id: dependency.id.clone() }
                }
                None => RejectReason::DependencyMissing { id: dependency.id.clone() },
            };
            doomed.push((id.clone(), reason));
            break;
        }
    }
    if doomed.is_empty() {
        return false;
    }
    for (id, reason) in doomed {
        if let Some(candidate) = alive.remove(&id) {
            rejected.push(Rejected::of_candidate(candidate, reason));
        }
    }
    true
}

/// Reject every mod that sits in a cycle of more than one mod. Returns whether anything was rejected.
fn drop_cycles(alive: &mut BTreeMap<String, Candidate>, rejected: &mut Vec<Rejected>) -> bool {
    let graph = load_graph(alive);
    let cycles: Vec<Vec<String>> = strongly_connected(&graph).into_iter().filter(|members| members.len() > 1).collect();
    if cycles.is_empty() {
        return false;
    }
    for members in cycles {
        for id in &members {
            if let Some(candidate) = alive.remove(id) {
                rejected.push(Rejected::of_candidate(candidate, RejectReason::Cycle { members: members.clone() }));
            }
        }
    }
    true
}

/// The edges of the load order: `graph[a]` holds every mod that must load after `a`. Only mods still alive count;
/// a hint that names an absent mod is ignored, a dependency on one was already rejected.
fn load_graph(alive: &BTreeMap<String, Candidate>) -> BTreeMap<String, BTreeSet<String>> {
    let mut graph: BTreeMap<String, BTreeSet<String>> = alive.keys().map(|id| (id.clone(), BTreeSet::new())).collect();
    for (id, candidate) in alive {
        for dependency in &candidate.manifest.dependencies {
            if alive.contains_key(&dependency.id) {
                edge(&mut graph, &dependency.id, id);
            }
        }
        for before_this in &candidate.manifest.load_after {
            if alive.contains_key(before_this) {
                edge(&mut graph, before_this, id);
            }
        }
        for after_this in &candidate.manifest.load_before {
            if alive.contains_key(after_this) {
                edge(&mut graph, id, after_this);
            }
        }
    }
    graph
}

fn edge(graph: &mut BTreeMap<String, BTreeSet<String>>, first: &str, second: &str) {
    if let Some(successors) = graph.get_mut(first) {
        successors.insert(second.to_owned());
    }
}

/// Kahn's algorithm with the ready set kept in id order. Only called on a graph without cycles.
fn topological_order(graph: &BTreeMap<String, BTreeSet<String>>) -> Vec<String> {
    let mut indegree: BTreeMap<&str, usize> = graph.keys().map(|id| (id.as_str(), 0)).collect();
    for successors in graph.values() {
        for successor in successors {
            if let Some(count) = indegree.get_mut(successor.as_str()) {
                *count += 1;
            }
        }
    }
    let mut ready: BTreeSet<&str> = indegree.iter().filter(|(_, count)| **count == 0).map(|(id, _)| *id).collect();
    let mut order = Vec::with_capacity(graph.len());
    while let Some(id) = ready.pop_first() {
        order.push(id.to_owned());
        let Some(successors) = graph.get(id) else {
            continue;
        };
        for successor in successors {
            let Some(count) = indegree.get_mut(successor.as_str()) else {
                continue;
            };
            *count -= 1;
            if *count == 0 {
                ready.insert(successor.as_str());
            }
        }
    }
    order
}

/// Tarjan's algorithm. Each component comes back sorted; components come back in no particular order.
fn strongly_connected(graph: &BTreeMap<String, BTreeSet<String>>) -> Vec<Vec<String>> {
    let mut tarjan = Tarjan {
        graph,
        next_index: 0,
        index: BTreeMap::new(),
        low: BTreeMap::new(),
        stack: Vec::new(),
        on_stack: BTreeSet::new(),
        components: Vec::new(),
    };
    for node in graph.keys() {
        if !tarjan.index.contains_key(node) {
            tarjan.visit(node);
        }
    }
    tarjan.components
}

struct Tarjan<'g> {
    graph: &'g BTreeMap<String, BTreeSet<String>>,
    next_index: usize,
    index: BTreeMap<String, usize>,
    low: BTreeMap<String, usize>,
    stack: Vec<String>,
    on_stack: BTreeSet<String>,
    components: Vec<Vec<String>>,
}

impl Tarjan<'_> {
    fn visit(&mut self, node: &str) {
        self.index.insert(node.to_owned(), self.next_index);
        self.low.insert(node.to_owned(), self.next_index);
        self.next_index += 1;
        self.stack.push(node.to_owned());
        self.on_stack.insert(node.to_owned());

        let successors: Vec<String> = self.graph.get(node).map(|set| set.iter().cloned().collect()).unwrap_or_default();
        for next in successors {
            if !self.index.contains_key(&next) {
                self.visit(&next);
                let lowest = self.low[&next];
                self.lower(node, lowest);
                continue;
            }
            if self.on_stack.contains(&next) {
                let target = self.index[&next];
                self.lower(node, target);
            }
        }

        if self.low[node] != self.index[node] {
            return;
        }
        let mut component = Vec::new();
        while let Some(member) = self.stack.pop() {
            self.on_stack.remove(&member);
            let done = member == node;
            component.push(member);
            if done {
                break;
            }
        }
        component.sort();
        self.components.push(component);
    }

    fn lower(&mut self, node: &str, candidate: usize) {
        let current = self.low[node];
        if candidate < current {
            self.low.insert(node.to_owned(), candidate);
        }
    }
}

#[cfg(test)]
mod tests;
