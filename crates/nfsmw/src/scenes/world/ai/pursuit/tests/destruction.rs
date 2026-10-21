//! Real installed cop damage travels through production vehicle contacts, rendering and retirement.
mod health_mapping;
mod wreck_motion;
use super::*;
use crate::scenes::world::{
    drive::{CarSim, DriveInput},
    props::PropWorld,
};
use blackbox_vehicle::ground::NoGround;
use game_install::GameDir;
use glam::Quat;
use nfsmw_data::car::{LoadOptions, physics::PhysicsData};
use nfsmw_data::world::{DEFAULT_TRACK, WorldIndex};

fn data() -> (GameDir, PhysicsData) {
    let _ = env_logger::try_init();
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR")).unwrap();
    let mut data = PhysicsData::load(&dir).unwrap();
    data.set_sequences(WorldIndex::open(&dir, DEFAULT_TRACK).unwrap().sequences);
    (dir, data)
}

fn installed_model(dir: &GameDir, data: &PhysicsData, renderer: &mut NullBackend, name: &str) -> CopModel {
    let mut model = cop_model(renderer, name, player());
    model.model.physics = data.car(name).unwrap();
    let sequence = model.model.physics.events.as_ref().unwrap().sequence.as_ref().expect("installed cop sequencer");
    assert!(
        sequence.package.engines[sequence.engine].systems.contains_key(&blackbox_attrib::vlt_hash("DZ_FRONT")),
        "cop sequence must contain the front damage system"
    );
    let front = &sequence.package.engines[sequence.engine].systems[&blackbox_attrib::vlt_hash("DZ_FRONT")];
    let response = front.states[&front.initial].responses[&blackbox_attrib::vlt_hash("DAMAGE_1")];
    let action = &sequence.package.actions[&response.action];
    assert!(
        action.tags.contains_key(&nfsmw_data::world::sequences::TAGS.start),
        "original cop start tag must dispatch: actual={:?}, host={:08X}, times={:?}",
        action.tags,
        nfsmw_data::world::sequences::TAGS.start,
        action.times
    );
    let car = nfsmw_data::car::load_driving(
        dir,
        &name.to_ascii_uppercase(),
        &LoadOptions { lod: 'A', all_parts: false, preset: None },
    )
    .unwrap();
    model.model.rig = Rc::new(CarRig::upload(renderer, keys::next_car(), car, CarShading::default(), sun::to_sun()));
    model
}

fn spawn(world: &mut TrafficWorld, name: &str) {
    assert!(world.spawn_cop_at(
        name,
        CopSpot { at: Vec3::ZERO, target: Vec3::Z * 50.0, speed: 0.0, player: player(), band: None },
        &floor(),
        &SurfaceTable::default()
    ));
}

fn ram(world: &mut TrafficWorld, striker: &mut CarSim, speed: f32) {
    ram_direction(world, striker, speed, true);
}

fn ram_direction(world: &mut TrafficWorld, striker: &mut CarSim, speed: f32, side: bool) {
    ram_at(world, striker, speed, side, 0.0);
}

fn ram_at(world: &mut TrafficWorld, striker: &mut CarSim, speed: f32, side: bool, along: f32) {
    let cop = world.cars[0].sims_mut().0;
    let at = Vec3::Y * 10.0;
    cop.body_mut().place(at, Quat::IDENTITY);
    cop.step(&DriveInput::default(), &NoGround, None);
    let direction = if side { Vec3::X } else { Vec3::Z };
    let distance = if side { cop.half_dimensions().x } else { cop.half_dimensions().z }
        + striker.half_dimensions().z
        + (speed / 60.0).min(0.2);
    let heading = if side { -std::f32::consts::FRAC_PI_2 } else { std::f32::consts::PI };
    striker.body_mut().place(at + direction * distance + Vec3::Z * along, Quat::from_rotation_y(heading));
    striker.body_mut().linear_velocity = -direction * speed;
    striker.step(&DriveInput::default(), &NoGround, None);
    world.set_damage_view(at);
    world.collide(Some(striker));
    world.cars[0].sync_contacts();
}

fn health_effects(
    world: &TrafficWorld,
    effects: &mut crate::scenes::world::damage_effects::DamageEffects,
    dir: &GameDir,
    renderer: &mut NullBackend,
    expected_health: f32,
) {
    let source = world.damage_effect_sources().next().expect("actual cop must feed health effects");
    assert_eq!(source.health, expected_health);
    assert!(source.fx.select(source.health).is_some());
    effects.step(world.damage_effect_sources(), 1.0 / 60.0);
    effects.maintain(renderer, dir);
    for _ in 0..120 {
        effects.step(world.damage_effect_sources(), 1.0 / 60.0);
    }
    let mut batches = Vec::new();
    effects.textured(Vec3::new(-10.0, -10.0, 10.0), Vec3::Y, &mut batches);
    assert!(batches.iter().any(|batch| !batch.vertices.is_empty()), "physical cop health must reach particle batches");
}

#[test]
#[ignore = "needs installed cop models, damage records and event sequences"]
fn installed_cop_native_destruction_uses_resolved_contacts_and_independent_visuals() {
    let (dir, data) = data();
    {
        let mut renderer = NullBackend::new(crate::settings::test_caps::native()).with_mesh_tracking();
        let mut world = TrafficWorld::new(cop_road());
        world.load_data(data.database(), None);
        world.add_cop_model(installed_model(&dir, &data, &mut renderer, "copmidsize"));
        world.start_pursuit(1, nfsmw_data::pursuit::heat_rows(data.database()), player());
        spawn(&mut world, "copmidsize");
        let mut intact = Vec::new();
        world.cars[0].instances(1.0, &mut intact);
        let mut striker = CarSim::new(data.car("bmwm3gtr").unwrap(), [0.3; 4]);
        let mut props = PropWorld::default();
        let mut engine_effects = crate::scenes::world::damage_effects::DamageEffects::default();
        let mut smoke_seen = false;
        // Moderate real contacts must pass through the smoke band before wrecking.
        ram_direction(&mut world, &mut striker, 20.0, false);
        world.update_events(&mut props, &data, Vec3::ZERO);
        let mut damage = world.cars[0].cop_health_sample().unwrap().damage;
        eprintln!("COPMIDSIZE initial 20m/s front ram: damage={damage:.3}");
        let mut impacts = 0;
        while !world.cars[0].is_wrecked() && impacts < 12 {
            ram(&mut world, &mut striker, 20.0);
            let sample = world.cars[0].cop_health_sample().unwrap();
            assert!(sample.damage > damage, "a real side ram must damage the cop: {damage} -> {}", sample.damage);
            damage = sample.damage;
            eprintln!("COPMIDSIZE 20m/s side ram {}: damage={damage:.3}", impacts + 1);
            impacts += 1;
            world.update_events(&mut props, &data, Vec3::ZERO);
            if damage >= 0.5 && !world.cars[0].is_wrecked() {
                health_effects(&world, &mut engine_effects, &dir, &mut renderer, 1.0 - damage);
                smoke_seen = true;
            }
        }
        assert!(world.cars[0].is_wrecked(), "solid rams must disable the cop, damage={damage}");
        assert!(smoke_seen, "real rams must exercise the partially damaged engine effect band");
        health_effects(&world, &mut engine_effects, &dir, &mut renderer, 0.0);
        assert!(!world.cars[0].is_live_police() && !world.cars[0].emergency_lights());
        // Source glass/quarter-panel systems are driven by directional corner impacts, including later hits.
        ram_at(&mut world, &mut striker, 50.0, true, 2.0);
        world.update_events(&mut props, &data, Vec3::ZERO);
        for _ in 0..1800 {
            world.update_events(&mut props, &data, Vec3::ZERO);
        }
        let mask = world.cars[0].glass_mask();
        assert_ne!(mask, 0, "physical impacts must crack the cop's glass");
        assert!(world.spawn_cop_at(
            "copmidsize",
            CopSpot { at: Vec3::Z * 60.0, target: Vec3::ZERO, speed: 0.0, player: player(), band: None },
            &floor(),
            &SurfaceTable::default(),
        ));
        assert_eq!(world.cars[1].glass_mask(), 0, "another cop sharing the rig stays intact");
        let mut sibling_before = Vec::new();
        world.cars[1].instances(1.0, &mut sibling_before);
        world.update_damaged_windows(&mut renderer, sun::to_sun());
        let mut sibling_after = Vec::new();
        world.cars[1].instances(1.0, &mut sibling_after);
        assert_eq!(
            sibling_before.iter().map(|p| p.mesh).collect::<Vec<_>>(),
            sibling_after.iter().map(|p| p.mesh).collect::<Vec<_>>(),
            "preparing the damaged cop's variant must not change its intact sibling"
        );
        let mut damaged = Vec::new();
        world.cars[0].instances(1.0, &mut damaged);
        assert!(
            damaged.iter().any(|p| intact.iter().any(|q| q.key == p.key && q.mesh != p.mesh)),
            "cracked windows must select uploaded variants on the actual actor"
        );
        assert!(world.cars[0].take_wreck());
        assert!(!world.cars[0].take_wreck(), "one wreck transition per car");
        let particles = props.take_particle_commands();
        // The stock CopCar sequence names a glass marker absent from COPMIDSIZE collision children.
        // The original skips that effect; never fabricate a render marker or a central glass burst.
        let geometry = world.cars[0].sims_mut().0.geometry.as_ref().unwrap().clone();
        assert!(!geometry.bounds.children(geometry.bounds.root()).iter().any(|n| n.name_hash == 0x152A27EB));
        assert!(particles.is_empty(), "missing source glass markers must skip the effect");
        assert!(props.fragment_count() > 0, "supported free cop parts must release through installed damage stimuli");
        engine_effects.release(&mut renderer);
        eprintln!(
            "native: disabled in {impacts} physical side rams; damage={damage:.3}, glass={mask:#x}, {} detached parts, {} particle commands",
            props.fragment_count(),
            particles.len()
        );
    }
}

#[test]
#[ignore = "needs installed cop physics and damage records"]
fn installed_cop_damage_tuning_follows_linked_records_and_survives_glancing_impacts() {
    let (dir, data) = data();
    for name in [
        "copmidsize",
        "copghost",
        "copgto",
        "copgtoghost",
        "copsuv",
        "copsuvl",
        "copsuvpatrol",
        "copcross",
        "copsport",
        "copsportghost",
    ] {
        let spec = data.car(name).unwrap().events.unwrap();
        assert!(spec.strength > 0.0 && spec.hit_points > 0.0, "{name} needs resolved damage data");
        assert_eq!(spec.hit_point_scale[9], 0.0);
        eprintln!(
            "{name}: installed HP={}, threshold={}, FORCE={}, scales={:?}",
            spec.hit_points, spec.hit_point_threshold, spec.strength, spec.hit_point_scale
        );
    }
    let mut renderer = NullBackend::new(crate::settings::test_caps::native());
    let mut world = TrafficWorld::new(cop_road());
    world.add_cop_model(installed_model(&dir, &data, &mut renderer, "copmidsize"));
    spawn(&mut world, "copmidsize");
    let mut striker = CarSim::new(data.car("bmwm3gtr").unwrap(), [0.3; 4]);
    ram(&mut world, &mut striker, 1.5);
    assert!(!world.cars[0].is_wrecked());
    assert_eq!(world.cars[0].cop_health_sample().unwrap().damage, 0.0, "a gentle contact cannot destroy a cop");
}

#[test]
fn pursuit_breakers_destroy_live_roadblock_cars_and_count_each_once() {
    let mut world = TrafficWorld::new(cop_road());
    let mut renderer = NullBackend::new(crate::settings::test_caps::native());
    world.add_cop_model(cop_model(&mut renderer, "copx", player()));
    world.start_pursuit(1, vec![wave()], player());
    spawn(&mut world, "copx");
    world.cars[0].set_roadblock();
    world.breaker_stops(vec![crate::scenes::world::props::scenery::StopArea {
        position: world.cars[0].physics_position(),
        radius: 10.0,
        duration: 1.0,
        gas_station: false,
    }]);
    let mut props = PropWorld::default();
    for _ in 0..3 {
        world.step(focus(), &floor(), &mut props, &SurfaceTable::default());
    }
    assert!(world.cars[0].is_wrecked());
    assert_eq!(world.cops_destroyed(), 1);
    assert_eq!(world.cops_engaged(), 0);
    assert_eq!(world.cars[0].controls().gas, 0.0);
    assert!(world.cars[0].controls().brake <= 0.25, "destroyed roadblocks cannot retain their full hold brake");
}

#[test]
fn native_cop_goal_reaction_identity_changes_and_clears_with_the_live_goal() {
    use blackbox_pursuit::goals::{
        GoalKind,
        approach::{GoalCommand, Step},
    };
    let mut world = TrafficWorld::new(cop_road());
    let mut renderer = NullBackend::new(crate::settings::test_caps::native());
    world.add_cop_model(cop_model(&mut renderer, "copx", player()));
    spawn(&mut world, "copx");
    let step = Step { command: GoalCommand { target_point: None, desired_speed: 10.0, arrived: false }, limit: 10.0 };
    for (kind, name) in [
        (GoalKind::Pit, "AIGoalPit"),
        (GoalKind::BoxIn, "AIGoalRam"),
        (GoalKind::RollingBlock, "AIGoalRam"),
        (GoalKind::PullOver, "AIGoalPullOver"),
    ] {
        world.cars[0].set_goal_drive(Some(step), Some(kind));
        assert_eq!(world.cars[0].goal_drive().collision_goal, Some(blackbox_attrib::vlt_hash(name)));
    }
    world.cars[0].set_goal_drive(None, Some(GoalKind::Pit));
    assert_eq!(world.cars[0].goal_drive().collision_goal, None, "a rejected goal must return to default reactions");
    world.cars[0].set_roadblock();
    assert_eq!(world.cars[0].sims_mut().0.collision_goal, Some(blackbox_attrib::vlt_hash("AIGoalStaticRoadBlock")));
}

#[test]
fn native_cop_shock_precedes_light_prop_damage_filter_but_obeys_incoming_speed_gate() {
    use crate::scenes::world::{
        ai::collide::DriverClass,
        events::{DamageHit, OtherBody},
    };
    for (closing_speed, expected_brake) in [(10.0, 0.0), (0.5, 0.25)] {
        let mut world = TrafficWorld::new(cop_road());
        let mut renderer = NullBackend::new(crate::settings::test_caps::native());
        world.add_cop_model(cop_model(&mut renderer, "copx", player()));
        spawn(&mut world, "copx");
        let hit = DamageHit {
            arm: Vec3::ZERO,
            normal: Vec3::X,
            half: Vec3::ONE,
            closing_speed,
            force: 100_000.0,
            shock_force: 100_000.0,
            before_velocity: Vec3::ZERO,
            other: OtherBody { smackable: Some((10.0, false)), no_car_effect: true, ..Default::default() },
        };
        world.cars[0].on_hit(100_000.0, false, DriverClass::Traffic, 10.0, hit, 0.0);
        assert_eq!(world.cars[0].cop_health_sample().unwrap().damage, 0.0, "light props cannot consume cop health");
        world.cars[0].destroy_by_breaker();
        world.cars[0].sims_mut().0.body_mut().linear_velocity = Vec3::Z * 10.0;
        world.step(focus(), &floor(), &mut PropWorld::default(), &SurfaceTable::default());
        assert_eq!(
            world.cars[0].controls().brake,
            expected_brake,
            "closing={closing_speed}: shock controls retirement coasting"
        );
    }
}
