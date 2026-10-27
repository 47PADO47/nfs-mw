use super::*;

fn graph(names: &[&str], imports: &[u64]) -> Graph {
    Graph { names: names.iter().map(|name| name.to_string()).collect(), imports: imports.to_vec() }
}

/// Graph 110 reads `X.BackfireGain` and imports sample 1; graph 111 is a bag of samples 1, 2 and 3 with no expression;
/// graph 112 reads `X.Other` through graph 113 and plays sample 4; graph 114, the root, lists every sample.
fn graphs() -> Graphs {
    let mut table = HashMap::new();
    table.insert(110, graph(&["X.BackfireGain"], &[1, 999]));
    table.insert(111, graph(&[], &[1, 2, 3, 998]));
    table.insert(112, graph(&["X.Other"], &[113]));
    table.insert(113, graph(&[], &[4]));
    table.insert(114, graph(&[], &(1..=40).collect::<Vec<u64>>()));
    let samples: HashSet<u64> = (1..=40).collect();
    let held = samples.iter().copied().chain([110, 111, 112, 113, 114, 998]).collect();
    Graphs { graphs: table, samples, held }
}

#[test]
fn the_samples_a_named_graph_plays_are_found_directly_and_through_the_graphs_it_imports() {
    let graphs = graphs();
    assert_eq!(graphs.samples_reading("BackfireGain"), BTreeSet::from([1]));
    assert_eq!(graphs.samples_reading("Other"), BTreeSet::from([4]));
    assert!(graphs.samples_reading("Missing").is_empty());
}

#[test]
fn a_bag_of_variants_that_lists_a_seed_adds_its_other_samples_and_the_top_graph_adds_nothing() {
    let graphs = graphs();
    let seeds = graphs.samples_reading("BackfireGain");
    assert_eq!(graphs.with_variants(&seeds), BTreeSet::from([1, 2, 3]));
    assert_eq!(graphs.with_variants(&BTreeSet::from([4])), BTreeSet::from([4]), "graph 113 lists only the seed");
}

#[test]
fn an_import_the_bundle_does_not_hold_is_a_shared_resource() {
    let graphs = graphs();
    assert_eq!(graphs.shared_imports_reading("BackfireGain"), [999]);
    assert!(graphs.shared_imports_reading("Other").is_empty());
}

#[test]
fn expression_names_are_picked_out_of_a_block() {
    let block = b"\0\0?333?333?333?\0RaceCars.EntityEngineSound[#].BackfireGain\0\x01\x02ab\0a.b c\0Players.LocalPlayer.GetPlayerGas";
    assert_eq!(
        expression_names(block),
        ["RaceCars.EntityEngineSound[#].BackfireGain", "Players.LocalPlayer.GetPlayerGas"]
    );
    assert!(expression_names(b"").is_empty());
}
