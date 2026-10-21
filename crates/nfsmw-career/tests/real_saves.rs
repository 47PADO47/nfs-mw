//! Optional test against real profile files. It runs only when `NFSMW_SAVE_DIR` points at a directory that holds
//! profiles, either as the game stores them (`<dir>/<alias>/<alias>`, the `Documents\NFS Most Wanted` folder) or as
//! plain files. Without the variable the test returns at once. No save bytes are copied into the repository: the
//! expected values below are the ones `docs/formats/career-save.md` and `docs/specs/career-phone.md` state for the
//! three development saves, which are recognised by their folder name.

use nfsmw_career::phone::InboxTab;
use nfsmw_career::save::ProfileSave;
use nfsmw_career::save::layout::FILE_LEN;
use nfsmw_career::save::milestones_block::MilestoneBlock;
use nfsmw_career::settings::{Flags, NODE_LIST_LEN};
use std::path::{Path, PathBuf};

/// The profile files under `dir`: `<dir>/<alias>/<alias>` and files of the right size directly in `dir`.
fn profile_files(dir: &Path) -> Vec<(String, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut found = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let candidate = if path.is_dir() { path.join(&name) } else { path.clone() };
        let Ok(meta) = std::fs::metadata(&candidate) else { continue };
        if meta.is_file() && meta.len() == FILE_LEN as u64 {
            found.push((name, candidate));
        }
    }
    found.sort();
    found
}

#[test]
fn real_profiles_round_trip_and_match_the_documented_values() {
    let Some(dir) = std::env::var_os("NFSMW_SAVE_DIR") else { return };
    for (name, path) in profile_files(Path::new(&dir)) {
        let file = std::fs::read(&path).unwrap();
        let save = ProfileSave::parse(&file).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(save.integrity().all_ok(), "{name}: container checksums and MD5");
        assert_eq!(save.to_bytes(), file, "{name}: byte-exact round trip");

        let settings = save.settings().unwrap();
        let stable = save.stable().unwrap();
        let mut rewritten = ProfileSave::parse(&file).unwrap();
        rewritten.set_settings(&settings).unwrap();
        rewritten.set_stable(&stable);
        rewritten.set_history(&save.history().unwrap()).unwrap();
        assert_eq!(rewritten.to_bytes(), file, "{name}: parsed blocks written back");

        let report = nfsmw_career::savefile::check_load(&file, save.default_stable_hash())
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(report.blob, nfsmw_career::savefile::BlobStatus::Accepted, "{name}: the gameplay blob is taken");
        assert!(settings.gameplay_digest_ok(), "{name}: the gameplay blob MD5");
        assert!(settings.has_career(), "{name}: career started");
        assert!((1..=16).contains(&settings.stage), "{name}: stage {}", settings.stage);
        let valid = settings.sms.slots.iter().filter(|s| s.id != 0xFF).count();
        assert_eq!(valid, 95, "{name}: valid SMS slots");
        for record in stable.careers.iter().filter(|r| !r.is_unused()) {
            assert_eq!(record.strike_limit, 3, "{name}: every career record starts with limit 3");
        }

        let mut node_block = vec![0u8; NODE_LIST_LEN];
        settings.nodes().write(&mut node_block).unwrap();
        assert_eq!(node_block, settings.node_states[..], "{name}: the node list round trip");

        let known = matches!(name.as_str(), "NAME" | "mwo" | "stock");
        if !known {
            continue;
        }
        assert_eq!(settings.flags, Flags(0x001D_F7F3), "{name}: the flags word is the same in the three saves");
        let nodes = settings.nodes();
        let won_in_career = |id: &str| nodes.get(id).is_some_and(|state| state.won_in_career());
        match name.as_str() {
            "mwo" => {
                assert_eq!(nodes.len(), 256, "{name}: node records");
                assert!(won_in_career("16.1.0") && won_in_career("16.2.1"), "{name}: the prologue races are won");
                assert_eq!(settings.stage, 1);
                assert_eq!(settings.unknown_0x10, 30_372);
                assert_eq!(settings.sms.counter, 74);
                assert_eq!(settings.sms.slots.iter().filter(|s| s.state == 4).count(), 80);
                assert_eq!((stable.sold.bounty, stable.sold.evaded), (43_116_691, 111));
                let mut no_sequence: Vec<u32> = [InboxTab::Story, InboxTab::Tips]
                    .iter()
                    .flat_map(|&tab| settings.sms.list(tab))
                    .filter(|entry| entry.sequence == 0)
                    .map(|entry| entry.id)
                    .collect();
                no_sequence.sort_unstable();
                assert_eq!(no_sequence, [10, 11, 12, 13, 95, 96], "calls written without a sequence number");
            }
            "stock" => {
                assert!(!won_in_career("16.2.1"), "{name}: 16.2.1 is not won");
                assert_eq!(settings.stage, 15);
                assert_eq!((settings.sms.counter, stable.sold.bounty), (1, 0));
                let delivered: Vec<_> = settings.sms.slots.iter().filter(|s| s.state != 0).collect();
                assert_eq!(delivered.len(), 1);
                assert_eq!((delivered[0].id, delivered[0].state, delivered[0].sequence), (15, 2, 1));
            }
            _ => {
                assert_eq!(settings.stage, 15);
                assert_eq!((settings.sms.counter, stable.sold.bounty), (0, 0));
            }
        }
        let block = MilestoneBlock::from_bytes(&settings.gameplay_blob[..]).unwrap();
        let milestones = block.milestones().unwrap();
        let traps = block.speed_traps().unwrap();
        assert_eq!((milestones.len(), traps.len()), (55, 39), "{name}: milestone and speed trap counts");
        let expected_state = if name == "mwo" { 4 } else { 1 };
        assert!(milestones.iter().all(|m| m.state_byte == expected_state), "{name}: milestone states");
        println!(
            "{name}: ok (stage {}, cash {}, total bounty {})",
            settings.stage,
            settings.cash,
            stable.total_bounty()
        );
    }
}
