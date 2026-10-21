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
        sequence: Some(Sequence { package: Arc::new(package), engine: 0 }),
        strength: 20_000.0,
        suppress_distance: 50.0,
        visual_scale: [1.0; 10],
        hit_points: 5.0,
        hit_point_threshold: 0.75,
        hit_point_scale: [1.0; 10],
        shock_force: 40.0,
        shock_time: 3.0,
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
        shock_force: 20_000.0,
        before_velocity: Vec3::X * 20.0,
        other: OtherBody::default(),
    }
}

#[test]
fn source_damage_tiers_follow_impact_stimuli_once_and_explicit_destruction_does_not_replay_them() {
    let mut spec = (*engine().spec).clone();
    let sequence = spec.sequence.as_mut().unwrap();
    let package = Arc::make_mut(&mut sequence.package);
    let state = package.engines[0].systems.get_mut(&vlt_hash("DZ_REAR")).unwrap().states.get_mut(&0).unwrap();
    for n in 1..=2 {
        state.responses.insert(vlt_hash(&format!("DAMAGE_{n}")), Response { end_state: 0, action: 50 + n });
    }
    for n in 1..=2 {
        package.actions.insert(
            50 + n,
            Action { name_hash: 0, duration: 0.0, tags: BTreeMap::from([(TAGS.start, 50 + n)]), times: vec![] },
        );
        package.event_lists.insert(50 + n, vec![event(300 + n)]);
    }
    let mut events = VehicleEvents::new(Arc::new(spec)).unwrap();
    events.hit(hit());
    assert_eq!(ids(&events.tick(0.0)), [100, 104, 105, 301, 302]);
    events.hit(hit());
    assert_eq!(ids(&events.tick(0.0)), [100, 104, 105], "repeat impacts must not replay damage transitions");
    events.destroy();
    assert!(events.tick(0.0).is_empty(), "explicit destruction does not invent a sequencer stimulus");
    events.hit(hit());
    assert_eq!(ids(&events.tick(0.0)), [100, 104, 105]);
}

#[test]
fn severe_corner_impacts_damage_the_body_without_puncturing_tires() {
    let mut events = engine();
    for (arm, normal) in [
        (Vec3::new(-1.0, 0.0, 3.0), Vec3::X),
        (Vec3::new(1.0, 0.0, 3.0), Vec3::NEG_X),
        (Vec3::new(-1.0, 0.0, -3.0), Vec3::X),
        (Vec3::new(1.0, 0.0, -3.0), Vec3::NEG_X),
    ] {
        events.hit(DamageHit { arm, normal, force: 1_000_000.0, ..hit() });
    }
    events.tick(0.0);
    for corner in 4..8 {
        assert!(events.damage.zone(corner).panel > 0);
    }
    assert!((0..4).all(|wheel| !events.damage.is_punctured(wheel)));
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
    let package = Arc::make_mut(&mut spec.sequence.as_mut().unwrap().package);
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
    Arc::make_mut(&mut spec.sequence.as_mut().unwrap().package).event_lists.insert(50, vec![fire(50)]);
    events.spec = Arc::new(spec);
    events.hit(hit());
    assert!(events.advance(0.0).is_err());
}

#[test]
fn damage_uses_callback_normal_impulse_instead_of_final_friction_and_restitution_velocity() {
    use blackbox_vehicle::rigid_body::{RigidBody, RigidBodySpec};
    let mut body = RigidBody::new(1000.0, Vec3::ONE, Vec3::ONE, RigidBodySpec::default());
    body.linear_velocity = Vec3::X * 20.0;
    let hit = DamageHit::capture(&body, Vec3::ZERO, Vec3::X, 40.0, OtherBody::default());
    body.linear_velocity = Vec3::new(-4.0, 0.0, 8.0);
    assert_eq!(hit.resolved(&body, 20.0, false).force, 20_000.0);
    assert_eq!(hit.resolved(&body, 20.0, true).force, 40_000.0);
    assert_eq!(hit.before_velocity, Vec3::X * 20.0);
}

#[test]
fn cop_callback_filters_light_smackables_for_both_health_and_presentation() {
    for (mass, immobile, allowed) in
        [(500.0, false, false), (2000.0, false, false), (2000.1, false, true), (500.0, true, true)]
    {
        let hit = DamageHit { other: OtherBody { smackable: Some((mass, immobile)), ..Default::default() }, ..hit() };
        assert_eq!(hit.allows_cop(), allowed);
    }
    assert!(!DamageHit { other: OtherBody { no_car_effect: true, ..Default::default() }, ..hit() }.allows_cop());
    assert!(hit().allows_cop());
}
