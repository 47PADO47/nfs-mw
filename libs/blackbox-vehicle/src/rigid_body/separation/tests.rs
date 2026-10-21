use glam::{Quat, Vec3};

use super::*;
use crate::rigid_body::{BodyHit, ObbContact, RigidBodySpec, resolve_bodies};

fn body(mass: f32, position: Vec3, velocity: Vec3) -> RigidBody {
    let mut body = RigidBody::new(mass, Vec3::ONE, Vec3::ONE, RigidBodySpec::default());
    body.place(position, Quat::IDENTITY);
    body.linear_velocity = velocity;
    body
}

fn contact(first: bool) -> BodyContact {
    BodyContact::from_geometry(
        ObbContact { point: Vec3::new(0.0, 0.3, 0.9), normal: Vec3::NEG_Z, overlap: 0.4, point_on_first: first },
        [0.5, 0.3],
    )
}

#[test]
fn corrected_surface_point_and_temporary_centres_follow_their_own_bodies() {
    for first in [false, true] {
        let mut a = body(1000.0, Vec3::ZERO, Vec3::Z * 20.0);
        let mut b = body(3000.0, Vec3::Z * 1.6, Vec3::ZERO);
        let (mut ma, mut mb) = (a.contact_moment(&BodyHit::default()), b.contact_moment(&BodyHit::default()));
        ma.centre.y = 0.175;
        let initial = ma.centre;
        let mut c = contact(first);
        let point = c.point;
        assert!(correct(&mut a, &mut b, &mut ma, &mut mb, &mut c, Separation::Mass));
        assert!((a.position.z + 0.3).abs() < 0.000001);
        assert!((b.position.z - 1.7).abs() < 0.000001);
        assert!((ma.centre - initial - a.position).length() < 0.000001);
        assert!((mb.centre - b.position).length() < 0.000001);
        let delta = match first {
            true => -0.3,
            false => 0.1,
        };
        assert!((c.point - point - Vec3::Z * delta).length() < 0.000001);
    }
}

#[test]
fn prop_separation_uses_normal_speed_even_when_the_mover_is_heavier() {
    for speeds in [(20.0, 0.0), (0.0, -20.0), (20.0, -60.0), (-20.0, 60.0)] {
        let mut a = body(10000.0, Vec3::ZERO, Vec3::Z * speeds.0);
        let mut b = body(10.0, Vec3::Z * 1.6, Vec3::Z * speeds.1);
        let (mut ma, mut mb) = (a.contact_moment(&BodyHit::default()), b.contact_moment(&BodyHit::default()));
        let mut c = contact(false);
        assert!(correct(&mut a, &mut b, &mut ma, &mut mb, &mut c, Separation::NormalSpeed));
        let share = match speeds {
            (20.0, 0.0) => 1.0,
            (0.0, -20.0) => 0.0,
            (20.0, -60.0) => 0.25,
            _ => 0.75,
        };
        assert!((a.position.z + share * 0.4).abs() < 0.000001, "{speeds:?}");
        assert!((b.position.z - 1.6 - (1.0 - share) * 0.4).abs() < 0.000001);
    }
}

#[test]
fn two_immobile_moments_release_only_positive_attachments() {
    for threshold in [-1.0, 0.0, 1.0] {
        let mut a = body(1000.0, Vec3::ZERO, Vec3::Z * 20.0);
        let mut b = body(1000.0, Vec3::Z * 1.6, Vec3::ZERO);
        let hit_a = BodyHit { immobile: true, breaking_impulse: threshold, ..Default::default() };
        let hit_b = BodyHit { immobile: true, ..Default::default() };
        let (mut ma, mut mb) = (a.contact_moment(&hit_a), b.contact_moment(&hit_b));
        let mut c = contact(true);
        assert_eq!(correct(&mut a, &mut b, &mut ma, &mut mb, &mut c, Separation::Mass), threshold > 0.0);
        assert!(mb.immobile);
        assert_eq!(ma.immobile, threshold <= 0.0);
    }
}

#[test]
fn collision_notifications_use_effective_mass_without_retuning_stored_mass() {
    let mut a = body(2000.0, Vec3::ZERO, Vec3::Z * 20.0);
    let mut b = body(1000.0, Vec3::Z * 1.6, Vec3::ZERO);
    let ha = BodyHit { mass_scale: 0.5, ..Default::default() };
    let hb = BodyHit { immobile: true, ..Default::default() };
    let r = resolve_bodies(&mut a, &mut b, &contact(true), &ha, &hb, Separation::Mass).unwrap();
    assert_eq!(r.impulse_delta_v[0], r.reaction.impulse / 1000.0);
    assert_eq!(r.impulse_delta_v[1], 0.0);
    assert_eq!(r.notification_delta_v, [r.reaction.impulse / 1000.0; 2]);
    assert_eq!(a.mass(), 2000.0);
    assert_eq!(b.mass(), 1000.0);
}

#[test]
fn immobile_notification_uses_partner_mass_in_both_body_orders() {
    for immobile_first in [false, true] {
        let mut a = body(2000.0, Vec3::ZERO, Vec3::Z * 20.0);
        let mut b = body(100.0, Vec3::Z * 1.6, Vec3::ZERO);
        let ha = BodyHit { immobile: immobile_first, mass_scale: 0.5, ..Default::default() };
        let hb = BodyHit { immobile: !immobile_first, mass_scale: 2.0, ..Default::default() };
        let before = [a.linear_velocity, b.linear_velocity];
        let r = resolve_bodies(&mut a, &mut b, &contact(true), &ha, &hb, Separation::Mass).unwrap();
        let fixed = usize::from(!immobile_first);
        let moving = 1 - fixed;
        let mass = [1000.0, 200.0][moving];
        assert!(r.reaction.impulse > 0.0);
        assert_eq!(r.impulse_delta_v[fixed], 0.0);
        assert_eq!(r.notification_delta_v, [r.reaction.impulse / mass; 2]);
        assert_eq!([a.linear_velocity, b.linear_velocity][fixed], before[fixed]);
        assert_eq!([a.mass(), b.mass()], [2000.0, 100.0]);
    }
}
