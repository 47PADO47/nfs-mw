//! The decoder loop of a pursuit track: it follows the graph with the control value of each moment.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::{SyncSender, sync_channel};

use ea_audio::mus::graph::{Cursor, Graph, Node, NodeKind, Transition};

use super::super::live::feed;
use crate::audio::radio::stream::Block;

fn node(kind: NodeKind, transitions: &[(i8, i8, i16)]) -> Node {
    Node {
        kind,
        controller: 0,
        section: 1,
        repeat: 0,
        router: 0,
        random: 0,
        beats: 4,
        bars: 1,
        part: 0,
        transitions: transitions.iter().map(|&(lo, hi, target)| Transition { lo, hi, target }).collect(),
    }
}

/// Node 0 is the head; 1 plays stream 10 then 2; 2 plays stream 11 and goes back to 1 while the control value is
/// calm (below 64) or on to 3 when it is high; 3 plays stream 12 and goes back to 1; 4 is a dead end.
fn pursuit() -> Graph {
    let audio = |stream| NodeKind::Audio { stream };
    Graph {
        project: 0,
        sections: 7,
        nodes: vec![
            node(NodeKind::Head, &[(0, 127, 1)]),
            node(audio(10), &[(0, 127, 2)]),
            node(audio(11), &[(0, 63, 1), (64, 127, 3)]),
            node(audio(12), &[(0, 127, 1)]),
            node(audio(13), &[]),
        ],
        events: vec![],
        routers: vec![],
        variables: vec![],
    }
}

/// Run the loop from the head with a `play` that records the streams and lets `each` react to every one, until
/// `limit` streams have been asked for.
fn streams(
    graph: &Graph,
    start: usize,
    control: &AtomicU8,
    limit: usize,
    mut each: impl FnMut(usize, u32),
) -> Vec<u32> {
    let mut cursor = Cursor::new(graph, start).unwrap();
    let first = match cursor.advance(graph, control.load(Ordering::Relaxed)) {
        ea_audio::mus::graph::Advance::Audio { stream, .. } => stream,
        other => panic!("{other:?}"),
    };
    let (tx, _rx) = sync_channel::<Block>(1);
    let mut played = Vec::new();
    feed(graph, cursor, first, control, &tx, &mut |stream, _tx: &SyncSender<Block>| {
        played.push(stream);
        each(played.len(), stream);
        Ok(played.len() < limit)
    });
    played
}

#[test]
fn a_calm_track_loops_between_two_bars() {
    let control = AtomicU8::new(0);
    let played = streams(&pursuit(), 0, &control, 6, |_, _| {});
    assert_eq!(played, [10, 11, 10, 11, 10, 11]);
}

#[test]
fn the_control_value_changes_the_next_bar_while_playing() {
    let control = AtomicU8::new(0);
    // The game raises the intensity while the third bar plays.
    let played = streams(&pursuit(), 0, &control, 7, |count, _| {
        if count == 3 {
            control.store(100, Ordering::Relaxed);
        }
    });
    // Bar 11 (after the change) goes on to the intense bar 12, and back.
    assert_eq!(played, [10, 11, 10, 11, 12, 10, 11]);
}

#[test]
fn a_track_that_reaches_a_dead_end_stops() {
    let control = AtomicU8::new(0);
    let mut graph = pursuit();
    graph.nodes[3] = node(NodeKind::Audio { stream: 12 }, &[(0, 127, 4)]);
    control.store(100, Ordering::Relaxed);
    let played = streams(&graph, 0, &control, 50, |_, _| {});
    assert_eq!(played, [10, 11, 12, 13]);
}

#[test]
fn a_failing_stream_stops_the_loop() {
    let graph = pursuit();
    let mut cursor = Cursor::new(&graph, 0).unwrap();
    let ea_audio::mus::graph::Advance::Audio { stream, .. } = cursor.advance(&graph, 0) else { panic!() };
    let (tx, _rx) = sync_channel::<Block>(1);
    let mut asked = 0;
    feed(&graph, cursor, stream, &AtomicU8::new(0), &tx, &mut |_, _| {
        asked += 1;
        Err("the file is gone".into())
    });
    assert_eq!(asked, 1);
}
