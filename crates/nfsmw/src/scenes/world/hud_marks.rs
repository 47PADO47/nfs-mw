//! The places the Custom HUD's world markers stand on (`hud/state/marks.rs`): the ground under the map places near the
//! car, the pursuit breakers in sight during a chase, and whether the static world hides each of them, and the next
//! gate, from the camera (`docs/specs/custom-hud-markers-impl.md`).
//!
//! The hiding test is the cops' own line-of-sight ray (`drive/sight.rs`): from the camera's eye to a point a little above
//! the place, against the resident collision. A road deck, a building or a hill in between hides the marker. The cost
//! is bounded: per HUD frame at most [`MAX_ENTRANCES`] ground probes and rays for the places, [`MAX_BREAKERS`] rays for
//! the breakers and one for the next gate.

use glam::{Mat4, Vec3, Vec4};

use super::drive::sight::sight_cast;
use super::{WorldScene, space};
use crate::hud::{EntranceMark, HudCamera, HudState, MarkPoint, WorldMarks};

/// Map places farther than this from the car get no marker, metres (`docs/specs/custom-hud/world-markers.md`).
pub(super) const ENTRANCE_RANGE: f32 = 300.0;
/// Pursuit breakers farther than this from the car are out of sight, metres. **[ours]**
pub(super) const BREAKER_SIGHT: f32 = 150.0;
/// The most map places and breakers looked at per frame: the HUD draws six markers at most.
const MAX_ENTRANCES: usize = 6;
const MAX_BREAKERS: usize = 4;
/// The hiding ray ends this far above the place, metres: the marker floats over it, and a ray to the ground itself would
/// always meet the ground.
const LIFT: f32 = 2.0;
/// A hit this close to the ray's end does not hide the marker, metres: it is the surface the marker stands on.
const SLACK: f32 = 1.0;
/// The ground under a place is looked for this far above and below the car, metres.
const GROUND_SPAN: f32 = 80.0;

impl WorldScene {
    /// The places the world markers stand on this frame, for `state` with its camera, race and map places already set.
    pub(super) fn world_marks(&self, state: &HudState) -> WorldMarks {
        let Some(eye) = state.camera.as_ref().and_then(eye) else { return WorldMarks::default() };
        let Some(car) = self.players.lead().map(|drive| drive.pose().position) else { return WorldMarks::default() };
        let collision = self.residency.collision();
        let cast = sight_cast(collision);
        let from = Vec3::from(space::to_physics(eye));
        let hidden = |place: Vec3| {
            let to = Vec3::from(space::to_physics(place + Vec3::Z * LIFT));
            cast(from, to).is_some_and(|hit| hit.point.distance(to) > SLACK)
        };
        let gate = state.race_info.as_ref().and_then(|race| race.next_gate);
        let gate_occluded = gate.is_some_and(|gate| hidden(Vec3::from(gate)));
        let places = state.markers.iter().map(|marker| marker.position).enumerate();
        let entrances = nearest(places, car, ENTRANCE_RANGE, MAX_ENTRANCES)
            .into_iter()
            .filter_map(|(marker, [x, y])| {
                let ground = space::ground_below(collision, x, y, car.z + GROUND_SPAN, car.z - GROUND_SPAN)?;
                let place = Vec3::new(x, y, ground);
                Some(EntranceMark { marker, point: MarkPoint { position: place.to_array(), occluded: hidden(place) } })
            })
            .collect();
        let pursuing = self.traffic.as_ref().is_some_and(|traffic| traffic.is_pursuing());
        let breakers = self
            .breaker_icons
            .near(pursuing, car, BREAKER_SIGHT, MAX_BREAKERS)
            .into_iter()
            .map(|place| MarkPoint { position: place.to_array(), occluded: hidden(place) })
            .collect();
        WorldMarks { gate_occluded, entrances, breakers }
    }
}

/// The camera's eye in render space, from its matrix: the point the projection sends to clip `w` 0 at the middle of the
/// view. `None` for a matrix without one.
fn eye(camera: &HudCamera) -> Option<Vec3> {
    let inverse = Mat4::from_cols_array_2d(&camera.view_proj).inverse();
    let eye = inverse * Vec4::Z;
    if eye.w.abs() <= f32::EPSILON {
        return None;
    }
    let eye = eye.truncate() / eye.w;
    eye.is_finite().then_some(eye)
}

/// The `count` items whose (x, y) is nearest `car` across the ground, within `range` metres, nearest first.
fn nearest<T>(items: impl Iterator<Item = (T, [f32; 2])>, car: Vec3, range: f32, count: usize) -> Vec<(T, [f32; 2])> {
    let mut near: Vec<(f32, T, [f32; 2])> = items
        .filter_map(|(item, at)| {
            let distance = (at[0] - car.x).hypot(at[1] - car.y);
            (distance <= range).then_some((distance, item, at))
        })
        .collect();
    near.sort_by(|a, b| a.0.total_cmp(&b.0));
    near.into_iter().take(count).map(|(_, item, at)| (item, at)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::viewer::camera::Lens;

    #[test]
    fn the_eye_comes_back_out_of_the_cameras_matrix() {
        let at = Vec3::new(120.0, -40.0, 12.0);
        let lens = Lens::look_at(at, at + Vec3::new(10.0, 3.0, -2.0), 60.0, 0.3, 16.0 / 9.0);
        let camera = HudCamera { view_proj: lens.view_proj().to_cols_array_2d(), viewport: [1600.0, 900.0] };
        let eye = eye(&camera).expect("a perspective camera has an eye");
        assert!(eye.distance(at) < 0.05, "{eye:?}");
    }

    #[test]
    fn the_nearest_places_within_range_come_first_and_the_rest_are_cut() {
        let places = [[0.0, 290.0], [10.0, 0.0], [400.0, 0.0], [0.0, -50.0]];
        let near = nearest(places.into_iter().enumerate(), Vec3::new(0.0, 0.0, 30.0), 300.0, 2);
        assert_eq!(near, vec![(1, [10.0, 0.0]), (3, [0.0, -50.0])]);
        let all = nearest(places.into_iter().enumerate(), Vec3::ZERO, 300.0, 6);
        assert_eq!(all.iter().map(|(i, _)| *i).collect::<Vec<_>>(), vec![1, 3, 0], "the one past 300 m is out");
    }
}
