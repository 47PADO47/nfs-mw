//! The body's centre swept over the step: the outline probes are cast from the centre, so a car fast enough to carry
//! its centre past a thin barrier in one step would be pushed out on the far side. The segment from where the centre
//! was to where it is finds that barrier, and the contact puts the centre back on the side it came from. A placement
//! resets where the centre was (`CarSim::previous_contact_pose`), so a teleport is never swept.

use glam::Vec3;

use super::{Cast, FLOOR_NORMAL, WallContact};

/// Moves shorter than this are not swept: the probes see everything a car can reach in them.
const MIN_SWEEP: f32 = 0.05;
/// How far in front of the wall the centre is put back.
const SKIN: f32 = 0.05;

/// The wall the centre crossed moving from `previous` to `position`, as a contact whose depth puts the centre back
/// [`SKIN`] in front of it. Floors and props are not swept.
pub fn sweep_contact(cast: Cast<'_>, previous: Vec3, position: Vec3) -> Option<WallContact> {
    if previous.distance_squared(position) < MIN_SWEEP * MIN_SWEEP {
        return None;
    }
    let hit = cast(previous, position)?;
    if hit.prop.is_some() || hit.normal.y.abs() > FLOOR_NORMAL {
        return None;
    }
    // The normal faces where the cast started: the side the centre came from.
    let depth = (hit.point - position).dot(hit.normal) + SKIN;
    (depth > 0.0).then_some(WallContact { point: hit.point, normal: hit.normal, depth, prop: None, info: hit.info })
}

#[cfg(test)]
mod tests {
    use blackbox_collision::{CollisionWorld, HitKind};
    use blackbox_vehicle::{FlatGround, Vehicle, VehicleSpec};
    use glam::Mat3;

    use super::super::{Hit, eligible, resolve_into};
    use super::*;

    /// A thin wall across the road at z = 2 (physics space) that blocks from both sides.
    fn thin_wall(from: Vec3, to: Vec3) -> Option<Hit> {
        let crosses = (from.z - 2.0) * (to.z - 2.0) <= 0.0 && from.z != to.z;
        let t = (2.0 - from.z) / (to.z - from.z);
        let normal = Vec3::Z * (from.z - 2.0).signum();
        crosses.then(|| Hit { point: from + (to - from) * t, normal, prop: None, info: None })
    }

    #[test]
    fn a_centre_carried_past_a_thin_barrier_is_put_back() {
        let contact = sweep_contact(&thin_wall, Vec3::new(0.0, 0.5, 1.5), Vec3::new(0.0, 0.5, 2.6)).expect("crossed");
        assert_eq!(contact.normal, Vec3::NEG_Z, "pushed back the way it came");
        assert!((contact.depth - 0.65).abs() < 1e-4, "{}", contact.depth);
        assert!(sweep_contact(&thin_wall, Vec3::new(0.0, 0.5, 0.5), Vec3::new(0.0, 0.5, 1.5)).is_none());
    }

    #[test]
    fn a_teleport_or_a_standing_car_is_not_swept() {
        let at = Vec3::new(0.0, 0.5, 2.0);
        assert!(sweep_contact(&thin_wall, at, at).is_none());
    }

    #[test]
    fn a_thin_barrier_crossed_in_one_step_pushes_the_car_back() {
        let mut car = Vehicle::new(VehicleSpec::example());
        assert!(car.place_on_ground(&FlatGround::new(0.0), 0.0, 0.0, 2.0, 0.0));
        let previous = car.body().position;
        // One step carried the centre 1 m past the wall at z = 2; its outline probes start beyond it.
        car.body_mut().position.z = 3.0;
        car.body_mut().linear_velocity = Vec3::new(0.0, 0.0, 60.0);
        let walls = nfsmw_data::car::physics::WallSpec::default();
        let no_props = |_: Vec3, _: Mat3, _: Vec3| Vec::new();
        let mut scratch = Vec::new();
        resolve_into(&mut car, previous, &thin_wall, &no_props, &walls, &|_| 1.0, true, &mut scratch);
        assert!(car.body().position.z < 2.0, "back on the near side: {}", car.body().position.z);
        assert!(car.body().linear_velocity.z < 60.0, "and the wall took speed");
    }

    fn barrier(front_facing: bool, group: u16) -> blackbox_collision::Hit {
        blackbox_collision::Hit {
            t: 0.5,
            point: [0.0; 3],
            normal: [0.0, 0.0, -1.0],
            kind: HitKind::Barrier,
            front_facing,
            surface_hash: 0,
            surface_index: 0,
            surface_flags: 0,
            section: 1,
            instance: 0,
            group,
        }
    }

    #[test]
    fn a_back_facing_barrier_of_a_two_sided_group_blocks() {
        let mut collision = CollisionWorld::new(None);
        collision.set_group_two_sided(9, true);
        assert!(eligible(&barrier(false, 9), &collision));
        assert!(eligible(&barrier(true, 9), &collision));
    }

    #[test]
    fn a_back_facing_one_sided_barrier_lets_the_car_through() {
        let mut collision = CollisionWorld::new(None);
        collision.set_group_two_sided(9, true);
        assert!(!eligible(&barrier(false, 0), &collision), "an ungrouped one-sided barrier");
        assert!(!eligible(&barrier(false, 4), &collision), "a group that is not two-sided");
        assert!(eligible(&barrier(true, 0), &collision), "its front still blocks");
    }
}
