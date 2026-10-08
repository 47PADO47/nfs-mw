use super::*;
use crate::ground::SurfaceGrip;

const DT: f32 = crate::FIXED_STEP;

fn params(front: bool) -> TireParams {
    TireParams {
        radius: 0.33,
        grip_scale: 1.0,
        static_grip: 1.1,
        dynamic_grip: 1.0,
        brake_spec: 1500.0,
        brake_lock_spec: 1200.0,
        ebrake_spec: 0.0,
        front,
    }
}

/// A quarter car: one tire under a 400 kg point mass sliding on the ground plane.
struct Corner {
    tire: Tire,
    p: TireParams,
    fwd: f32,
    lat: f32,
    mass: f32,
}

impl Corner {
    fn new(fwd: f32, lat: f32) -> Self {
        let tire = Tire { av: fwd / 0.33, ..Tire::default() };
        Self { tire, p: params(false), fwd, lat, mass: 400.0 }
    }

    fn step(&mut self, drive: f32, brake: f32) {
        let speed = self.fwd.hypot(self.lat);
        let sc = StepScales::new(speed, false);
        self.tire.begin_frame(sc.max_slip, sc.grip_scale, sc.traction_scale);
        self.tire.drive_torque = drive;
        self.tire.brake = brake;
        let fy = self.tire.update_loaded(
            &self.p,
            &LoadedInput {
                lat_vel: self.lat,
                fwd_vel: self.fwd,
                body_speed: speed,
                load: self.mass * 9.81,
                dt: DT,
                quarter_mass: self.mass,
                surface: SurfaceGrip::DEFAULT,
            },
        );
        let fx = self.tire.longitudinal_force;
        self.fwd += fx / self.mass * DT;
        self.lat += fy / self.mass * DT;
    }
}

#[test]
fn rolling_freely_makes_no_force() {
    let mut c = Corner::new(20.0, 0.0);
    for _ in 0..120 {
        c.step(0.0, 0.0);
    }
    assert!((c.fwd - 20.0).abs() < 0.05, "fwd {}", c.fwd);
    assert!(c.tire.slip.abs() < 0.1);
    assert!(c.lat == 0.0);
}

#[test]
fn sideways_motion_is_resisted_within_the_friction_limit() {
    let mut c = Corner::new(20.0, 3.0);
    let mut max_force = 0.0_f32;
    for _ in 0..180 {
        c.step(0.0, 0.0);
        max_force = max_force.max(c.tire.lateral_force.abs());
        assert!(c.lat.is_finite());
    }
    assert!(c.lat.abs() < 0.3, "lateral speed left: {}", c.lat);
    assert!(max_force <= 400.0 * 9.81 * 1.1 * 1.2 + 1.0, "force {max_force}");
    assert!(c.fwd > 18.0, "cornering drag took too much: {}", c.fwd);
}

#[test]
fn lateral_force_opposes_the_slide_direction() {
    let mut right = Corner::new(15.0, 2.0);
    let mut left = Corner::new(15.0, -2.0);
    right.step(0.0, 0.0);
    left.step(0.0, 0.0);
    assert!(right.tire.lateral_force < 0.0 && left.tire.lateral_force > 0.0);
    assert!((right.tire.lateral_force + left.tire.lateral_force).abs() < 1e-3);
}

#[test]
fn drive_torque_accelerates_and_is_bounded_by_friction() {
    let mut c = Corner::new(0.0, 0.0);
    let mut max_fx = 0.0_f32;
    for _ in 0..180 {
        c.step(900.0, 0.0);
        max_fx = max_fx.max(c.tire.longitudinal_force);
    }
    assert!(c.fwd > 3.0, "speed {}", c.fwd);
    assert!(max_fx <= 400.0 * 9.81 * 1.1 * 1.5 * 1.2 + 1.0, "fx {max_fx}");
    assert!(c.fwd.is_finite() && c.tire.av.is_finite());
}

#[test]
fn too_much_torque_spins_the_wheel() {
    let mut c = Corner::new(5.0, 0.0);
    for _ in 0..60 {
        c.step(6000.0, 0.0);
    }
    assert!(c.tire.slip > 1.0, "slip {}", c.tire.slip);
    assert!(c.tire.traction < 1.0);
    let limit = 400.0 * 9.81 * 1.1 * 1.2 * 1.6;
    assert!(c.tire.longitudinal_force < limit);
}

#[test]
fn braking_stops_without_reversing() {
    let mut c = Corner::new(25.0, 0.0);
    let mut distance = 0.0;
    for _ in 0..(60 * 10) {
        c.step(0.0, 1.0);
        distance += c.fwd * DT;
        assert!(c.fwd >= -0.05, "reversed: {}", c.fwd);
    }
    assert!(c.fwd.abs() < 0.2, "still rolling at {}", c.fwd);
    // Stopping from 25 m/s at about 1 g is about 32 m; allow a wide band for lock-up behaviour.
    assert!(distance > 20.0 && distance < 80.0, "distance {distance}");
}

#[test]
fn stopped_tire_is_quiet() {
    let mut c = Corner::new(0.0, 0.0);
    for _ in 0..60 {
        c.step(0.0, 0.0);
    }
    assert_eq!((c.fwd, c.lat), (0.0, 0.0));
    assert_eq!(c.tire.lateral_force, 0.0);
}

#[test]
fn slow_lateral_creep_settles_without_chatter() {
    let mut c = Corner::new(0.0, 0.8);
    let mut sign_changes = 0;
    let mut last = c.lat;
    for _ in 0..120 {
        c.step(0.0, 0.0);
        if c.lat * last < 0.0 {
            sign_changes += 1;
        }
        if c.lat != 0.0 {
            last = c.lat;
        }
    }
    assert!(sign_changes <= 1, "{sign_changes} sign changes");
    assert!(c.lat.abs() < 0.2, "{}", c.lat);
}

#[test]
fn free_tire_coasts_and_brakes() {
    let p = params(false);
    let mut t = Tire { av: 30.0, ..Tire::default() };
    t.update_free(&p, DT);
    assert!(t.av == 30.0, "coasts when nothing acts");
    t.brake = 1.0;
    for _ in 0..600 {
        t.update_free(&p, DT);
        assert!(t.av >= 0.0);
    }
    assert_eq!(t.av, 0.0);
}

#[test]
fn check_sign_stops_a_sign_flip() {
    let mut t = Tire { av: 1.0, ..Tire::default() };
    t.check_sign();
    t.av = -0.5;
    t.check_sign();
    assert_eq!(t.av, 0.0);
    t.av = -0.5;
    t.check_sign();
    assert_eq!(t.av, -0.5);
}
