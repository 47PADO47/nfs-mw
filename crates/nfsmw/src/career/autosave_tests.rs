use super::*;
use nfsmw_career::savefile::AutosaveManager;

fn inputs(node: Option<&str>, boss_race: bool, context: FrontEndContext) -> AutosaveInputs {
    AutosaveInputs { context, node_id: node.map(str::to_owned), boss_race, gameplay_idle: true, profile_loaded: true }
}

#[test]
fn no_node_in_career_context_passes_the_gate() {
    let facts = inputs(None, false, FrontEndContext::CAREER);
    let gate = facts.gate();
    assert!(gate.layer_present);
    assert!(!gate.force);
    assert_eq!(gate.current_node, None);
    let mut manager = AutosaveManager::default();
    assert!(manager.can_autosave(&gate));
}

#[test]
fn node_id_and_boss_flag_reach_the_gate() {
    let facts = inputs(Some("14.2.1"), true, FrontEndContext::CAREER);
    let gate = facts.gate();
    assert_eq!(gate.current_node, Some(CurrentNode { id: "14.2.1", boss_race: true }));
}

#[test]
fn online_context_blocks_the_gate() {
    let online = FrontEndContext(FrontEndContext::CAREER.0 | FrontEndContext::ONLINE.0);
    let facts = inputs(None, false, online);
    let mut manager = AutosaveManager::default();
    assert!(!manager.can_autosave(&facts.gate()));
}

#[test]
fn final_event_and_boss_race_block_the_gate() {
    let final_event = inputs(Some("1.8.1"), false, FrontEndContext::CAREER);
    let boss = inputs(Some("14.2.1"), true, FrontEndContext::CAREER);
    let mut manager = AutosaveManager::default();
    assert!(!manager.can_autosave(&final_event.gate()));
    assert!(!manager.can_autosave(&boss.gate()));
}
