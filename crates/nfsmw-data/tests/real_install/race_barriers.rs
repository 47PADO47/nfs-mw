//! The scenery groups and override records of `L2RA.BUN` against `docs/specs/race-barriers.md`: every name a race's
//! `Barriers` list gives has a group, the override records name the instances the section hooks name, and the race
//! walls are hidden, turnable `XO_TrackBarrierPlayer_1` and `XO_TrackBarrierTraffic_*` pieces.

use std::collections::{BTreeMap, HashMap, HashSet};

use blackbox_scenery::{INSTANCE_TURNABLE, OVERRIDE_HIDDEN, find_group};
use nfsmw_data::world::{DEFAULT_TRACK, WorldIndex, load_section};

use crate::install;

/// What the stream says about one override index: section number, model name and the instance's flag word.
struct Hooked {
    section: i16,
    instance: u16,
    model: String,
    flags: u32,
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR); reads the whole 533 MB stream"]
fn race_barrier_groups_resolve_to_hidden_wall_pieces() {
    let Some(dir) = install() else { return };
    let data = nfsmw_data::gameplay::load(&dir).expect("gameplay data");
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    assert_eq!(index.scenery_groups.len(), 754);
    assert_eq!(index.scenery_overrides.len(), 22_340);

    // Every name a race lists has a group (the star removed).
    let listing: Vec<_> = data.races.iter().filter(|r| !r.barriers.is_empty()).collect();
    assert_eq!(listing.len(), 230, "races with a Barriers list");
    let names: HashSet<&str> =
        listing.iter().flat_map(|r| r.barriers.iter()).map(|e| e.trim_start_matches('*')).collect();
    assert_eq!(names.len(), 736);
    for name in &names {
        assert!(find_group(&index.scenery_groups, blackbox_hash::bstring_hash(name)).is_some(), "{name} has no group");
    }

    let mut file = dir.open_file(&index.stream_file).unwrap();
    let mut hooked: HashMap<u16, Hooked> = HashMap::new();
    let mut collision_groups = HashSet::new();
    for (i, section) in index.sections.iter().enumerate() {
        let sd = load_section(&mut file, i, section).unwrap();
        for sc in &sd.scenery {
            for h in &sc.override_hooks {
                let inst = &sc.instances[usize::from(h.instance_index)];
                let model = sc.info_of(inst).map(|i| i.name.clone()).unwrap_or_default();
                let entry =
                    Hooked { section: section.number, instance: h.instance_index, model, flags: inst.exclude_flags };
                hooked.insert(h.override_index, entry);
            }
        }
        collision_groups.extend(sd.collision.iter().flat_map(|p| p.instances.iter().map(|i| i.group)));
    }

    // The override record of a hook names the same instance and stores the instance's own low flag bits.
    assert_eq!(hooked.len(), index.scenery_overrides.len(), "every override index is hooked once");
    let mut deltas: BTreeMap<i16, usize> = BTreeMap::new();
    for (&o, h) in &hooked {
        let record = index.scenery_overrides[usize::from(o)];
        *deltas.entry(h.section - record.section).or_default() += 1;
        assert_eq!(record.instance, h.instance as i16, "override {o}");
        assert_eq!(u32::from(record.flags), h.flags & 0xFFFF, "override {o} flags");
    }
    // The record stores the drivable section's number; the instance is in that section or in its far tile (n + 40).
    assert_eq!(deltas.keys().copied().collect::<Vec<_>>(), [0, 40], "{deltas:?}");

    // The race walls: what the barrier groups (all but the world-wide group 1) hold.
    let mut models: BTreeMap<(&str, u32), usize> = BTreeMap::new();
    for g in index.scenery_groups.iter().filter(|g| g.number != 1) {
        for o in &g.override_indexes {
            let h = &hooked[o];
            *models.entry((h.model.as_str(), h.flags)).or_default() += 1;
        }
    }
    eprintln!("barrier group pieces (model, flags): {models:?}");
    assert_eq!(models[&("XO_TrackBarrierPlayer_1", 0x0280_8070)], 5_771);
    assert_eq!(models[&("XO_TrackBarrierTraffic_", 0x0280_8070)], 1_151, "the name is cut at 23 characters");
    assert_eq!(models[&("XO_TrackBarrierLocked_1", 0x0200_8070)], 32);
    for (model, flags) in models.keys() {
        assert!(flags & u32::from(OVERRIDE_HIDDEN) != 0, "{model} is hidden until its group is switched on");
    }

    // The door: junk piles and a digger, none of which the star mirrors.
    let door = find_group(&index.scenery_groups, blackbox_hash::bstring_hash("SCENERY_GROUP_DOOR")).unwrap();
    assert_eq!((door.number, door.override_indexes.len(), door.drive_through), (71, 5, false));
    assert!(door.override_indexes.iter().all(|o| hooked[o].flags & INSTANCE_TURNABLE == 0));

    // Drive-through groups and collision.
    let drive_through = index.scenery_groups.iter().filter(|g| g.drive_through).count();
    assert_eq!(drive_through, 126);
    let mut through_models: BTreeMap<(bool, &str), usize> = BTreeMap::new();
    for g in index.scenery_groups.iter().filter(|g| g.number != 1) {
        for o in &g.override_indexes {
            *through_models.entry((g.drive_through, hooked[o].model.as_str())).or_default() += 1;
        }
    }
    // The drive-through groups are exactly the traffic barrier groups.
    assert_eq!(through_models.get(&(true, "XO_TrackBarrierTraffic_")), Some(&1_151), "{through_models:?}");
    assert_eq!(through_models.keys().filter(|(through, _)| *through).count(), 1, "{through_models:?}");
    let without: Vec<u16> =
        index.scenery_groups.iter().map(|g| g.number).filter(|n| !collision_groups.contains(n)).collect();
    eprintln!("groups without a collision instance: {without:?}");
    assert!(without.len() <= 2, "{without:?}");
}
