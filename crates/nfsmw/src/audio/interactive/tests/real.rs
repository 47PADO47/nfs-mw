//! Against the user's own install: the pursuit start nodes, and a walk through each set at three control values.
//! Nothing here asserts what the original does; it checks that the sets can be entered and decoded, and prints
//! what a walk meets so a human can compare it with the spec.
//!
//! `NFSMW_GAME_DIR="D:/..." cargo test --release -p nfsmw interactive::tests::real -- --ignored --nocapture`

use std::collections::BTreeSet;

use ea_audio::mus::graph::{Advance, Cursor};
use game_install::GameDir;

use super::super::pursuit::PursuitFiles;
use crate::audio::radio::feeder::FileSource;

fn files() -> Option<PursuitFiles> {
    let dir = GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR")?)).expect("cannot index");
    let map = dir.read("SOUND/PFDATA/MW_Music.mpf").unwrap();
    let mus = dir.resolve("SOUND/PFDATA/MW_Music.mus").unwrap().to_path_buf();
    Some(PursuitFiles::new(&map, mus).unwrap())
}

#[test]
#[ignore = "needs NFSMW_GAME_DIR"]
fn every_pursuit_set_can_be_entered_and_its_first_bar_decodes() {
    let Some(files) = files() else { return };
    let source = FileSource::open(&files.mus).unwrap();
    for set in 1..=4u8 {
        let start = files.start_node(set).unwrap();
        assert_eq!(usize::from(files.graph.nodes[start].section), usize::from(set), "set {set} is section {set}");
        for control in [0u8, 64, 127] {
            let mut cursor = Cursor::new(&files.graph, start).unwrap();
            let (mut bars, mut streams, mut ended) = (0usize, BTreeSet::new(), None);
            for _ in 0..300 {
                match cursor.advance(&files.graph, control) {
                    Advance::Audio { stream, .. } => {
                        bars += 1;
                        streams.insert(stream);
                    }
                    Advance::End(end) => {
                        ended = Some(end);
                        break;
                    }
                }
            }
            println!(
                "set {set} (node {start:#X}) at control {control}: {bars} bars, {} distinct streams, end {ended:?}",
                streams.len()
            );
            assert!(bars > 0, "set {set} at control {control} plays nothing");
            let first = *streams.iter().next().unwrap() as usize;
            let pcm = files.mpf.decode(&source, first).unwrap();
            assert!(!pcm.samples.is_empty());
        }
    }
}
