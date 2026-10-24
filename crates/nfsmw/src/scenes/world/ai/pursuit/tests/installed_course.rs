//! Physical multi-unit encounter on a curved installed lane, with no injected visibility answers.

use super::installed_route::{LOADED_START, diagnostic, open, target, tick};
use super::*;
use crate::scenes::world::{drive::STEP, props::PropWorld};
use crate::viewer::Scene;
use blackbox_collision::RayOptions;
use blackbox_roads::{LaneType, NavKind, SplitMix};

const TARGET_SPEED: f32 = 12.0;
const SECONDS: f32 = 8.0;
const TARGET_LEAD: f32 = 45.0;

fn place(
    model: &CopModel,
    mut nav: RoadNav,
    stagger: u32,
    collision: &CollisionWorld,
    surfaces: &SurfaceTable,
) -> Option<AiCar> {
    let mut position = space::to_render(nav.position.to_array());
    let ground = crate::scenes::world::road::probe(collision, position.x, position.y, position.z + 30.0)?;
    if !ground.road {
        return None;
    }
    position.z = ground.height;
    nav.half_width = model.model.physics.spec.dimension.x;
    let forward = space::to_render(nav.forward.to_array());
    AiCar::place(
        &model.model,
        Role::Traffic,
        nav,
        Spawn { position, heading: forward.y.atan2(forward.x) },
        Start { speed: 10.0, stagger, cruise: Cruise::patrol(35.0 * 0.44704, 55.0 * 0.44704), patrol: true },
        &WorldGround { collision, surfaces },
    )
}

fn course(
    world: &TrafficWorld,
    model: &CopModel,
    collision: &CollisionWorld,
    surfaces: &SurfaceTable,
    quantile: usize,
) -> ([AiCar; 2], RoadNav, f32) {
    let filter = SegmentFilter { traffic: true, cop: true, no_decision: true };
    let mut candidates = Vec::new();
    for (segment_id, segment) in world.network.segments.iter().enumerate() {
        if !filter.allows(segment) || segment.length < 40.0 {
            continue;
        }
        let profile = world.network.profile_at(segment_id as u16, segment.nodes[0]);
        for (lane, zone) in profile.zones.iter().enumerate() {
            if zone.kind != zone::TRAFFIC || zone.width < 2.0 * model.model.physics.spec.dimension.x {
                continue;
            }
            for t in [0.15, 0.4, 0.65] {
                let mut nav = RoadNav::traffic();
                nav.init_in_lane(&world.network, segment_id as u16, lane, t);
                let at = space::to_render(nav.position.to_array());
                if (at.x - LOADED_START[0]).hypot(at.y - LOADED_START[1]) > 220.0 {
                    continue;
                }
                let mut ahead = nav.clone();
                let mut rng = SplitMix(53_012);
                let mut turn = 0.0_f32;
                let mut supported = true;
                for _ in 0..36 {
                    let previous = ahead.forward;
                    ahead.advance_with_lookahead(&world.network, 4.0, Vec3::ZERO, 0.0, &mut rng);
                    turn += previous.dot(ahead.forward).clamp(-1.0, 1.0).acos();
                    let at = space::to_render(ahead.position.to_array());
                    let road = crate::scenes::world::road::probe(collision, at.x, at.y, at.z + 10.0);
                    if ahead.dead_end || road.is_none_or(|surface| !surface.road) {
                        supported = false;
                        break;
                    }
                }
                // This is coverage selection, not an original-game steering tolerance.
                if supported && turn >= 15.0_f32.to_radians() {
                    candidates.push((turn, nav));
                }
            }
        }
    }
    candidates.sort_by(|a, b| a.0.total_cmp(&b.0));
    let candidate_count = candidates.len();
    let mut eligible = Vec::new();
    let mut segments = std::collections::HashSet::new();
    for (turn, nav) in candidates {
        let mut front = nav.clone();
        front.advance_with_lookahead(&world.network, 14.0, Vec3::ZERO, 0.0, &mut SplitMix(53_012));
        let Some(rear) = place(model, nav.clone(), 0, collision, surfaces) else { continue };
        let Some(front) = place(model, front, 4, collision, surfaces) else { continue };
        let mut target_nav = nav.clone();
        target_nav.advance_with_lookahead(&world.network, TARGET_LEAD, Vec3::ZERO, 0.0, &mut SplitMix(53_012));
        let target = target(&target_nav, TARGET_SPEED);
        let cast = crate::scenes::world::drive::sight::sight_cast(collision);
        if [&rear, &front]
            .iter()
            .any(|car| cast(car.physics_position() + Vec3::Y * 0.5, target.position + Vec3::Y * 0.5).is_some())
        {
            continue;
        }
        if !segments.insert(nav.segment) {
            continue;
        }
        eligible.push((turn, nav));
    }
    assert!(
        eligible.len() >= 3,
        "three distinct resident curved segments must support visible physical patrols: {} eligible from {candidate_count} candidates",
        eligible.len()
    );
    let selected = quantile * (eligible.len() - 1) / 2;
    let (turn, nav) = eligible.swap_remove(selected);
    let mut front = nav.clone();
    front.advance_with_lookahead(&world.network, 14.0, Vec3::ZERO, 0.0, &mut SplitMix(53_012));
    let rear = place(model, nav.clone(), 0, collision, surfaces).expect("selected rear placement was checked");
    let front = place(model, front, 4, collision, surfaces).expect("selected front placement was checked");
    eprintln!(
        "installed curved encounter: quantile={quantile}/2 eligible_segments={} segment={} lane={} t={} selected_turn_degrees={:.1} cop_positions={:?}",
        eligible.len() + 1,
        nav.segment,
        nav.lane,
        nav.t,
        turn.to_degrees(),
        [rear.physics_position(), front.physics_position()]
    );
    ([rear, front], nav, turn)
}

#[derive(Default, Debug)]
struct Motion {
    along: f32,
    forward_speed: f32,
    gas: f32,
    brake: f32,
    full_steer: u32,
    outside: u32,
    outside_run: u32,
    max_outside_run: u32,
    max_edge_excess: f32,
    wall_stall: u32,
    max_wall_stall: u32,
}

impl Motion {
    fn sample(&mut self, world: &TrafficWorld, car: &AiCar, collision: &CollisionWorld) {
        let body = car.body();
        let mut road =
            RoadNav::new(NavKind::Direction, LaneType::Cop, SegmentFilter { cop: true, ..Default::default() });
        assert!(
            road.init_at_point(&world.network, &world.index, body.position, car.physics_forward(), false),
            "a physical pursuit unit must retain a nearby road: {}",
            diagnostic(world)
        );
        let along = body.velocity.dot(road.forward);
        self.along += along * STEP;
        self.forward_speed += car.speed();
        let controls = car.controls();
        self.gas += controls.gas;
        self.brake += controls.brake;
        self.full_steer += u32::from(controls.steer.abs() > 0.95);
        let cross = road.road_cross_section(&world.network);
        let lateral = cross.lateral(Vec2::new(body.position.x, body.position.z));
        let excess = (cross.left_offset - lateral).max(lateral - cross.right_offset).max(0.0);
        self.max_edge_excess = self.max_edge_excess.max(excess);
        self.outside += u32::from(excess > body.half_width);
        self.outside_run = match excess > body.half_width {
            true => self.outside_run + 1,
            false => 0,
        };
        self.max_outside_run = self.max_outside_run.max(self.outside_run);
        let from = body.position + Vec3::Y * 0.5;
        let to = from + car.physics_forward() * (body.half_length + 1.0);
        let wall = collision
            .ray_cast(from.to_array(), to.to_array(), &RayOptions::default())
            .is_some_and(|hit| hit.normal[1].abs() < 0.5);
        self.wall_stall = match wall && car.speed().abs() < TARGET_SPEED * 0.25 {
            true => self.wall_stall + 1,
            false => 0,
        };
        self.max_wall_stall = self.max_wall_stall.max(self.wall_stall);
    }
}

fn encounter(heat: u32, quantile: usize) {
    let (mut scene, mut renderer, mut world) = open();
    let model = world.cop_models.iter().find(|model| model.model.name.eq_ignore_ascii_case("copmidsize")).unwrap();
    let performance = blackbox_vehicle::performance::measure_with_integration(
        &model.model.physics.spec,
        model.model.physics.tire_integration,
    );
    let (cars, mut route, selected_turn) =
        course(&world, model, scene.residency.collision(), &scene.physics.surfaces, quantile);
    let identities = cars.each_ref().map(|car| car.audio_emitter().id);
    world.cars.extend(cars);
    let mut rng = SplitMix(53_012);
    route.advance_with_lookahead(&world.network, TARGET_LEAD, Vec3::ZERO, 0.0, &mut rng);
    world.player_position = Some(target(&route, TARGET_SPEED).position);
    let mut rows = nfsmw_data::pursuit::heat_rows(scene.physics.database());
    for row in &mut rows {
        // Keep model and count matched across heat cases to isolate physical
        // steering/tuning. This does not test the real higher-heat composition.
        row.cops = vec![nfsmw_data::pursuit::CopRecord { name: "copmidsize".into(), count: 2, chance: 100 }];
    }
    world.set_career_bin(Some(16));
    // Explicit development start raises the range, with the stock career growth
    // clock keeping this short physical encounter at the selected heat.
    world.police.set_forced_pursuit(true);
    world.start_pursuit(heat, rows, performance);
    assert_eq!(world.police.level(), Some(heat));
    let mut props = PropWorld::default();
    let mut motion = [Motion::default(), Motion::default()];
    let frames = (SECONDS / STEP).round() as u32;
    for frame in 0..frames {
        route.advance_with_lookahead(&world.network, TARGET_SPEED * STEP, Vec3::ZERO, 0.0, &mut rng);
        assert!(!route.dead_end, "curved encounter target must remain on the selected through route");
        tick(&mut world, target(&route, TARGET_SPEED), &mut scene, &mut renderer, &mut props);
        for (index, identity) in identities.iter().enumerate() {
            let car = world
                .cars
                .iter()
                .find(|car| car.audio_emitter().id == *identity)
                .expect("each original patrol must remain in the encounter");
            motion[index].sample(&world, car, scene.residency.collision());
        }
        if frame.is_multiple_of(60) {
            eprintln!(
                "curved encounter heat={heat} quantile={quantile}/2 t={:.2}s target={:?} motion={motion:?}; {}",
                frame as f32 * STEP,
                route.position,
                diagnostic(&world)
            );
        }
    }
    assert_eq!(world.police.level(), Some(heat));
    for (index, sample) in motion.iter().enumerate() {
        // An engineering sanity bound: each cop must make at least one quarter
        // of the moving target's progress. This is not an OG numeric parity claim.
        assert!(
            sample.along > TARGET_SPEED * SECONDS * 0.25,
            "unit {index} must make useful road progress through the encounter: selected_turn={selected_turn} motion={motion:?}; {}",
            diagnostic(&world)
        );
        assert!(
            sample.forward_speed / frames as f32 > TARGET_SPEED * 0.25,
            "unit {index} must retain useful physical speed: motion={motion:?}; {}",
            diagnostic(&world)
        );
        assert!(
            sample.max_wall_stall < (1.0 / STEP) as u32,
            "unit {index} must not spend a continuous second stopped against a wall: motion={motion:?}; {}",
            diagnostic(&world)
        );
        assert!(
            sample.max_outside_run < (1.0 / STEP) as u32,
            "unit {index} must not remain beyond the drivable corridor by its half-width for a continuous second: outside_seconds={} longest_seconds={} motion={motion:?}; {}",
            sample.outside as f32 * STEP,
            sample.max_outside_run as f32 * STEP,
            diagnostic(&world)
        );
    }
    scene.close(&mut renderer);
}

#[test]
#[ignore = "needs installed city collision, road network and patrol geometry selected by NFSMW_GAME_DIR"]
fn installed_two_patrols_keep_moving_through_a_curved_encounter() {
    encounter(1, 0);
}

#[test]
#[ignore = "needs installed city collision, road network and patrol geometry selected by NFSMW_GAME_DIR"]
fn installed_two_patrols_follow_a_median_curve_at_heat_three() {
    encounter(3, 1);
}

#[test]
#[ignore = "needs installed city collision, road network and patrol geometry selected by NFSMW_GAME_DIR"]
fn installed_two_patrols_follow_a_stronger_curve_at_heat_five() {
    encounter(5, 2);
}

#[test]
#[ignore = "needs installed city collision, road network and patrol geometry selected by NFSMW_GAME_DIR"]
fn installed_two_patrols_follow_a_stronger_curve_at_heat_six() {
    encounter(6, 2);
}
