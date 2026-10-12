//! Separate driver input, real road geometry and upright spawn placement on a steep highway.

use super::*;
use crate::scenes::world::ai::car::{AiCar, Ctx, Role, Start};
use crate::scenes::world::ai::traffic::Cruise;
use blackbox_roads::{LaneType, NavKind, RoadNav, SegmentFilter, SegmentIndex, SplitMix};
use blackbox_vehicle::ground::{Ground, GroundHit, SurfaceGrip};
use blackbox_vehicle::rigid_body::BOX_CORNERS;
use blackbox_vehicle::suspension::{Geometry, compression, probe};

struct Plane {
    point: Vec3,
    normal: Vec3,
    surface: SurfaceGrip,
}

type Support = (f32, [Option<(f32, f32)>; 4]);

impl Ground for Plane {
    fn hit(&self, origin: Vec3, direction: Vec3, limit: f32) -> Option<GroundHit> {
        let denominator = self.normal.dot(direction);
        if denominator >= -1e-6 {
            return None;
        }
        let distance = self.normal.dot(self.point - origin) / denominator;
        (distance >= 0.0 && distance <= limit).then_some(GroundHit {
            distance,
            normal: self.normal,
            surface: self.surface,
        })
    }
}

#[derive(Default)]
struct Motion {
    vertical: [(f32, usize); 2],
    angular: [(f32, usize); 2],
    min_up: [f32; 2],
    gap: f32,
}

impl Motion {
    fn new() -> Self {
        Self { min_up: [1.0; 2], ..Default::default() }
    }

    fn record(&mut self, a: &CarSim, b: &CarSim, joint: &BallJoint, tick: usize) -> bool {
        let mut peak = false;
        for (i, sim) in [a, b].into_iter().enumerate() {
            assert!(sim.is_finite(), "body {i} tick {tick}");
            let vertical = sim.body().linear_velocity.y.abs();
            let angular = sim.body().angular_velocity.length();
            if vertical > self.vertical[i].0 {
                self.vertical[i] = (vertical, tick);
                peak = true;
            }
            if angular > self.angular[i].0 {
                self.angular[i] = (angular, tick);
            }
            self.min_up[i] = self.min_up[i].min(sim.up().y);
        }
        self.gap = self.gap.max(joint.gap(a.body(), b.body()));
        peak
    }

    fn print(&self, label: &str) {
        eprintln!(
            "{label}: max vy/tick {:?}, angular/tick {:?}, min up {:?}, max joint gap {:.5}",
            self.vertical, self.angular, self.min_up, self.gap
        );
    }
}

fn support(sim: &CarSim, model: &TrafficModel, ground: &dyn Ground) -> Support {
    let body = sim.body();
    let half = body.dimension() + body.spec().collision_box_pad;
    let depth = BOX_CORNERS
        .into_iter()
        .filter_map(|corner| body.ground_contact(body.position + body.rotation() * (half * corner), ground, STEP))
        .map(|contact| contact.depth)
        .fold(0.0, f32::max);
    let spec = &model.physics.spec;
    let geometry = Geometry::new(&spec.chassis, &spec.tires, spec.dimension.y);
    let wheels = std::array::from_fn(|i| {
        let contact = probe(ground, body.position + body.rotation() * geometry.arms[i], spec.dimension.y)?;
        let axle = spec.chassis.axle(i / 2, 0.0);
        let raw = axle.ride * contact.normal.dot(sim.up()).clamp(0.0, 1.0) + contact.penetration;
        let value = compression(axle.ride, axle.travel, contact.normal.dot(sim.up()), contact.penetration);
        Some((raw, value.lift_need))
    });
    (depth, wheels)
}

fn snapshot(label: &str, phase: &str, tick: usize, a: &CarSim, b: &CarSim, data: &PhysicsData) {
    eprintln!(
        "{label} {phase} tick {tick}: pos {:?}, up {:?}, vy {:?}, angular {:?}, wheels {:?}, compression {:?}, load {:?}",
        [a.body().position, b.body().position],
        [a.up(), b.up()],
        [a.body().linear_velocity.y, b.body().linear_velocity.y],
        [a.body().angular_velocity, b.body().angular_velocity],
        [a.telemetry().wheels_on_ground, b.telemetry().wheels_on_ground],
        [a.wheel_sounds(&data.surfaces).map(|w| w.compression), b.wheel_sounds(&data.surfaces).map(|w| w.compression)],
        [a.wheel_sounds(&data.surfaces).map(|w| w.load), b.wheel_sounds(&data.surfaces).map(|w| w.load)]
    );
}

fn coast(
    model: &TrafficModel,
    spawn: Spawn,
    ground: &dyn Ground,
    data: &PhysicsData,
    label: &str,
    runtime: bool,
) -> [Support; 2] {
    let speed = Cruise::default().street * 0.75;
    let trailer_model = model.trailer.as_ref().unwrap();
    let mut a = CarSim::for_rig(model.physics.clone(), &model.rig);
    let mut b = CarSim::for_rig(trailer_model.physics.clone(), &trailer_model.rig);
    a.configure_ai();
    b.configure_trailer();
    let place = |sim: &mut CarSim, spawn| match runtime {
        true => sim.place_traffic_moving(ground, spawn, speed),
        false => sim.place_moving(ground, spawn, speed),
    };
    assert!(place(&mut a, spawn));
    let mut coupling = Coupling::new(&a, &b).unwrap();
    let back = coupling.joint.anchor_b.z - coupling.joint.anchor_a.z;
    let forward = Vec3::new(spawn.heading.cos(), spawn.heading.sin(), 0.0);
    assert!(place(&mut b, Spawn { position: spawn.position - forward * back, ..spawn }));
    coupling.align(&a, &mut b);
    let initial = [support(&a, model, ground), support(&b, trailer_model, ground)];
    eprintln!(
        "{label} initial: gap {:.5}, support tractor {:?}, trailer {:?}",
        coupling.gap(&a, &b),
        initial[0],
        initial[1]
    );
    snapshot(label, "placed", 0, &a, &b, data);
    let mut motion = Motion::new();
    for tick in 0..30 {
        a.step(&DriveInput::default(), ground, None);
        b.step(&DriveInput::default(), ground, None);
        if tick < 6 {
            snapshot(label, "before joint", tick, &a, &b, data);
        }
        coupling.update_hitch(&a, &b);
        coupling.resolve(&mut a, &mut b);
        let _ = motion.record(&a, &b, &coupling.joint, tick);
        if tick < 6 {
            snapshot(label, "after joint", tick, &a, &b, data);
        }
    }
    motion.print(label);
    assert!(coupling.is_hitched(), "{label} coast startup released its hitch");
    assert!(motion.gap < 0.1, "{label} coast hitch error");
    initial
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_highway_slope_startup_separates_placement_from_driver_input() {
    let path = std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR to run installed tests");
    let dir = GameDir::open(path).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let net = index.road_network.as_ref().unwrap();
    let segments = SegmentIndex::build(net);
    // This legal highway produced the largest no-contact load motion in the bounded driving probe.
    let curve = lane_line(net, 3189, 1, 4);
    let point = curve.position(0.3);
    let tangent = curve.tangent(0.3);
    let direction = space::to_render(tangent.to_array());
    let probe = Spawn { position: space::to_render(point.to_array()), heading: direction.y.atan2(direction.x) };
    let collision = collision(&dir, &index, probe);
    let mut data = PhysicsData::load(&dir).unwrap();
    data.set_sequences(index.sequences.clone());
    let ground = WorldGround { collision: &collision, surfaces: &data.surfaces };
    let mut nav = RoadNav::new(NavKind::Direction, LaneType::Racing, SegmentFilter::default());
    let mut rng = SplitMix(13);
    assert!(nav.init_at_point(net, &segments, point, tangent, false));
    assert!(nav.can_traffic_spawn(net, &mut rng));
    assert_eq!(nav.segment, 3189, "fixture must remain on the reproduced highway");
    let position = space::to_render(nav.position.to_array());
    let surface = road::probe(&collision, position.x, position.y, position.z + 10.0).unwrap();
    assert!(surface.road, "fixture must start on asphalt");
    let forward = space::to_render(nav.forward.to_array());
    let spawn =
        Spawn { position: Vec3::new(position.x, position.y, surface.height), heading: forward.y.atan2(forward.x) };
    let at = Vec3::from(space::to_physics(spawn.position));
    let hit = ground.hit(at + Vec3::Y * 2.0, Vec3::NEG_Y, 5.0).unwrap();
    let plane = Plane { point: at, normal: hit.normal, surface: hit.surface };
    let flat = FlatGround { height: at.y, surface: hit.surface };
    eprintln!("slope fixture: segment {} lane {}, spawn {spawn:?}, normal {:?}", nav.segment, nav.lane, hit.normal);
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    for name in ["semib", "semilog"] {
        let mut model = model(&dir, &data, name, &mut renderer);
        model.trailer = Some(Box::new(super::model(&dir, &data, &data.trailer_of(name).unwrap(), &mut renderer)));
        coast(&model, spawn, &ground, &data, &format!("{name} upright real-road coast"), false);
        coast(&model, spawn, &plane, &data, &format!("{name} upright local-plane coast"), false);
        coast(&model, spawn, &ground, &data, &format!("{name} runtime real-road coast"), true);
        coast(&model, spawn, &flat, &data, &format!("{name} runtime flat coast"), true);
        let initial = coast(&model, spawn, &plane, &data, &format!("{name} runtime local-plane coast"), true);
        for (body, (depth, wheels)) in initial.into_iter().enumerate() {
            assert!(depth <= 0.001, "{name} body {body}: startup box penetrates its support plane by {depth} m");
            assert!(wheels.iter().all(Option::is_some), "{name} body {body}: initial wheel support must be reachable");
            for (wheel, (raw, lift)) in wheels.into_iter().flatten().enumerate() {
                assert!(raw <= 0.001, "{name} body {body} wheel {wheel}: startup preloads the spring by {raw} m");
                assert!(
                    lift <= 0.001,
                    "{name} body {body} wheel {wheel}: startup is {lift} m beyond suspension travel"
                );
            }
        }
        let mut nav = nav.clone();
        nav.half_width = model.physics.spec.dimension.x;
        nav.enable_trail(net);
        nav.advance_with_lookahead(net, 30.0, Vec3::ZERO, 30.0, &mut rng);
        let start =
            Start { stagger: 0, speed: Cruise::default().street * 0.75, cruise: Cruise::default(), patrol: false };
        let mut car = AiCar::place(&model, Role::Traffic, nav, spawn, start, &ground).unwrap();
        let mut props = PropWorld::default();
        let mut motion = Motion::new();
        let mut world_hits = 0;
        let label = format!("{name} real-road driver");
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
            let (a, b) = car.sims_mut();
            let b = b.unwrap();
            for sim in [&*a, &*b] {
                world_hits += sim.events.as_ref().map_or(0, |events| events.pending_hits().len());
            }
            if tick < 12 || tick % 10 == 0 {
                snapshot(&label, "before joint", tick, a, b, &data);
            }
            car.update_events(&mut props, &data, spawn.position);
            car.sync_contacts();
            let joint = car.trailer().unwrap().coupling.joint;
            let (a, b) = car.sims_mut();
            let b = b.unwrap();
            let peak = motion.record(a, b, &joint, tick);
            if tick < 12 || tick % 10 == 0 {
                snapshot(&label, "after joint", tick, a, b, &data);
            }
            if peak && tick >= 12 {
                let asphalt = [a, b].map(|sim| {
                    let p = space::to_render(sim.body().position.to_array());
                    road::probe(&collision, p.x, p.y, p.z + 10.0)
                });
                eprintln!("{label} peak tick {tick}: road support {asphalt:?}");
            }
        }
        motion.print(&label);
        eprintln!("{label}: real world hits {world_hits}, fragments {}", props.fragment_count());
        assert_eq!(world_hits, 0, "diagnostic must remain free of real wall/prop contacts");
        assert_eq!(props.fragment_count(), 0, "diagnostic must not spontaneously release cargo");
        assert!(car.trailer().unwrap().is_hitched(), "driver diagnostic released the trailer");
        assert!(motion.gap < 0.1, "driver diagnostic joint error");
    }
}
