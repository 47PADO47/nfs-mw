//! Bounded startup probes use the actual driver on legal curved and sloping highway lanes.

use super::*;
use crate::scenes::world::ai::car::{AiCar, Ctx, Role, Start};
use crate::scenes::world::ai::traffic::Cruise;
use blackbox_roads::{LaneType, NavKind, RoadNav, SegmentFilter, SegmentIndex, SplitMix, flags};
use blackbox_vehicle::Ground;

fn candidates(net: &RoadNetwork) -> Vec<(u16, Spawn, f32)> {
    let mut ranked = Vec::new();
    for (i, segment) in net.segments.iter().enumerate() {
        if segment.is_decision() || segment.has(flags::NO_TRAFFIC) || segment.length < 80.0 {
            continue;
        }
        let profile = net.profile_at(i as u16, segment.nodes[0]);
        if profile.zones.iter().filter(|z| z.kind == zone::TRAFFIC).count() < 4 {
            continue;
        }
        let Some(lane) = profile.lanes_of(zone::TRAFFIC, true).first().copied() else { continue };
        let curve = lane_line(net, i as u16, 1, lane);
        let tangent = curve.tangent(0.3);
        let grade = tangent.y.abs() / tangent.x.hypot(tangent.z).max(0.01);
        let curvature = curve.curvature_xz(0.3).abs();
        let position = space::to_render(curve.position(0.3).to_array());
        let heading = space::to_render(tangent.to_array());
        ranked.push((i as u16, Spawn { position, heading: heading.y.atan2(heading.x) }, curvature * 1000.0 + grade));
    }
    ranked.sort_by(|a, b| b.2.total_cmp(&a.2));
    ranked
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_loaded_truck_drivers_have_bounded_startup_on_curved_highways() {
    let path = std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR to run installed tests");
    let dir = GameDir::open(path).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let net = index.road_network.as_ref().unwrap();
    let segments = SegmentIndex::build(net);
    let mut data = PhysicsData::load(&dir).unwrap();
    data.set_sequences(index.sequences.clone());
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    let models: Vec<_> = ["semib", "semilog"]
        .into_iter()
        .map(|name| {
            let mut tractor = model(&dir, &data, name, &mut renderer);
            tractor.trailer = Some(Box::new(model(&dir, &data, &data.trailer_of(name).unwrap(), &mut renderer)));
            tractor
        })
        .collect();
    let start = Start { stagger: 0, speed: Cruise::default().street * 0.75, cruise: Cruise::default(), patrol: false };
    let mut selected: Vec<Vec3> = Vec::new();
    let mut runs = 0;
    for (segment, probe, score) in candidates(net).into_iter().take(40) {
        if selected.iter().any(|at| at.truncate().distance(probe.position.truncate()) < 200.0) {
            continue;
        }
        let collision = collision(&dir, &index, probe);
        let ground = WorldGround { collision: &collision, surfaces: &data.surfaces };
        let heading = Vec3::from(space::to_physics(Vec3::new(probe.heading.cos(), probe.heading.sin(), 0.0)));
        let mut nav = RoadNav::new(NavKind::Direction, LaneType::Racing, SegmentFilter::default());
        let mut rng = SplitMix(13);
        if !nav.init_at_point(net, &segments, Vec3::from(space::to_physics(probe.position)), heading, false)
            || !nav.can_traffic_spawn(net, &mut rng)
        {
            continue;
        }
        let position = space::to_render(nav.position.to_array());
        let Some(surface) = road::probe(&collision, position.x, position.y, position.z + 10.0) else { continue };
        if !surface.road {
            continue;
        }
        let forward = space::to_render(nav.forward.to_array());
        let spawn =
            Spawn { position: Vec3::new(position.x, position.y, surface.height), heading: forward.y.atan2(forward.x) };
        let Some(hit) = ground.hit(Vec3::from(space::to_physics(spawn.position)) + Vec3::Y * 2.0, Vec3::NEG_Y, 5.0)
        else {
            continue;
        };
        let original_nav = nav.clone();
        eprintln!(
            "curved highway fixture: candidate {segment}, active {} lane {}, score {score:.5}, spawn {spawn:?}, ground normal {:?}, curvature {:.5}",
            nav.segment, nav.lane, hit.normal, nav.curvature
        );
        for model in &models {
            let mut nav = original_nav.clone();
            nav.half_width = model.physics.spec.dimension.x;
            nav.enable_trail(net);
            nav.advance_with_lookahead(net, 30.0, Vec3::ZERO, 30.0, &mut rng);
            let Some(mut car) = AiCar::place(model, Role::Traffic, nav, spawn, start, &ground) else { continue };
            let mut props = PropWorld::default();
            let mut max_vertical = [0.0_f32; 2];
            let mut max_angular = [0.0_f32; 2];
            let mut min_up = [1.0_f32; 2];
            let mut max_gap = 0.0_f32;
            let mut world_hits = 0;
            let mut tick_count = 0;
            let mut first_fragments = None;
            for tick in 0..120 {
                props.step(&ground, STEP);
                let bodies = [(car.avoidable(), false)];
                let mut context = Ctx {
                    net,
                    index: &segments,
                    rng: &mut rng,
                    bodies: &bodies,
                    me: 0,
                    player: None,
                    aims: &[],
                    lights: None,
                };
                car.step(&mut context, &collision, &mut props, &data.surfaces);
                let (tractor, trailer) = car.sims_mut();
                let trailer = trailer.unwrap();
                for (i, sim) in [tractor, trailer].into_iter().enumerate() {
                    assert!(sim.is_finite(), "{} body {i} tick {tick}", model.name);
                    max_vertical[i] = max_vertical[i].max(sim.body().linear_velocity.y.abs());
                    max_angular[i] = max_angular[i].max(sim.body().angular_velocity.length());
                    min_up[i] = min_up[i].min(sim.up().y);
                    if let Some(events) = &sim.events {
                        for hit in events.pending_hits() {
                            eprintln!("{} tick {tick} body {i}: real world DamageHit {hit:?}", model.name);
                        }
                        world_hits += events.pending_hits().len();
                    }
                }
                car.update_events(&mut props, &data, spawn.position);
                car.sync_contacts();
                let joint = car.trailer().unwrap().coupling.joint;
                let (tractor, trailer) = car.sims_mut();
                let trailer = trailer.unwrap();
                assert!(tractor.is_finite() && trailer.is_finite(), "{} solve tick {tick}", model.name);
                max_gap = max_gap.max(joint.gap(tractor.body(), trailer.body()));
                if props.fragment_count() > 0 && first_fragments.is_none() {
                    first_fragments = Some(tick);
                }
                tick_count += 1;
                // A real barrier response is a distinct driving/collision case, not a spawn-only test.
                if world_hits > 0 {
                    break;
                }
                let at = Vec3::from(space::to_physics(car.position()));
                if let Some(located) =
                    blackbox_roads::closest_segment(net, &segments, at, heading, 1.0, SegmentFilter::default())
                    && located.segment != original_nav.segment
                {
                    break;
                }
            }
            eprintln!(
                "{} curved startup: {tick_count} ticks, max vy {max_vertical:?}, max angular {max_angular:?}, min up {min_up:?}, gap {max_gap:.5}, world hits {world_hits}, cargo {} first {first_fragments:?}, hitched {}",
                model.name,
                props.fragment_count(),
                car.trailer().unwrap().is_hitched()
            );
            if world_hits == 0 {
                assert_eq!(props.fragment_count(), 0, "{} released cargo without any other body", model.name);
                assert!(car.trailer().unwrap().is_hitched(), "{} released its hitch without contact", model.name);
                assert!(max_gap < 0.1, "{} hitch error {max_gap:.5} m", model.name);
            }
            runs += 1;
        }
        selected.push(spawn.position);
        if selected.len() == 6 {
            break;
        }
    }
    assert_eq!(runs, 12, "fixture must probe both loaded truck types at six separate highway sites");
}
