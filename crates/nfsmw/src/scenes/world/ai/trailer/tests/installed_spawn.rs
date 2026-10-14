//! New rigs meet real highway ground and run cargo events before any player contact.

use super::*;
use crate::app::test_backend::NullBackend;
use crate::scenes::world::{cargo, road, space, sun};
use crate::settings::CarShading;
use blackbox_gfx::{Capabilities, GraphicsApi};
use blackbox_roads::{RoadNetwork, lane_line, zone};
use game_install::GameDir;
use nfsmw_data::car::{LoadOptions, physics::PhysicsData};
use nfsmw_data::world::{DEFAULT_TRACK, WorldIndex};

mod driving;
mod materials;
mod slope;

fn model(dir: &GameDir, data: &PhysicsData, name: &str, renderer: &mut NullBackend) -> TrafficModel {
    let folders = nfsmw_data::car::list(dir);
    let folder = data
        .lineage(name)
        .into_iter()
        .map(|name| name.to_ascii_uppercase())
        .find(|name| folders.contains(name))
        .unwrap();
    let options = LoadOptions { lod: 'A', all_parts: false, preset: None };
    let model = nfsmw_data::car::load(dir, &folder, &options).unwrap();
    static NEXT_TEMPLATE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
    let template = NEXT_TEMPLATE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let rig = Rc::new(CarRig::upload(renderer, template, model, CarShading::default(), sun::to_sun()));
    TrafficModel {
        name: name.into(),
        rig,
        physics: data.car(name).unwrap(),
        tractor: data.is_tractor(name),
        trailer: None,
        cop_lights: None,
        audio_profile: crate::audio::traffic::TrafficProfile { horn: Some(5), ..Default::default() },
    }
}

fn highway(net: &RoadNetwork) -> (Spawn, usize, i32) {
    // Traffic's existing highway criterion is four traffic zones across the road.
    let max_highway_length = net
        .segments
        .iter()
        .enumerate()
        .filter(|(_, segment)| !segment.is_decision())
        .filter(|(i, segment)| {
            segment.nodes.iter().all(|&node| {
                net.profile_at(*i as u16, node).zones.iter().filter(|z| z.kind == zone::TRAFFIC).count() >= 4
            })
        })
        .map(|(_, segment)| segment.length)
        .fold(0.0_f32, f32::max);
    eprintln!("maximum four-traffic-zone nondecision segment: {max_highway_length:.2} m");
    let mut long = 0;
    let mut wide = 0;
    let mut straight = 0;
    let (segment, lane) = net
        .segments
        .iter()
        .enumerate()
        .filter(|(_, segment)| !segment.is_decision() && segment.length > 40.0)
        .filter_map(|(i, segment)| {
            long += 1;
            let profiles = segment.nodes.map(|node| net.profile_at(i as u16, node));
            if profiles.iter().any(|p| p.zones.iter().filter(|z| z.kind == zone::TRAFFIC).count() < 4) {
                return None;
            }
            wide += 1;
            let lane = profiles[0].lanes_of(zone::TRAFFIC, true).first().copied()?;
            let curve = lane_line(net, i as u16, 1, lane);
            let start = curve.position(0.3);
            let tangent = curve.tangent(0.3);
            let direction = Vec3::new(tangent.x, 0.0, tangent.z).normalize();
            let right = blackbox_roads::right_of(direction);
            // Shorter spans can still diagnose the first second of visible startup.
            let pieces = match segment.length >= 100.0 {
                true => 10,
                false => 3,
            };
            if (0..=pieces).any(|step| {
                let t = 0.3 + step as f32 * 5.0 / segment.length;
                (curve.position(t) - start).dot(right).abs() > 0.5
            }) {
                return None;
            }
            straight += 1;
            Some((i as u16, lane, segment.length))
        })
        .max_by(|a, b| a.2.total_cmp(&b.2))
        .map(|(segment, lane, _)| (segment, lane))
        .unwrap_or_else(|| {
            panic!("no highway fixture: {long} long segments, {wide} with four lanes, {straight} nearly straight")
        });
    let line = lane_line(net, segment, 1, lane);
    let position = space::to_render(line.position(0.3).to_array());
    let forward = space::to_render(line.tangent(0.3).to_array());
    let spawn = Spawn { position, heading: forward.y.atan2(forward.x) };
    eprintln!(
        "highway fixture: segment {segment} lane {lane}, length {:.2} m, spawn {spawn:?}",
        net.segment(segment).length
    );
    eprintln!("highway fixture candidates: {long} long segments, {wide} with four lanes, {straight} nearly straight");
    let (ticks, pieces) = match net.segment(segment).length >= 100.0 {
        true => (180, 10),
        false => (60, 3),
    };
    eprintln!("highway fixture: {ticks} fixed startup ticks, {} m ahead", pieces * 5);
    (spawn, ticks, pieces)
}

fn collision(dir: &GameDir, index: &WorldIndex, spawn: Spawn) -> CollisionWorld {
    let mut world = CollisionWorld::new(index.collision_grid.clone());
    let mut count = 0;
    for section in &index.sections {
        if !section.is_spatial() || section.distance_to(spawn.position.x, spawn.position.y) > 250.0 {
            continue;
        }
        let bytes = dir.read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize).unwrap();
        for pack in blackbox_collision::read_collision_packs(&bytes).unwrap() {
            world.insert(pack);
            count += 1;
        }
    }
    assert!(count > 0, "fixture must load highway collision packs");
    eprintln!("highway fixture: {count} resident collision packs");
    world
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_loaded_trucks_spawn_on_highway_without_contact_or_cargo_release() {
    let path = std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR to run installed tests");
    let dir = GameDir::open(path).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let (mut spawn, ticks, pieces) = highway(index.road_network.as_ref().unwrap());
    let collision = collision(&dir, &index, spawn);
    let road = road::probe(&collision, spawn.position.x, spawn.position.y, spawn.position.z + 10.0).unwrap();
    assert!(road.road, "fixture must start on authored highway asphalt");
    spawn.position.z = road.height;
    let forward = Vec3::new(spawn.heading.cos(), spawn.heading.sin(), 0.0);
    for distance in (-6..=pieces).map(|step| step as f32 * 5.0) {
        let at = spawn.position + forward * distance;
        let surface = road::probe(&collision, at.x, at.y, at.z + 10.0);
        assert!(surface.is_some_and(|surface| surface.road), "fixture asphalt missing at {distance} m: {surface:?}");
    }
    let mut data = PhysicsData::load(&dir).unwrap();
    data.set_sequences(index.sequences.clone());
    let ground = WorldGround { collision: &collision, surfaces: &data.surfaces };
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    let speed = crate::scenes::world::ai::traffic::STREET_SPEED * 0.75;
    for name in ["semib", "semilog"] {
        let tractor_model = model(&dir, &data, name, &mut renderer);
        let trailer_model = model(&dir, &data, &data.trailer_of(name).unwrap(), &mut renderer);
        let mut tractor = CarSim::for_rig(tractor_model.physics.clone(), &tractor_model.rig);
        tractor.configure_ai();
        assert!(tractor.place_traffic_moving(&ground, spawn, speed));
        let mut trailer = TrailerCar::place(&trailer_model, &tractor, spawn, speed, &ground).unwrap();
        assert!(trailer.sim.events.is_some(), "fixture must run installed cargo events");
        let anchors = trailer.coupling.joint.world_anchors(tractor.body(), trailer.sim.body());
        let initial_gap = trailer.coupling.gap(&tractor, &trailer.sim);
        let initial = [tractor.body().position, trailer.sim.body().position];
        let mut props = PropWorld::default();
        let mut max_gap = 0.0_f32;
        let mut max_vertical_speed = [0.0_f32; 2];
        let mut max_angular_speed = [0.0_f32; 2];
        let mut min_up = [1.0_f32; 2];
        let mut damage = 0;
        let mut first_release = None;
        for tick in 0..ticks {
            props.step(&ground, STEP);
            let input = DriveInput { throttle: 0.15, ..DriveInput::default() };
            let a = tractor.step(&input, &ground, Some((&collision, &mut props)));
            // Same step/event/contact order as TrafficWorld, retaining the world's damage trace.
            let b = trailer.sim.step(&DriveInput::default(), &ground, Some((&collision, &mut props)));
            trailer.coupling.update_hitch(&tractor, &trailer.sim);
            for (body, impact) in [("tractor", a), ("trailer", b)] {
                if !impact.damage.is_empty() {
                    eprintln!(
                        "{name} tick {tick} {body}: world damage {:?}, deepest {:?}",
                        impact.damage, impact.deepest
                    );
                }
                damage += impact.damage.len();
            }
            let view = tractor.pose().position;
            cargo::update(&mut tractor, &tractor_model.rig, &mut props, &data, view);
            trailer.update_events(&mut props, &data, view);
            trailer.sync_contacts(&mut tractor);
            max_gap = max_gap.max(trailer.coupling.gap(&tractor, &trailer.sim));
            for (i, sim) in [&tractor, &trailer.sim].into_iter().enumerate() {
                assert!(sim.is_finite(), "{name} body {i} tick {tick}");
                max_vertical_speed[i] = max_vertical_speed[i].max(sim.body().linear_velocity.y.abs());
                max_angular_speed[i] = max_angular_speed[i].max(sim.body().angular_velocity.length());
                min_up[i] = min_up[i].min(sim.up().y);
            }
            if props.fragment_count() > 0 && first_release.is_none() {
                first_release = Some(tick);
            }
            if tick < 3 {
                eprintln!(
                    "{name} tick {tick}: gap {:.5}, y {:?}, up {:?}, vy {:?}, fragments {}",
                    trailer.coupling.gap(&tractor, &trailer.sim),
                    [tractor.body().position.y, trailer.sim.body().position.y],
                    [tractor.up().y, trailer.sim.up().y],
                    [tractor.body().linear_velocity.y, trailer.sim.body().linear_velocity.y],
                    props.fragment_count()
                );
            }
        }
        eprintln!(
            "{name}: anchors {anchors:?}, initial gap {initial_gap:.5}, initial {initial:?}, max gap {max_gap:.5}, max vy {max_vertical_speed:?}, max angular {max_angular_speed:?}, min up {min_up:?}, world damage {damage}, cargo {} first release {first_release:?}, hitched {}",
            props.fragment_count(),
            trailer.is_hitched()
        );
        assert_eq!(damage, 0, "fixture must not strike a wall or prop before evaluating startup");
        assert!(trailer.is_hitched(), "{name} released on a straight highway without another car");
        assert_eq!(props.fragment_count(), 0, "{name} released cargo without an impact");
        assert!(max_gap < 0.1, "{name} hitch error {max_gap:.5} m");
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_same_model_trucks_and_loads_keep_distinct_stable_render_identities() {
    use crate::scenes::world::ai::car::{AiCar, Ctx, Role, Start};
    use crate::scenes::world::ai::traffic::Cruise;
    use blackbox_roads::{LaneType, NavKind, RoadNav, SegmentFilter, SegmentIndex, SplitMix};
    use std::collections::BTreeSet;

    let path = std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR to run installed tests");
    let dir = GameDir::open(path).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let net = index.road_network.as_ref().unwrap();
    let segments = SegmentIndex::build(net);
    let (mut spawn, _, _) = highway(net);
    let collision = collision(&dir, &index, spawn);
    let surface = road::probe(&collision, spawn.position.x, spawn.position.y, spawn.position.z + 10.0).unwrap();
    assert!(surface.road, "fixture must stand on highway asphalt");
    spawn.position.z = surface.height;
    let mut data = PhysicsData::load(&dir).unwrap();
    data.set_sequences(index.sequences.clone());
    let ground = WorldGround { collision: &collision, surfaces: &data.surfaces };
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    let mut model = model(&dir, &data, "semilog", &mut renderer);
    model.trailer = Some(Box::new(self::model(&dir, &data, "trailerlog", &mut renderer)));
    let forward = Vec3::new(spawn.heading.cos(), spawn.heading.sin(), 0.0);
    let start = Start { stagger: 0, speed: 8.0, cruise: Cruise::default(), patrol: false };
    let place = |at| {
        let spawn = Spawn { position: at, heading: spawn.heading };
        let mut nav = RoadNav::new(NavKind::Direction, LaneType::Racing, SegmentFilter::default());
        assert!(nav.init_at_point(
            net,
            &segments,
            Vec3::from(space::to_physics(at)),
            Vec3::from(space::to_physics(forward)),
            false
        ));
        AiCar::place(&model, Role::Traffic, nav, spawn, start, &ground).unwrap()
    };
    let mut first = place(spawn.position);
    let second = place(spawn.position + forward * 35.0);
    assert_eq!(Rc::strong_count(&model.rig), 3, "both tractors must reuse the uploaded template");
    assert!(
        Rc::ptr_eq(&first.trailer().unwrap().rig, &second.trailer().unwrap().rig),
        "both trailers must retain the same uploaded rig"
    );
    assert_eq!(
        Rc::strong_count(&model.trailer.as_ref().unwrap().rig),
        3,
        "both loads must reuse the uploaded template"
    );
    let draw = |car: &AiCar| {
        let mut instances = Vec::new();
        car.instances(1.0, &mut instances);
        instances
    };
    let a = draw(&first);
    let b = draw(&second);
    assert!(!a.is_empty() && a.len() == b.len(), "fixture must draw two copies of the loaded truck");
    assert_eq!(
        a.iter().map(|i| i.mesh).collect::<Vec<_>>(),
        b.iter().map(|i| i.mesh).collect::<Vec<_>>(),
        "actors must share meshes"
    );
    let keys = |instances: &[blackbox_render::Instance]| instances.iter().map(|i| i.key).collect::<BTreeSet<_>>();
    let first_keys = keys(&a);
    let second_keys = keys(&b);
    assert_eq!(first_keys.len(), a.len(), "tractor and loaded trailer need separate identities");
    assert_eq!(second_keys.len(), b.len(), "second tractor and loaded trailer need separate identities");
    let duplicates: Vec<_> = first_keys.intersection(&second_keys).copied().collect();
    eprintln!(
        "two shared-template trucks: {} + {} rendered parts, {} duplicate actor keys",
        a.len(),
        b.len(),
        duplicates.len()
    );
    assert!(
        duplicates.is_empty(),
        "the Bevy instance pool would overwrite {} truck/load parts between actors: {duplicates:?}",
        duplicates.len()
    );

    let bodies = [(first.avoidable(), false), (second.avoidable(), false)];
    let mut rng = SplitMix(1);
    let mut props = PropWorld::default();
    let mut context =
        Ctx { net, index: &segments, rng: &mut rng, bodies: &bodies, me: 0, player: None, aims: &[], lights: None };
    first.step(&mut context, &collision, &mut props, &data.surfaces);
    first.update_events(&mut props, &data, spawn.position);
    first.sync_contacts();
    let moved = draw(&first);
    let still = draw(&second);
    assert_eq!(keys(&moved), first_keys, "moving an actor must preserve every part's identity");
    assert_eq!(keys(&still), second_keys, "another actor's identities must stay stable");
    assert!(
        a.iter().zip(&moved).any(|(a, b)| a.transform != b.transform),
        "fixture must physically move the first truck"
    );
    assert!(
        b.iter().zip(&still).all(|(a, b)| a.key == b.key && a.mesh == b.mesh && a.transform == b.transform),
        "moving one truck must not change the other's meshes or transforms"
    );
}
