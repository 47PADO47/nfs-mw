//! Original event packages are read locally; no event bytecode is embedded here.

use blackbox_attrib::vlt_hash;
use blackbox_sequence::{Player, QueueMode};
use nfsmw_data::world::sequences::{SequenceCatalog, TAGS, WorldEvent};

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn all_world_sequences_parse_and_log_damage_releases_the_original_parts_once() {
    let Some(dir) = crate::install() else { return };
    let catalog = SequenceCatalog::read(&crate::unwrapped(&dir, "TRACKS/L2RA.BUN")).unwrap();
    assert_eq!(catalog.len(), 75);
    let sequence = catalog.get(vlt_hash("TrailerLog")).unwrap();
    let p = &sequence.package;
    let mut player = Player::new(p, sequence.engine, TAGS).unwrap();
    assert_eq!(p.engines[sequence.engine].systems.len(), 7);
    assert_eq!(p.actions.len(), 3);
    let impact = vlt_hash("IMPACT_1");
    assert!(player.stimulate(p, Some(vlt_hash("DZ_LEFT")), impact, QueueMode::Allow, None).unwrap());
    let mut fragments = Vec::new();
    let mut explosions = Vec::new();
    for _ in 0..240 {
        let mut budget = 100;
        while let Some(firing) = player.pop() {
            budget -= 1;
            assert!(budget > 0, "event dispatch loop");
            for event in &p.event_lists[&firing.list] {
                match WorldEvent::decode(event).unwrap() {
                    WorldEvent::Fragment(fragment) => fragments.push(fragment),
                    WorldEvent::Explosion(explosion) => explosions.push(explosion),
                    WorldEvent::Stimulus { system, stimulus, .. } => {
                        player.stimulate(p, Some(system), stimulus, QueueMode::Allow, None).unwrap();
                    }
                    WorldEvent::Other(_) => {}
                }
            }
        }
        player.advance(p, 1.0 / 60.0).unwrap();
    }
    assert_eq!(fragments.len(), 9);
    assert_eq!(explosions.len(), 1);
    let existing: Vec<_> = fragments.iter().filter(|f| f.inherit_velocity == 0.3).collect();
    assert_eq!(existing.len(), 7);
    assert!(existing.iter().all(|f| f.hide_part && !f.hide_parent && !f.no_physics && !f.simple_physics));
    for n in 1..=7 {
        assert!(existing.iter().any(|f| f.part == vlt_hash(&format!("ATTACHMENT{n}"))));
    }
    assert_eq!(player.state(vlt_hash("Cargo")), Some(vlt_hash("Cargo_Empty")));
    // A second impact cannot create a second load after the empty-state transition.
    player.stimulate(p, Some(vlt_hash("DZ_RIGHT")), impact, QueueMode::Allow, None).unwrap();
    while let Some(firing) = player.pop() {
        for event in &p.event_lists[&firing.list] {
            if let WorldEvent::Stimulus { system, stimulus, .. } = WorldEvent::decode(event).unwrap() {
                assert!(!player.stimulate(p, Some(system), stimulus, QueueMode::Allow, None).unwrap());
            }
            assert!(!matches!(WorldEvent::decode(event).unwrap(), WorldEvent::Fragment(_)));
        }
    }
}
