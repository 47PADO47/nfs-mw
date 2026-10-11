use std::collections::BTreeMap;

use blackbox_sequence::{Action, Engine, Event, Package, Response, State, System};
use nfsmw_data::world::sequences::Sequence;

use super::*;

fn event(id: u32) -> Event {
    Event { id, data: id.to_le_bytes().to_vec() }
}

fn engine() -> VehicleEvents {
    let stimuli = [
        "COLLISION",
        "COLLISION_VEHICLE",
        "COLLISION_COP",
        "COLLISION_PLAYER",
        "IMPACT_1",
        "IMPACT_2",
        "IMPACT_3",
        "IMPACT_4",
        "IMPACT_5",
        "IMPACT_6",
    ];
    let responses =
        stimuli.iter().enumerate().map(|(i, s)| (vlt_hash(s), Response { end_state: 0, action: i as u32 })).collect();
    let state = State { filter: None, responses };
    let system = System { initial: 0, default: 0, states: BTreeMap::from([(0, state)]) };
    let package = Package {
        engines: vec![Engine {
            name: "synthetic".into(),
            name_hash: 0,
            systems: DAMAGE_ZONES.map(|s| (vlt_hash(s), system.clone())).into_iter().collect(),
        }],
        actions: (0..10)
            .map(|i| {
                (i, Action { name_hash: 0, duration: 0.0, tags: BTreeMap::from([(TAGS.start, i)]), times: vec![] })
            })
            .collect(),
        event_lists: (0..10).map(|i| (i, vec![event(100 + i)])).collect(),
        ..Package::default()
    };
    VehicleEvents::new(Arc::new(CarEventSpec {
        sequence: Sequence { package: Arc::new(package), engine: 0 },
        strength: 20_000.0,
        suppress_distance: 50.0,
        visual_scale: [1.0; 10],
    }))
    .unwrap()
}

fn hit() -> DamageHit {
    DamageHit {
        arm: Vec3::NEG_Z * 3.0,
        normal: Vec3::Z,
        half: Vec3::new(1.0, 1.0, 4.0),
        closing_speed: 10.0,
        force: 40_000.0,
        before_velocity: Vec3::X * 20.0,
        other: OtherBody::default(),
    }
}

fn ids(events: &[Effect]) -> Vec<u32> {
    events
        .iter()
        .map(|e| match e.event {
            WorldEvent::Other(id) => id,
            other => panic!("unexpected {other:?}"),
        })
        .collect()
}

#[test]
fn all_zones_respect_strict_position_and_normal_boundaries() {
    for (arm, normal, zone) in [
        ([0.0, 0.0, 3.0], [0.0, 0.0, -1.0], 0),
        ([0.0, 0.0, -3.0], [0.0, 0.0, 1.0], 1),
        ([-1.0, 0.0, 0.0], [1.0, 0.0, 0.0], 2),
        ([1.0, 0.0, 0.0], [-1.0, 0.0, 0.0], 3),
        ([-1.0, 0.0, 3.0], [1.0, 0.0, 0.0], 4),
        ([1.0, 0.0, 3.0], [-1.0, 0.0, 0.0], 5),
        ([-1.0, 0.0, -3.0], [1.0, 0.0, 0.0], 6),
        ([1.0, 0.0, -3.0], [-1.0, 0.0, 0.0], 7),
        ([0.0, 1.0, 0.0], [0.0, -1.0, 0.0], 8),
        ([0.0, -1.0, 0.0], [0.0, 1.0, 0.0], 9),
        ([0.5, 0.0, 3.0], [-1.0, 0.0, 0.0], 0),
        ([1.0, 0.0, 3.0], [-0.8, 0.0, 0.6], 0),
        ([0.0, 0.0, 2.0], [0.0, 0.0, -1.0], 2),
    ] {
        assert_eq!(DamageHit { arm: arm.into(), normal: normal.into(), ..hit() }.zone(), zone);
    }
}

#[test]
fn stimuli_keep_original_order_and_immediate_pre_impact_context() {
    let mut events = engine();
    events
        .hit(DamageHit { other: OtherBody { vehicle: true, player: true, cop: true, ..Default::default() }, ..hit() });
    let out = events.tick(0.0);
    assert_eq!(ids(&out), [100, 101, 102, 103, 104, 105]);
    assert!(out.iter().all(|e| e.velocity == Some(hit().before_velocity)));
    assert!(events.tick(0.0).is_empty());
}

#[test]
fn small_slow_suppressed_and_no_effect_hits_do_not_trigger_cargo_damage() {
    for change in [
        DamageHit { closing_speed: 0.99, ..hit() },
        DamageHit { other: OtherBody { no_car_effect: true, ..Default::default() }, ..hit() },
    ] {
        let mut events = engine();
        events.hit(change);
        assert!(events.tick(0.0).is_empty());
    }
    let mut events = engine();
    events.hit(hit());
    assert!(events.tick(50.01).is_empty());
    events.hit(DamageHit { force: 19_999.0, ..hit() });
    assert_eq!(ids(&events.tick(50.0)), [100]);
    events.hit(DamageHit { force: 20_000.0, ..hit() });
    assert_eq!(ids(&events.tick(0.0)), [100, 104]);
    events.hit(DamageHit { force: 1_000_000.0, ..hit() });
    assert_eq!(ids(&events.tick(0.0)), [100, 104, 105, 106, 107, 108, 109]);
}

#[test]
fn nested_lists_execute_in_place_and_cycles_stop_with_an_error() {
    let mut events = engine();
    let mut spec = (*events.spec).clone();
    let package = Arc::make_mut(&mut spec.sequence.package);
    let fire = |list: u32| Event {
        id: 0xA3CF_BFEC,
        data: [0xA3CF_BFEC, list, 0, 0].into_iter().flat_map(u32::to_le_bytes).collect(),
    };
    package.event_lists.insert(0, vec![event(200), fire(50), event(203)]);
    package.event_lists.insert(50, vec![event(201), event(202)]);
    events.spec = Arc::new(spec.clone());
    events.hit(DamageHit { force: 0.0, ..hit() });
    let out = events.tick(0.0);
    assert_eq!(ids(&out), [200, 201, 202, 203]);
    assert!(out.iter().all(|e| e.velocity == Some(hit().before_velocity)));
    Arc::make_mut(&mut spec.sequence.package).event_lists.insert(50, vec![fire(50)]);
    events.spec = Arc::new(spec);
    events.hit(hit());
    assert!(events.advance(0.0).is_err());
}
