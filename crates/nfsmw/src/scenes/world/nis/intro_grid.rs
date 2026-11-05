//! Visible staging for race entrants whose intro has no authored car track.
use glam::Vec3;

use super::car_world::actor_grid_slot;
use super::cars::{CarSpot, Target};
use crate::scenes::world::WorldScene;

impl WorldScene {
    pub(super) fn stage_extra_intro_cars(&self, spots: &mut Vec<(Target, CarSpot)>) {
        if !self.nis.race_start_scene() {
            return;
        }
        let Some(primary) = spots.iter().find_map(|(target, spot)| (*target == Target::Player).then_some(*spot)) else {
            return;
        };
        let (Some(race), Some(traffic)) = (&self.race, &self.traffic) else { return };
        for (id, plan) in race.plan.iter().enumerate().skip(1) {
            let Some(index) = traffic.racer_car_index(id) else { continue };
            let target = Target::Traffic(index);
            if spots.iter().any(|(linked, _)| *linked == target) {
                continue;
            }
            let slot = actor_grid_slot(race.plan[0].grid_slot, plan.grid_slot);
            let occupied: Vec<_> = spots.iter().map(|(_, spot)| *spot).collect();
            spots.push((target, fallback(primary, slot, &occupied)));
        }
    }
}

fn fallback(primary: CarSpot, slot: usize, occupied: &[CarSpot]) -> CarSpot {
    let mut local = Vec3::new(-7.5 * (slot / 2) as f32, -3.5 * (slot % 2) as f32, 0.0);
    loop {
        let position = primary.position + primary.rotation * local;
        let overlaps = occupied.iter().any(|spot| {
            let delta = primary.rotation.inverse() * (spot.position - position);
            delta.x.abs() < 6.0 && delta.y.abs() < 2.6
        });
        if !overlaps {
            return CarSpot { position, rotation: primary.rotation };
        }
        local.x -= 7.5;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Quat;

    #[test]
    fn missing_tracks_keep_the_full_field_visible_and_apart() {
        let primary = CarSpot { position: Vec3::new(500.0, -100.0, 20.0), rotation: Quat::from_rotation_z(1.2) };
        let mut occupied = vec![primary];
        for slot in 1..16 {
            let spot = fallback(primary, slot, &occupied);
            assert!(occupied.iter().all(|other| other.position.distance(spot.position) > 3.0));
            occupied.push(spot);
        }
        assert_eq!(occupied.len(), 16);
    }
}
