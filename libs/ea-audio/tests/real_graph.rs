//! The music graph against the user's own install; ignored without `NFSMW_GAME_DIR`.
//!
//! Run with `NFSMW_GAME_DIR=... cargo test -p ea-audio --test real_graph -- --ignored --nocapture`.

use ea_audio::mus::graph::{Graph, NodeKind, WalkEnd};
use ea_audio::mus::{Chain, Mpf};
use std::path::PathBuf;

/// The 26 licensed songs, as the order of the game's song list gives them: low 24 bits of the song's event id,
/// the start node, the number of streams in the chain and its length in tenths of a second (the sum of the
/// stored stream durations, rounded). Ids, counts and lengths are facts about the file, not its content.
const SONGS: [(u32, usize, usize, u64); 26] = [
    (0x01eaf18b, 2771, 54, 2328),
    (0x01121590, 2625, 83, 2513),
    (0x015254da, 2176, 54, 2307),
    (0x019dab22, 2560, 58, 2552),
    (0x0159c841, 1561, 39, 1776),
    (0x010b97e1, 2830, 87, 2248),
    (0x010ad947, 1490, 66, 2043),
    (0x01f6c15d, 1605, 106, 2787),
    (0x018b365e, 2713, 51, 2053),
    (0x015c0092, 1797, 80, 2157),
    (0x01ba79c9, 3101, 69, 2450),
    (0x01ab73e9, 1999, 164, 2597),
    (0x013618d6, 2460, 50, 1850),
    (0x015c9f2e, 2515, 40, 2148),
    (0x01715c30, 3177, 87, 4288),
    (0x018cba15, 2376, 75, 2330),
    (0x0126ada3, 1882, 112, 2412),
    (0x01540e5c, 3036, 58, 1957),
    (0x01bea71d, 3410, 83, 2371),
    (0x01211dd5, 2922, 50, 2262),
    (0x01e686ea, 3273, 130, 3705),
    (0x01e10499, 1716, 76, 2028),
    (0x010ff61f, 3559, 57, 2143),
    (0x01cae6a5, 2237, 132, 3739),
    (0x01f1b18d, 2977, 50, 2058),
    (0x0106a402, 3498, 56, 2233),
];

fn load() -> Option<(Mpf, Graph)> {
    let dir = PathBuf::from(std::env::var_os("NFSMW_GAME_DIR")?).join("SOUND/PFDATA/MW_Music.mpf");
    let bytes = std::fs::read(dir).unwrap();
    Some((Mpf::parse(&bytes).unwrap(), Graph::parse(&bytes).unwrap()))
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_graph_has_the_counts_of_the_file() {
    let Some((mpf, graph)) = load() else { return };
    assert_eq!((graph.nodes.len(), graph.events.len(), graph.routers.len(), graph.variables.len()), (3681, 70, 123, 5));
    let count = |f: &dyn Fn(&NodeKind) -> bool| graph.nodes.iter().filter(|n| f(&n.kind)).count();
    assert_eq!(count(&|k| matches!(k, NodeKind::Audio { .. })), 3326);
    assert_eq!(count(&|k| *k == NodeKind::Head), 89);
    assert_eq!(count(&|k| *k == NodeKind::End), 89);
    assert_eq!(count(&|k| matches!(k, NodeKind::FireEvent { .. })), 177);
    let names: Vec<_> = graph.variables.iter().map(|v| v.name.as_str()).collect();
    assert_eq!(names, ["rapsheet", "pursuitid", "partnode", "newnode", "ambstate"]);
    // Every audio node names a stream that exists, and every stream is used.
    let mut used = vec![false; mpf.streams.len()];
    for node in &graph.nodes {
        if let NodeKind::Audio { stream } = node.kind {
            used[stream as usize] = true;
        }
    }
    assert!(used.iter().all(|&u| u));
    // Every fire-event node names an event of the file.
    for node in &graph.nodes {
        if let NodeKind::FireEvent { event } = node.kind {
            assert!(graph.event(event).is_some(), "event {event:06x}");
        }
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn all_26_songs_resolve_to_their_chains() {
    let Some((mpf, graph)) = load() else { return };
    for (n, &(event, start, streams, tenths)) in SONGS.iter().enumerate() {
        assert_eq!(graph.song_start(event), Some(start), "song {n}");
        let chain: Chain = mpf.song_chain(&graph, event, 0).unwrap();
        assert_eq!(chain.end, WalkEnd::EndNode, "song {n}");
        assert_eq!(chain.segments.len(), streams, "song {n}");
        let secs_x10 = (chain.total_ms() + 50) / 100;
        assert_eq!(secs_x10, tenths, "song {n}: {:.3} s", chain.total_secs());
        // The distinct streams are one gapless run of the music file (the last bars of many songs play the same
        // stream more than once, and a few songs go back a few streams near the end).
        let mut distinct: Vec<usize> = chain.segments.iter().map(|s| s.stream).collect();
        distinct.sort_unstable();
        distinct.dedup();
        assert!(distinct.windows(2).all(|w| w[1] == w[0] + 1), "song {n}: the streams have gaps");
        // The control value does not matter for a song, and no router is involved.
        for value in [0u8, 64, 127] {
            assert_eq!(mpf.chain(&graph, start, value).unwrap(), chain, "song {n} at {value}");
        }
        let walk = graph.walk(start, 0).unwrap();
        assert!(walk.nodes.iter().all(|&i| graph.nodes[i].router == 0), "song {n} uses a router");
        println!("song {n:2}: {streams:3} streams, {:.1} s", chain.total_secs());
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_songs_do_not_share_streams() {
    let Some((mpf, graph)) = load() else { return };
    let mut owner = vec![None; mpf.streams.len()];
    for (n, &(event, ..)) in SONGS.iter().enumerate() {
        for seg in mpf.song_chain(&graph, event, 0).unwrap().segments {
            let previous = owner[seg.stream].replace(n);
            assert!(
                previous.is_none() || previous == Some(n),
                "stream {} is in songs {previous:?} and {n}",
                seg.stream
            );
        }
    }
}
