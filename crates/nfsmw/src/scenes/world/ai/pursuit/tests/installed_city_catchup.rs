//! Opt-in private target traces exercise installed city roads, visibility and production police physics.
//! No captured game data or default trace path belongs in this source.

use super::*;
use crate::scenes::world::{WorldScene, drive::STEP, props::PropWorld};
use crate::viewer::Scene;
use serde_json::Value;

#[derive(Clone, Copy)]
pub(super) struct Frame {
    time: f32,
    pub(super) body: Body,
    speed: f32,
}

pub(super) struct Trace {
    pub(super) frames: Vec<Frame>,
    pub(super) identity: String,
}

fn number(value: &Value) -> f32 {
    let value = value.as_f64().expect("trace scalar must be a number") as f32;
    assert!(value.is_finite(), "trace scalar must be finite and fit f32");
    value
}

fn vector(value: &Value) -> Vec3 {
    let values = value.as_array().expect("trace vector must be an array");
    assert_eq!(values.len(), 3, "trace position/velocity uses physics XYZ");
    Vec3::new(number(&values[0]), number(&values[1]), number(&values[2]))
}

impl Trace {
    pub(super) fn load() -> Self {
        let path = std::env::var_os("NFSMW_PURSUIT_TARGET_TRACE")
            .expect("explicit NFSMW_PURSUIT_TARGET_TRACE is required; no built-in capture");
        let size = std::fs::metadata(&path).expect("target trace metadata").len();
        assert!(size <= 16 * 1024 * 1024, "target trace must fit the bounded test input");
        let value: Value = serde_json::from_slice(&std::fs::read(path).expect("target trace read"))
            .expect("target trace must be valid JSON");
        assert_eq!(value["heat"].as_u64(), Some(5), "this acceptance uses the actual heat-5 fleet");
        let rows = value["frames"].as_array().expect("trace frames");
        assert!((2..=20_000).contains(&rows.len()), "trace frame count is bounded");
        let frames: Vec<_> = rows
            .iter()
            .map(|row| {
                let f = row["forward"].as_array().expect("trace forward uses physics XZ");
                assert_eq!(f.len(), 2);
                let forward = Vec2::new(number(&f[0]), number(&f[1]));
                assert!((0.99..=1.01).contains(&forward.length()), "trace planar forward must be unit length");
                let speed = number(&row["speed"]);
                assert!(speed >= 0.0, "recorded target speed cannot be negative");
                Frame {
                    time: number(&row["time"]),
                    body: Body {
                        position: vector(&row["position"]),
                        velocity: vector(&row["velocity"]),
                        forward: forward.normalize(),
                        half_width: 1.0,
                        half_length: 2.3,
                    },
                    speed,
                }
            })
            .collect();
        assert!(frames[0].time.abs() < 1e-5, "relative trace time must begin at zero");
        assert!((30.0..=60.0).contains(&frames.last().unwrap().time), "sustained trace must span 30..60 s");
        for pair in frames.windows(2) {
            let dt = pair[1].time - pair[0].time;
            assert!(dt > 0.0 && dt <= 0.2, "trace has a nonadvancing timestamp or sampling gap: dt={dt}");
            assert!(pair[0].body.forward.dot(pair[1].body.forward) > 0.85, "trace heading discontinuity");
            assert!(
                pair[0].body.position.distance(pair[1].body.position)
                    <= 2.0 * pair[0].speed.max(pair[1].speed) * dt + 2.0,
                "trace position discontinuity"
            );
        }
        let identity = format!(
            "source={} samples_sha256={} start_sim={} frames={} duration={:.3}",
            value["source_capture"],
            value["source_sha256"],
            value["start_sim_time"],
            rows.len(),
            frames.last().unwrap().time
        );
        Self { frames, identity }
    }

    fn at(&self, time: f32) -> Frame {
        let end = self.frames.partition_point(|frame| frame.time < time).clamp(1, self.frames.len() - 1);
        let (a, b) = (self.frames[end - 1], self.frames[end]);
        let t = ((time - a.time) / (b.time - a.time)).clamp(0.0, 1.0);
        Frame {
            time,
            body: Body {
                position: a.body.position.lerp(b.body.position, t),
                velocity: a.body.velocity.lerp(b.body.velocity, t),
                forward: a.body.forward.lerp(b.body.forward, t).normalize(),
                ..a.body
            },
            speed: a.speed + (b.speed - a.speed) * t,
        }
    }
}

fn stream(scene: &mut WorldScene, renderer: &mut NullBackend, position: Vec3) {
    let at = space::to_render(position.to_array());
    for _ in 0..2000 {
        scene.residency.update_centres(renderer, &[[at.x, at.y]]);
        if scene.residency.complete()
            && scene.residency.collision().resident_near(position.to_array(), 30.0) != Some(false)
        {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!("trace setup failed: installed collision did not become resident at {position:?}");
}

pub(super) fn road_cursor(world: &TrafficWorld, body: Body) -> RoadNav {
    let mut nav = RoadNav::new(NavKind::Direction, LaneType::Cop, SegmentFilter { cop: true, ..Default::default() });
    let forward = Vec3::new(body.forward.x, 0.0, body.forward.y);
    assert!(
        nav.init_at_point(&world.network, &world.index, body.position, forward, false),
        "trace setup: no nearby cop road"
    );
    let difference = body.position - nav.position;
    assert!(
        difference.x.hypot(difference.z) < 30.0 && difference.y.abs() < 8.0,
        "trace setup: target does not fit installed road; target={:?} nav={:?}",
        body.position,
        nav.position
    );
    nav
}

#[derive(Debug)]
struct LaneFit {
    segment: u16,
    lane: usize,
    t: f32,
    edge_gap: f32,
    height: f32,
}

fn physical_lane_fit(world: &TrafficWorld, point: Vec3) -> Option<LaneFit> {
    let filter = SegmentFilter { cop: true, ..Default::default() };
    let mask = LaneType::Cop.drivable_mask();
    let mut best: Option<(f32, LaneFit)> = None;
    for segment in world.index.near(point, blackbox_roads::SEARCH_RADIUS) {
        let seg = world.network.segment(segment);
        if !filter.allows(seg) {
            continue;
        }
        let from = blackbox_roads::travel_profile(&world.network, segment, 1, false);
        let to = blackbox_roads::travel_profile(&world.network, segment, 1, true);
        if from.zones.is_empty() || to.zones.is_empty() {
            continue;
        }
        for lane in 0..from.zones.len().max(to.zones.len()) {
            let (a, b) = (from.zones[lane.min(from.zones.len() - 1)], to.zones[lane.min(to.zones.len() - 1)]);
            if !a.in_mask(mask) || !b.in_mask(mask) {
                continue;
            }
            let line = blackbox_roads::lane_line(&world.network, segment, 1, lane);
            let t = line.closest_t(point, seg.length);
            let delta = point - line.position(t);
            let forward = Vec2::new(line.tangent(t).x, line.tangent(t).z).normalize_or_zero();
            let half_width = (a.width * (1.0 - t) + b.width * t) * 0.5;
            let horizontal = Vec2::new(delta.x, delta.z);
            let lateral = horizontal.dot(Vec2::new(forward.y, -forward.x)).abs();
            // Preserve longitudinal endpoint distance: road width only absorbs lateral displacement.
            let edge_gap = (lateral - half_width).max(0.0).hypot(horizontal.dot(forward));
            let distance = edge_gap.hypot(delta.y);
            if best.as_ref().is_none_or(|(old, _)| distance < *old) {
                best = Some((distance, LaneFit { segment, lane, t, edge_gap, height: delta.y.abs() }));
            }
        }
    }
    best.map(|(_, fit)| fit)
}

fn validate_geometry(trace: &Trace, world: &TrafficWorld, scene: &mut WorldScene, renderer: &mut NullBackend) {
    let mut reasons = std::collections::BTreeMap::<&str, usize>::new();
    let mut raw_materials = std::collections::BTreeMap::<String, usize>::new();
    let mut drivable_materials = std::collections::BTreeMap::<String, usize>::new();
    let (mut min_normal, mut max_normal) = (f32::INFINITY, f32::NEG_INFINITY);
    let (mut min_offset, mut max_offset) = (f32::INFINITY, f32::NEG_INFINITY);
    let (mut max_nav_xy, mut max_nav_y) = (0.0_f32, 0.0_f32);
    let (mut max_lane_gap, mut max_lane_y) = (0.0_f32, 0.0_f32);
    let (mut max_cop_centre, mut max_any_centre) = (0.0_f32, 0.0_f32);
    let mut failures = 0;
    let mut association_rows = 0;
    for frame in &trace.frames {
        stream(scene, renderer, frame.body.position);
        let mut problems = Vec::new();
        let p = frame.body.position;
        let collision = scene.residency.collision();
        let raw = collision.ground(p.x, p.z, p.y + 2.0, p.y - 158.0);
        let drivable = crate::scenes::world::drive::drivable_ground_below(collision, p.x, p.z, p.y + 2.0, p.y - 158.0);
        let mut nav =
            RoadNav::new(NavKind::Direction, LaneType::Cop, SegmentFilter { cop: true, ..Default::default() });
        let direction = Vec3::new(frame.body.forward.x, 0.0, frame.body.forward.y);
        let located = nav.init_at_point(&world.network, &world.index, p, direction, false);
        let nav_delta = p - nav.position;
        if located {
            let xy = nav_delta.x.hypot(nav_delta.z);
            max_nav_xy = max_nav_xy.max(xy);
            max_nav_y = max_nav_y.max(nav_delta.y.abs());
        }
        let cop_filter = SegmentFilter { cop: true, ..Default::default() };
        let cop_centre = blackbox_roads::closest_segment(&world.network, &world.index, p, direction, 0.0, cop_filter);
        let any_centre =
            blackbox_roads::closest_segment(&world.network, &world.index, p, direction, 0.0, SegmentFilter::default());
        max_cop_centre = max_cop_centre.max(cop_centre.map_or(f32::INFINITY, |hit| hit.point.distance(p)));
        max_any_centre = max_any_centre.max(any_centre.map_or(f32::INFINITY, |hit| hit.point.distance(p)));
        let lane = physical_lane_fit(world, p);
        if lane.is_none() {
            problems.push("no_physical_cop_lane");
        }
        if let Some(fit) = &lane {
            max_lane_gap = max_lane_gap.max(fit.edge_gap);
            max_lane_y = max_lane_y.max(fit.height);
            if fit.edge_gap >= 30.0 || fit.height >= 8.0 {
                problems.push("physical_lane_distance");
            }
            if (!located || nav_delta.x.hypot(nav_delta.z) >= 30.0) && association_rows < 12 {
                let seg = world.network.segment(fit.segment);
                eprintln!(
                    "CITY_TRACE_ROAD_ASSOC t={:.3} target={p:?} weighted=({located},segment={},lane={},t={},flags={:#06x},delta={nav_delta:?}) nearest_cop_centre={cop_centre:?} nearest_any_centre={any_centre:?} physical_lane=(segment={},lane={},t={:.3},flags={:#06x},edge_gap={:.3},height={:.3})",
                    frame.time,
                    nav.segment,
                    nav.lane,
                    nav.t,
                    world.network.segment(nav.segment).flags,
                    fit.segment,
                    fit.lane,
                    fit.t,
                    seg.flags,
                    fit.edge_gap,
                    fit.height
                );
                association_rows += 1;
            }
        }
        if let Some(hit) = raw {
            let offset = p.y - hit.point[1];
            min_offset = min_offset.min(offset);
            max_offset = max_offset.max(offset);
            min_normal = min_normal.min(hit.normal[1]);
            max_normal = max_normal.max(hit.normal[1]);
            *raw_materials
                .entry(format!("{:08x}:known={}", hit.surface_hash, scene.physics.surfaces.knows(hit.surface_hash)))
                .or_default() += 1;
        }
        if drivable.is_none() {
            problems.push("no_drivable_ground");
        }
        if let Some(hit) = drivable {
            *drivable_materials
                .entry(format!("{:08x}:known={}", hit.surface_hash, scene.physics.surfaces.knows(hit.surface_hash)))
                .or_default() += 1;
            let grip = scene.physics.surfaces.grip(hit.surface_hash);
            if !scene.physics.surfaces.knows(hit.surface_hash) || grip.drive <= 0.0 || grip.lateral <= 0.0 {
                problems.push("unknown_or_nondriveable_material");
            }
            if hit.normal[1] <= 0.97 {
                problems.push("drivable_normal_not_flat");
            }
            if !(-0.5..=4.0).contains(&(p.y - hit.point[1])) {
                problems.push("drivable_body_ground_height");
            }
        }
        if raw != drivable {
            *reasons.entry("raw_vs_drivable_hit_difference_diagnostic").or_default() += 1;
        }
        if problems.is_empty() {
            continue;
        }
        failures += 1;
        for problem in &problems {
            *reasons.entry(problem).or_default() += 1;
        }
        if failures <= 12 {
            eprintln!(
                "CITY_TRACE_GEOMETRY_FAILURE t={:.3} target={p:?} reasons={problems:?} nav_located={located} nav_delta={nav_delta:?} raw={raw:?} raw_grip={:?} drivable={drivable:?} drivable_grip={:?}",
                frame.time,
                raw.map(|hit| (
                    scene.physics.surfaces.knows(hit.surface_hash),
                    scene.physics.surfaces.grip(hit.surface_hash)
                )),
                drivable.map(|hit| (
                    scene.physics.surfaces.knows(hit.surface_hash),
                    scene.physics.surfaces.grip(hit.surface_hash)
                )),
            );
        }
    }
    eprintln!(
        "CITY_TRACE_GEOMETRY samples={} failed={failures} reasons={reasons:?} raw_materials={raw_materials:?} drivable_materials={drivable_materials:?} raw_normal_y={min_normal:.6}..{max_normal:.6} raw_body_offset={min_offset:.3}..{max_offset:.3} weighted_max_xy={max_nav_xy:.3} weighted_max_y={max_nav_y:.3} nearest_cop_centre_max={max_cop_centre:.3} nearest_any_centre_max={max_any_centre:.3} physical_lane_edge_max={max_lane_gap:.3} physical_lane_y_max={max_lane_y:.3}",
        trace.frames.len(),
    );
    assert_eq!(failures, 0, "trace setup: installed geometry preflight failed; chase scoring is blocked");
}

#[derive(Debug)]
struct Result {
    initial_gap: f32,
    closest_gap: f32,
    final_gap: Option<f32>,
    original_max_speed: f32,
    original_nos_ticks: u32,
    original_present_ticks: u32,
    late_ticks: u32,
    late_matched_ticks: u32,
    late_contact_ticks: u32,
}

#[test]
#[ignore = "needs installed city and explicit private NFSMW_PURSUIT_TARGET_TRACE; no captured data is shipped"]
fn installed_city_heat_five_closes_and_retains_a_recorded_fast_target() {
    let trace = Trace::load();
    let first = trace.frames[0];
    let at = space::to_render(first.body.position.to_array());
    let (mut scene, mut renderer, world) = super::installed_route::open_at([at.x, at.y]);
    let rows = nfsmw_data::pursuit::heat_rows(scene.physics.database());
    let row = rows.get(4).expect("installed heat-5 row");
    let dominant = nfsmw_data::pursuit::ground_cops(row)
        .max_by_key(|record| record.count.saturating_mul(record.chance))
        .expect("heat-5 row needs an ordinary ground cop")
        .name
        .clone();
    let names: Vec<_> = nfsmw_data::pursuit::ground_cops(row).map(|record| record.name.clone()).collect();
    scene.traffic = Some(world);
    crate::scenes::world::ai::models::load_cop_models(&mut scene, &mut renderer, &names).unwrap();
    crate::scenes::world::ai::models::load_requested(&mut scene, &mut renderer, false, true);
    let mut world = scene.traffic.take().unwrap();
    assert!(names.iter().all(|name| world.has_cop_model(name)), "all installed ordinary heat-5 variants must load");
    validate_geometry(&trace, &world, &mut scene, &mut renderer);
    let player = scene.physics.car("bmwm3gtr").expect("installed BMW target performance");
    let performance = nfsmw_data::car::physics::measured::ai_performance_of_spec(&player.spec, player.tire_integration);
    let boost = match player.spec.nos.present() {
        true => player.spec.nos.torque_boost,
        false => 0.0,
    };
    let mut nav = super::installed_city_placement::trailing(&world, road_cursor(&world, first.body), first.body);
    super::installed_city_placement::prepare_cursor(&world, &dominant, &mut nav);
    stream(&mut scene, &mut renderer, nav.position);
    let spot = CopSpot {
        at: nav.position,
        target: first.body.position,
        speed: first.speed * 0.9,
        player: performance,
        band: None,
    };
    assert!(
        world.spawn_cop_on_road(&dominant, spot, nav.clone(), scene.residency.collision(), &scene.physics.surfaces),
        "trace setup: rolling cop placement failed"
    );
    let original = world.cars.last().unwrap();
    super::installed_city_placement::assert_handoff(original, &nav);
    let identity = original.audio_emitter().id;
    let initial_gap = original.physics_position().distance(first.body.position);
    let trailing = (first.body.position - original.physics_position()).dot(Vec3::new(
        first.body.forward.x,
        0.0,
        first.body.forward.y,
    ));
    eprintln!(
        "CITY_TRACE_PLACEMENT selected_nav=({},{},{:.6},{:?},{:?}) physical=({:?},{:?}) velocity={:?} actual_speed={:.3} requested_start={:.3} radial_gap={initial_gap:.3} signed_gap={trailing:.3} {}; {}",
        nav.segment,
        nav.lane,
        nav.t,
        nav.position,
        nav.forward,
        original.physics_position(),
        original.physics_forward(),
        original.body().velocity,
        original.speed(),
        spot.speed,
        original.telemetry_debug(),
        original.debug(),
    );
    assert!(
        (60.0..=120.0).contains(&initial_gap) && trailing > 50.0,
        "trace setup: rolling cop must begin behind target, gap={initial_gap}, along={trailing}"
    );
    let model = world.cop_models.iter().find(|model| model.model.name.eq_ignore_ascii_case(&dominant)).unwrap();
    eprintln!(
        "CITY_TRACE {} install={:?} model={dominant} id={identity} initial_gap={initial_gap:.2} performance={:?} ai={:?} nos={:?} player_performance={performance:?} player_nos_boost={boost}",
        trace.identity,
        std::env::var_os("NFSMW_GAME_DIR"),
        model.car.performance,
        model.car.ai,
        model.model.physics.spec.nos
    );
    world.set_career_bin(Some(16));
    world.police.set_forced_pursuit(true);
    world.player_position = Some(first.body.position);
    world.start_pursuit(5, rows, performance);
    let mut props = PropWorld::default();
    let duration = trace.frames.last().unwrap().time;
    let ticks = (duration / STEP).floor() as u32;
    let mut result = Result {
        initial_gap,
        closest_gap: initial_gap,
        final_gap: None,
        original_max_speed: 0.0,
        original_nos_ticks: 0,
        original_present_ticks: 0,
        late_ticks: 0,
        late_matched_ticks: 0,
        late_contact_ticks: 0,
    };
    for tick in 0..=ticks {
        let frame = trace.at(tick as f32 * STEP);
        super::installed_route::tick_with_boost(&mut world, frame.body, &mut scene, &mut renderer, &mut props, boost);
        assert_eq!(world.police.level(), Some(5), "fixture must retain forced heat5 context");
        let late = frame.time >= duration - 10.0;
        result.late_ticks += u32::from(late);
        result.late_contact_ticks += u32::from(late && world.police.has_contact());
        result.final_gap = None;
        if let Some(car) = world.cars.iter().find(|car| car.audio_emitter().id == identity) {
            let gap = car.physics_position().distance(frame.body.position);
            result.original_present_ticks += 1;
            result.closest_gap = result.closest_gap.min(gap);
            result.final_gap = Some(gap);
            result.original_max_speed = result.original_max_speed.max(car.speed());
            result.original_nos_ticks += u32::from(car.controls().nos);
            result.late_matched_ticks += u32::from(
                late && car.is_cop()
                    && !car.is_fleeing()
                    && !car.is_searching()
                    && gap < 150.0
                    && car.speed() >= frame.speed * 0.75
                    && car.sight_age() < 1.0,
            );
        }
        if tick.is_multiple_of(60) {
            let forward = Vec3::new(frame.body.forward.x, 0.0, frame.body.forward.y);
            let units: Vec<_> = world.cars.iter().filter(|car| car.is_cop()).map(|car| {
                let delta = frame.body.position - car.physics_position();
                let (_, wanted) = car.drive_request();
                format!("{} id={} gap={:.1} along={:.1} lateral={:.1} actual={:.1} wanted={wanted:.1} controls={:?} sight_active={} sight_age={:.2} search={} flee={} {}; {}", car.name, car.audio_emitter().id, delta.length(), delta.dot(forward), delta.cross(forward).y.abs(), car.speed(), car.controls(), car.sight_active(), car.sight_age(), car.is_searching(), car.is_fleeing(), car.telemetry_debug(), car.debug())
            }).collect();
            eprintln!(
                "CITY_CATCHUP t={:.2} target_speed={:.2} target={:?} result={result:?} {}; {}",
                frame.time,
                frame.speed,
                frame.body.position,
                world.pursuit_status(),
                units.join(" | ")
            );
        }
    }
    scene.close(&mut renderer);
    eprintln!(
        "CITY_CATCHUP_SUMMARY late_original_engagement_fraction={:.3} late_contact_fraction={:.3} original_present_fraction={:.3} {result:?}",
        result.late_matched_ticks as f32 / result.late_ticks as f32,
        result.late_contact_ticks as f32 / result.late_ticks as f32,
        result.original_present_ticks as f32 / (ticks + 1) as f32,
    );
    // Engineering regression criteria for this rolling established-pursuer setup,
    // not a fitted original-game speed curve or a requirement on stationary arrivals.
    assert!(
        result.closest_gap <= result.initial_gap - 20.0,
        "rolling original unit must close a meaningful deficit: {result:?}"
    );
    assert!(
        result.late_matched_ticks * 3 >= result.late_ticks,
        "same physical unit must retain late moving engagement and recent real sight: {result:?}"
    );
    assert!(
        result.late_contact_ticks * 2 >= result.late_ticks,
        "actual pursuit must retain contact late in the route: {result:?}"
    );
}
