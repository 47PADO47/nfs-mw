//! Audit all installed AI collision links, including goal-specific police profiles.

use blackbox_attrib::{Database, Value};
use nfsmw_data::car::physics::{AiCollisionReactions, CollisionReactions, GoalReaction};

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn every_ai_collision_profile_and_goal_reference_resolves() {
    let Some(dir) = crate::install() else { return };
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
    let mut profiles = 0;
    let mut defaults = 0;
    let mut goals = 0;
    let mut vehicles = 0;
    for c in db.collections_of("collisionreactions") {
        assert!(CollisionReactions::read(c).is_some(), "invalid profile {:08x}", c.key());
        profiles += 1;
    }
    for ai in db.collections_of("aivehicle") {
        if let Some(reference) = ai.get("PlayerCollisionsDefault").and_then(Value::as_ref_spec) {
            let profile = db.resolve(reference).expect("default profile reference must resolve");
            assert!(CollisionReactions::read(profile).is_some());
            defaults += 1;
        }
        let Some(items) = ai.get("PlayerCollisions").and_then(Value::as_array) else { continue };
        for item in items {
            let (_, reference) = GoalReaction::reference(item).expect("goal record must have the PC layout");
            let profile = db.resolve(reference).expect("goal profile reference must resolve");
            assert!(CollisionReactions::read(profile).is_some());
            goals += 1;
        }
    }
    for vehicle in db.collections_of("pvehicle") {
        if vehicle.follow("aivehicle").is_none() {
            continue;
        }
        let reactions = AiCollisionReactions::read(&db, vehicle).expect("vehicle profile links must parse");
        assert!(reactions.goals.iter().all(|goal| goal.reactions.is_some()));
        vehicles += 1;
    }
    assert!(profiles > 20 && defaults > 10 && goals > 10 && vehicles > 50);
    println!(
        "audited {profiles} collision profiles, {defaults} AI defaults, {goals} goal overrides, {vehicles} vehicles"
    );
}
