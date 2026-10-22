//! Named masks are separate from panels; installed actions supply the collision trigger.
use super::*;

fn named(name: &str) -> Event {
    let id = vlt_hash("EDamageLights");
    Event { id, data: [id.to_le_bytes(), vlt_hash(name).to_le_bytes()].concat() }
}

#[test]
fn named_lights_are_consumed_selectively_and_repeated_unknown_events_do_not_poison_the_engine() {
    let mut spec = (*engine().spec).clone();
    let package = Arc::make_mut(&mut spec.sequence.as_mut().unwrap().package);
    package.event_lists.get_mut(&0).unwrap().extend([
        named("LIGHT_COPS"),
        named("LIGHT_LHEAD"),
        named("not_an_original_light"),
        named("LIGHT_COPS"),
    ]);
    let mut events = VehicleEvents::new(Arc::new(spec)).unwrap();
    events.hit(hit());
    let output = events.tick(0.0);
    assert_eq!(ids(&output), [100, 104, 105], "named mutations are consumed, unrelated events keep their order");
    assert_eq!(events.broken_lights(), 0x7001, "the right head and every brake/reverse lamp remain intact");
    events.hit(hit());
    assert_eq!(ids(&events.tick(0.0)), [100, 104, 105]);
    assert!(!events.failed);
    assert_eq!(events.broken_lights(), 0x7001);
    assert!(events.set_light_damage(vlt_hash("LIGHT_LHEAD"), false));
    assert_eq!(events.broken_lights(), 0x7000);
    assert!(!events.set_light_damage(vlt_hash("not_an_original_light"), true));
    assert_eq!(events.broken_lights(), 0x7000);
}

#[test]
fn named_lamp_groups_accumulate_and_destruction_overrides_selective_clear_until_repair() {
    let mut events = engine();
    assert!(events.set_light_damage(vlt_hash("LIGHT_HEADLIGHTS"), true));
    assert!(events.set_light_damage(vlt_hash("LIGHT_RBRAKE"), true));
    assert_eq!(events.broken_lights(), 7 | 16);
    assert!(events.set_light_damage(vlt_hash("LIGHT_LHEAD"), false));
    assert_eq!(events.broken_lights(), 6 | 16);
    events.destroy();
    assert!(events.set_light_damage(vlt_hash("LIGHT_HEADLIGHTS"), false));
    assert_eq!(events.broken_lights(), u32::MAX);
    events.repair_lights();
    assert_eq!(events.broken_lights(), 0);
    assert!(events.set_light_damage(vlt_hash("LIGHT_LREVERSE"), true));
    assert_eq!(events.broken_lights(), 64, "repair clears the old independent mask as well as destruction");
}

fn installed_spec() -> Arc<CarEventSpec> {
    use game_install::GameDir;
    use nfsmw_data::{
        car::physics::PhysicsData,
        world::{DEFAULT_TRACK, WorldIndex},
    };
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let mut data = PhysicsData::load(&dir).unwrap();
    data.set_sequences(index.sequences);
    let spec = data.car("copmidsize").unwrap().events.unwrap();
    let sequence = spec.sequence.as_ref().expect("the actual police physics must bind an installed event engine");
    assert_eq!(sequence.package.engines[sequence.engine].name, "CopCar");
    spec
}

fn tier(spec: &CarEventSpec, zone: usize, level: f32) -> DamageHit {
    assert!(spec.strength > 0.0 && spec.visual_scale[zone] > 0.0);
    let (arm, normal) = match zone {
        8 => (Vec3::Y * 2.0, Vec3::NEG_Y),
        2 => (Vec3::NEG_X * 2.0, Vec3::X),
        0 => (Vec3::Z * 3.0, Vec3::NEG_Z),
        _ => panic!("unsupported test zone"),
    };
    DamageHit { arm, normal, force: level * spec.strength / spec.visual_scale[zone], ..hit() }
}

#[test]
#[ignore = "needs installed CopCar sequences and police physics"]
fn installed_top_and_left_collisions_reach_authored_named_cop_lamp_damage() {
    let spec = installed_spec();
    let mut control = VehicleEvents::new(spec.clone()).unwrap();
    control.hit(tier(&spec, 0, 1.1));
    control.tick(0.0);
    assert_eq!(control.broken_lights(), 0, "a front visual tier is not an invented cop-light trigger");
    for zone in [8, 2] {
        let mut events = VehicleEvents::new(spec.clone()).unwrap();
        events.hit(tier(&spec, zone, 1.1));
        let mut effects = events.tick(0.0);
        if zone == 2 {
            assert_eq!(events.broken_lights(), 0, "left tier one must not skip its authored state");
            for _ in 0..61 {
                effects.extend(events.tick(0.0));
            }
            events.hit(tier(&spec, zone, 2.1));
            effects.extend(events.tick(0.0));
        }
        assert!(!events.failed);
        assert_eq!(
            events.broken_lights(),
            0x7000,
            "zone {zone}: actual nested stimulus must damage only the light bar"
        );
        assert!(
            effects.iter().any(|effect| matches!(effect.event, WorldEvent::Fragment(_))),
            "the same real action must still detach its fragment"
        );
        assert!(effects.iter().all(|effect| !matches!(effect.event, WorldEvent::DamageLights(_))));
        events.hit(tier(&spec, zone, 2.1));
        events.tick(0.0);
        assert_eq!(events.broken_lights(), 0x7000);
        events.repair_lights();
        assert_eq!(events.broken_lights(), 0);
    }
}

#[test]
#[ignore = "needs installed CopCar sequences and police physics"]
fn installed_root_explosion_nested_lists_preserve_the_named_light_target() {
    let spec = installed_spec();
    let sequence = spec.sequence.as_ref().unwrap().clone();
    let mut events = VehicleEvents::new(spec).unwrap();
    assert!(
        events
            .player
            .as_mut()
            .unwrap()
            .stimulate(&sequence.package, Some(0), vlt_hash("EXPLOSION_DAMAGE"), QueueMode::Allow, Some(1),)
            .unwrap()
    );
    let effects = events.tick(0.0);
    assert!(!events.failed);
    assert_eq!(events.broken_lights(), 0x7000);
    assert!(effects.iter().any(|effect| matches!(effect.event, WorldEvent::Fragment(_))));
}
