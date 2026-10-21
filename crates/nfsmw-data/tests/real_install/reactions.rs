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

fn reaction_values(r: &nfsmw_data::car::physics::CollisionReaction) -> [f32; 6] {
    [r.elasticity, r.roll_height, r.weight_bias, r.mass_scale, r.stun_speed, r.stun_time]
}

fn profile_values(profile: &CollisionReactions) -> [[f32; 6]; 4] {
    [
        reaction_values(&profile.front),
        reaction_values(&profile.front_side),
        reaction_values(&profile.rear),
        reaction_values(&profile.rear_side),
    ]
}

fn first_goal_reference(ai: blackbox_attrib::CollectionRef<'_>, goal: u32) -> Option<blackbox_attrib::RefSpec> {
    let count = match ai.get("PlayerCollisions") {
        Some(Value::Array(values)) => values.len(),
        Some(_) => 1,
        None => 0,
    };
    (0..count)
        .filter_map(|i| GoalReaction::reference(ai.get_at("PlayerCollisions", i)?))
        .find(|(key, _)| *key == goal)
        .map(|(_, reference)| reference)
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_cop_directional_reactions_cover_all_inherited_types_and_goals() {
    let Some(dir) = crate::install() else { return };
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
    let mut count = 0;
    for vehicle in db.collections_of("pvehicle").filter(|v| v.name().is_some_and(|name| name.starts_with("cop"))) {
        let Some(ai) = vehicle.follow("aivehicle") else { continue };
        let reactions = AiCollisionReactions::read(&db, vehicle).expect("installed police reaction links");
        let default_source = ai.follow("PlayerCollisionsDefault");
        let default = default_source.and_then(CollisionReactions::read);
        assert_eq!(reactions.default.as_deref().map(profile_values), default.as_ref().map(profile_values));
        print!(
            "COP_REACTION\t{}\tai={:?}\tmass={:?}\tdefault={:?}:{:?}",
            vehicle.name().unwrap(),
            ai.name(),
            vehicle.get_f32("MASS"),
            default_source.and_then(|v| v.name()),
            default.as_ref().map(profile_values)
        );
        for entry in &reactions.goals {
            let chosen = reactions.profile(Some(entry.goal));
            let reference = first_goal_reference(ai, entry.goal).expect("source goal record");
            let source = db.resolve(reference);
            let expected = source.and_then(CollisionReactions::read).or_else(|| default.clone());
            assert_eq!(chosen.map(|v| profile_values(v)), expected.as_ref().map(profile_values));
            print!(
                "\tgoal={:08X}/{:?}:{:?}",
                entry.goal,
                source.and_then(|v| v.name()),
                chosen.map(|v| profile_values(v))
            );
        }
        println!();
        for source in default_source.into_iter().chain(reactions.goals.iter().filter_map(|goal| {
            let reference = first_goal_reference(ai, goal.goal)?;
            db.resolve(reference)
        })) {
            let profile = CollisionReactions::read(source).unwrap();
            for (field, parsed) in ["FRONT_REACTION", "FRONTSIDE_REACTION", "REAR_REACTION", "REARSIDE_REACTION"]
                .into_iter()
                .zip(profile_values(&profile))
            {
                let Value::Raw { bytes, .. } = source.get(field).unwrap() else { panic!("source reaction layout") };
                let raw: Vec<_> = bytes.as_chunks::<4>().0.iter().map(|v| f32::from_le_bytes(*v)).collect();
                assert_eq!(
                    parsed.as_slice(),
                    raw.as_slice(),
                    "{}: {} {field}",
                    vehicle.name().unwrap(),
                    source.name().unwrap_or("?")
                );
            }
        }
        count += 1;
    }
    assert!(count >= 10, "expected shipped police variants, found {count}");
    println!("COP_REACTION_COUNT={count}");
}
