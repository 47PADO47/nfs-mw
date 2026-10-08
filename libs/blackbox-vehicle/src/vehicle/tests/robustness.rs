use super::*;
use crate::ground::{Ground, GroundHit, NoGround, SurfaceGrip};

/// A tiny deterministic generator (xorshift) so the tests need no dependencies.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }

    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next()
    }

    fn input(&mut self) -> InputState {
        InputState {
            throttle: if self.next() < 0.3 { 0.0 } else { self.range(0.0, 1.0) },
            brake: if self.next() < 0.6 { 0.0 } else { self.range(0.0, 1.0) },
            steer: self.range(-1.2, 1.2),
            handbrake: if self.next() < 0.8 { 0.0 } else { self.range(0.0, 1.0) },
            nos: self.next() < 0.3,
            shift_up: self.next() < 0.02,
            shift_down: self.next() < 0.02,
        }
    }
}

fn state_bits(v: &Vehicle) -> Vec<u32> {
    let mut bits = Vec::new();
    let p = v.position();
    let q = v.orientation();
    let (l, a) = (v.linear_velocity(), v.angular_velocity());
    for f in [p.x, p.y, p.z, q.x, q.y, q.z, q.w, l.x, l.y, l.z, a.x, a.y, a.z, v.rpm()] {
        bits.push(f.to_bits());
    }
    for i in 0..4 {
        let w = v.wheel(i);
        bits.extend([w.compression, w.angular_velocity, w.load, w.slip].map(f32::to_bits));
    }
    bits.push(v.gear() as u32);
    bits
}

#[test]
fn the_same_inputs_give_bit_identical_states() {
    let g = flat();
    let mut a = Vehicle::new(VehicleSpec::example());
    let mut b = Vehicle::new(VehicleSpec::example());
    a.place_moving(Vec3::new(0.0, REST_Y, 0.0), identity(), 15.0);
    b.place_moving(Vec3::new(0.0, REST_Y, 0.0), identity(), 15.0);
    let (mut ra, mut rb) = (Rng(7), Rng(7));
    for step in 0..3000 {
        a.step(FIXED_STEP, &ra.input(), &g);
        b.step(FIXED_STEP, &rb.input(), &g);
        assert_eq!(state_bits(&a), state_bits(&b), "diverged at step {step}");
    }
}

#[test]
fn a_cloned_vehicle_continues_identically() {
    let g = flat();
    let mut a = Vehicle::new(VehicleSpec::example());
    a.place_moving(Vec3::new(0.0, REST_Y, 0.0), identity(), 10.0);
    let mut rng = Rng(99);
    for _ in 0..500 {
        a.step(FIXED_STEP, &rng.input(), &g);
    }
    let mut b = a.clone();
    let mut rng2 = Rng(5);
    let mut rng3 = Rng(5);
    for _ in 0..500 {
        a.step(FIXED_STEP, &rng2.input(), &g);
        b.step(FIXED_STEP, &rng3.input(), &g);
    }
    assert_eq!(state_bits(&a), state_bits(&b));
}

fn assert_sane(v: &Vehicle) {
    assert!(v.position().is_finite(), "position {:?}", v.position());
    assert!(v.orientation().is_finite() && (v.orientation().length() - 1.0).abs() < 1e-3);
    assert!(v.linear_velocity().is_finite() && v.angular_velocity().is_finite());
    assert!(v.rpm().is_finite() && v.rpm() >= 0.0);
    for i in 0..4 {
        let w = v.wheel(i);
        assert!(w.position.is_finite() && w.angular_velocity.is_finite() && w.load.is_finite());
        assert!(w.lateral_force.is_finite() && w.longitudinal_force.is_finite() && w.slip.is_finite());
    }
}

#[test]
fn random_inputs_never_produce_nan() {
    let g = flat();
    for seed in 1..=6u64 {
        let mut v = Vehicle::new(VehicleSpec::example());
        v.place_moving(Vec3::new(0.0, REST_Y, 0.0), identity(), (seed as f32) * 8.0);
        let mut rng = Rng(seed * 7919);
        // Hold each random input for a while, like a driver would.
        let mut input = rng.input();
        for step in 0..20_000 {
            if step % 20 == 0 {
                input = rng.input();
            }
            v.step(FIXED_STEP, &input, &g);
            if step % 100 == 0 {
                assert_sane(&v);
            }
        }
        assert_sane(&v);
        assert!(v.position().length() < 100_000.0);
    }
}

#[test]
fn nan_and_extreme_inputs_are_survivable() {
    let g = flat();
    let mut v = parked();
    let nasty = [
        InputState { throttle: f32::NAN, brake: f32::INFINITY, steer: f32::NEG_INFINITY, ..Default::default() },
        InputState { throttle: 1e30, brake: -1e30, steer: 1e30, handbrake: 1e30, ..Default::default() },
    ];
    for input in nasty {
        for _ in 0..300 {
            v.step(FIXED_STEP, &input, &g);
        }
        assert_sane(&v);
    }
    // Zero, negative and NaN steps are ignored.
    let before = state_bits(&v);
    for dt in [0.0, -1.0, f32::NAN] {
        v.step(dt, &InputState::default(), &g);
    }
    assert_eq!(before, state_bits(&v));
}

#[test]
fn a_flipped_car_comes_to_rest_on_its_roof() {
    let g = flat();
    let mut v = Vehicle::new(VehicleSpec::example());
    v.place(Vec3::new(0.0, 1.5, 0.0), glam::Quat::from_rotation_z(std::f32::consts::PI));
    for _ in 0..(8.0 / FIXED_STEP) as usize {
        v.step(FIXED_STEP, &InputState::default(), &g);
        assert!(v.position().is_finite());
        assert!(v.position().y > 0.3, "sank into the road: y = {}", v.position().y);
    }
    assert!(up(&v).y < -0.9, "still upside down");
    assert!(v.speed() < 0.5);
    assert_eq!(v.wheels_on_ground(), 0);
}

/// Bumpy ground: a slow sine over a plane, with a changing surface.
struct Rolling;

impl Ground for Rolling {
    fn hit(&self, origin: Vec3, dir: Vec3, max_distance: f32) -> Option<GroundHit> {
        let h = |x: f32, z: f32| 0.15 * (z * 0.3).sin() + 0.1 * (x * 0.5).cos();
        // Step along the ray and refine, plenty for a smooth surface.
        let mut t = 0.0;
        let mut prev = origin.y - h(origin.x, origin.z);
        if prev < 0.0 {
            return None;
        }
        while t < max_distance {
            t += 0.01;
            let p = origin + dir * t;
            let gap = p.y - h(p.x, p.z);
            if gap <= 0.0 {
                let e = 0.01;
                let n = Vec3::new(h(p.x - e, p.z) - h(p.x + e, p.z), 2.0 * e, h(p.x, p.z - e) - h(p.x, p.z + e))
                    .normalize();
                let grip = if p.x > 0.0 { 1.0 } else { 0.7 };
                return Some(GroundHit {
                    distance: t,
                    normal: n,
                    surface: SurfaceGrip { lateral: grip, drive: grip, rolling: 1.0 },
                });
            }
            prev = gap;
        }
        let _ = prev;
        None
    }
}

#[test]
fn bumpy_ground_with_changing_grip_stays_sane() {
    let mut v = Vehicle::new(VehicleSpec::example());
    v.place_moving(Vec3::new(0.0, 1.0, 0.0), identity(), 20.0);
    let mut rng = Rng(31337);
    let mut input = rng.input();
    for step in 0..6000 {
        if step % 30 == 0 {
            input = rng.input();
            input.brake = 0.0;
        }
        v.step(FIXED_STEP, &input, &Rolling);
        if step % 50 == 0 {
            assert_sane(&v);
        }
    }
    assert_sane(&v);
}

#[test]
fn no_ground_means_free_flight_for_every_input() {
    let mut v = Vehicle::new(VehicleSpec::example());
    v.place(Vec3::new(0.0, 500.0, 0.0), identity());
    let mut rng = Rng(11);
    for _ in 0..600 {
        v.step(FIXED_STEP, &rng.input(), &NoGround);
        assert_sane(&v);
    }
    assert_eq!(v.wheels_on_ground(), 0);
}

fn variants() -> Vec<(&'static str, VehicleSpec)> {
    let base = VehicleSpec::example();
    let mut out = vec![("rear drive", base.clone())];
    let mut fwd = base.clone();
    fwd.transmission.torque_split = 1.0;
    fwd.transmission.differential = [0.4, 0.0, 0.0];
    out.push(("front drive", fwd));
    let mut awd = base.clone();
    awd.transmission.torque_split = 0.4;
    awd.transmission.differential = [0.3, 0.3, 0.5];
    out.push(("all wheel drive", awd));
    let mut turbo = base.clone();
    turbo.induction = crate::induction::InductionSpec {
        low_boost: 0.1,
        high_boost: 0.4,
        spool: 0.5,
        spool_time_up: 1.2,
        spool_time_down: 0.6,
        vacuum: -0.1,
        psi: 14.0,
    };
    turbo.transmission.torque_converter = 0.5;
    turbo.engine.speed_limiter = [120.0, 10.0];
    out.push(("turbo automatic with limiter", turbo));
    let mut four = base.clone();
    four.transmission.gear_ratio = vec![3.2, 0.0, 2.8, 1.6, 1.0, 0.7];
    four.transmission.gear_efficiency = vec![0.85; 6];
    four.engine.torque = vec![150.0];
    four.mass = 900.0;
    four.dimension = Vec3::new(0.8, 0.5, 1.9);
    out.push(("short gearbox, no torque curve", four));
    let mut heavy = base;
    heavy.mass = 12_000.0;
    heavy.chassis.spring_stiffness = [2500.0, 2500.0];
    heavy.chassis.shock_stiffness = [200.0, 200.0];
    heavy.chassis.shock_ext_stiffness = [260.0, 260.0];
    out.push(("heavy", heavy));
    out
}

#[test]
fn every_drivetrain_layout_stays_sane_under_random_input() {
    let g = flat();
    for (name, spec) in variants() {
        let mut v = Vehicle::new(spec);
        assert!(v.place_on_ground(&g, 0.0, 0.0, 5.0, 0.3), "{name}");
        let mut rng = Rng(4242);
        let mut input = rng.input();
        for step in 0..8000 {
            if step % 25 == 0 {
                input = rng.input();
            }
            v.step(FIXED_STEP, &input, &g);
            if step % 100 == 0 {
                assert_sane(&v);
            }
        }
        assert_sane(&v);
        println!("{name}: ok");
    }
}

#[test]
fn every_layout_drives_away_on_full_throttle() {
    let g = flat();
    for (name, spec) in variants() {
        if name == "short gearbox, no torque curve" || name == "heavy" {
            continue;
        }
        let mut v = Vehicle::new(spec);
        assert!(v.place_on_ground(&g, 0.0, 0.0, 5.0, 0.0), "{name}");
        run(&mut v, &InputState::default(), 2.0);
        for _ in 0..(8.0 / FIXED_STEP) as usize {
            v.step(FIXED_STEP, &InputState { throttle: 1.0, ..Default::default() }, &g);
        }
        assert!(v.forward_speed() > 10.0, "{name}: only {} m/s after 8 s", v.forward_speed());
        assert!(v.position().x.abs() < 1.0, "{name}: veered to x = {}", v.position().x);
    }
}
