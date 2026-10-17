//! Stock PC brake-flare history, sampled on the shared 60 Hz clock.
//! Behavior and timing choice: `docs/specs/nitrous-light-trails.md`.

use glam::Mat4;

const SLOTS: usize = 3;
const DIVISIONS: usize = 8;

#[cfg(test)]
mod installed_tests;

#[derive(Default)]
pub struct TailLightTrails {
    poses: Option<[Mat4; SLOTS]>,
    active: bool,
}

impl TailLightTrails {
    /// Keep history even while NOS is off, as the original car-flare dispatcher does.
    pub fn step(&mut self, pose: Mat4, nos_burning: bool, speed: f32) {
        if !pose.is_finite() || !speed.is_finite() {
            self.clear();
            return;
        }
        let poses = self.poses.get_or_insert([pose; SLOTS]);
        poses.rotate_left(1);
        poses[SLOTS - 1] = pose;
        self.active = nos_burning && speed.abs() > 0.2;
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Physics leads the displayed car by the interpolation phase. Rebase the whole
    /// smear to its displayed position so the newest edge cannot extend ahead of it.
    pub fn anchored(&self, displayed: Mat4) -> impl Iterator<Item = Mat4> + Clone {
        let newest = self.poses.map_or(displayed.w_axis, |p| p[SLOTS - 1].w_axis);
        let delta = displayed.w_axis - newest;
        self.samples().map(move |mut p| {
            p.w_axis += delta;
            p
        })
    }

    /// Three directed ring edges, including the original newest-to-oldest overlap.
    /// Only translation interpolates; every copy keeps its edge's start orientation.
    pub fn samples(&self) -> impl Iterator<Item = Mat4> + Clone {
        let poses = self.poses.filter(|_| self.active);
        poses.into_iter().flat_map(|poses| {
            [(0, 1), (2, 0), (1, 2)].into_iter().flat_map(move |(start, end)| {
                (1..=DIVISIONS).map(move |j| {
                    let mut frame = poses[start];
                    frame.w_axis = frame.w_axis.lerp(poses[end].w_axis, j as f32 / DIVISIONS as f32);
                    frame
                })
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Quat, Vec3};

    fn pose(x: f32, turn: f32) -> Mat4 {
        Mat4::from_rotation_translation(Quat::from_rotation_z(turn), Vec3::X * x)
    }

    #[test]
    fn samples_follow_the_three_pc_edges_with_start_orientation() {
        let mut trail = TailLightTrails::default();
        for i in 0..3 {
            trail.step(pose(i as f32, i as f32 * 0.1), true, 60.0);
        }
        let samples: Vec<_> = trail.samples().collect();
        assert_eq!(samples.len(), 24);
        for (index, expected_x, rotation) in
            [(0, 0.125, 0.0), (7, 1.0, 0.0), (8, 1.75, 0.2), (15, 0.0, 0.2), (16, 1.125, 0.1), (23, 2.0, 0.1)]
        {
            assert!((samples[index].w_axis.x - expected_x).abs() < 1e-5);
            assert!(samples[index].x_axis.abs_diff_eq(pose(0.0, rotation).x_axis, 1e-5));
        }
    }

    #[test]
    fn release_and_low_speed_remove_copies_but_keep_history_for_the_next_burn() {
        let mut trail = TailLightTrails::default();
        for i in 0..3 {
            trail.step(pose(i as f32, 0.0), false, 60.0);
        }
        assert_eq!(trail.samples().count(), 0);
        trail.step(pose(3.0, 0.0), true, 60.0);
        assert_eq!(trail.samples().count(), 24);
        assert_eq!(trail.samples().next().unwrap().w_axis.x, 1.125);
        for (burning, speed) in [(false, 60.0), (true, 0.2), (true, f32::NAN)] {
            trail.step(pose(4.0, 0.0), burning, speed);
            assert_eq!(trail.samples().count(), 0);
        }
    }

    #[test]
    fn reset_seeds_current_pose_without_a_bridge_to_the_previous_location() {
        let mut trail = TailLightTrails::default();
        trail.step(pose(1000.0, 0.0), true, 60.0);
        trail.clear();
        assert_eq!(trail.samples().count(), 0);
        trail.step(pose(5.0, 0.0), true, 60.0);
        assert!(trail.samples().all(|p| p.w_axis.x == 5.0));
    }

    #[test]
    fn displayed_phase_keeps_the_newest_trail_endpoint_on_the_car() {
        let mut trail = TailLightTrails::default();
        let step_distance = (366.0 / 3.6) / 60.0;
        for i in 0..3 {
            trail.step(pose(i as f32 * step_distance, 0.0), true, 366.0 / 3.6);
        }
        for alpha in [0.0, 0.5, 1.0] {
            let displayed_x = (1.0 + alpha) * step_distance;
            let samples: Vec<_> = trail.anchored(pose(displayed_x, 0.0)).collect();
            assert!((samples.last().unwrap().w_axis.x - displayed_x).abs() < 1e-5);
            assert!(samples.iter().all(|p| p.w_axis.x <= displayed_x + 1e-5));
            assert!((samples[15].w_axis.x - (displayed_x - 2.0 * step_distance)).abs() < 1e-5);
        }
    }

    #[test]
    fn equal_simulated_motion_has_equal_trails_at_different_display_rates() {
        use crate::scenes::world::drive::FixedClock;
        let mut expected = None;
        for fps in [30, 60, 120, 144] {
            let (mut clock, mut trail, mut tick) = (FixedClock::default(), TailLightTrails::default(), 0);
            for _ in 0..fps {
                for _ in 0..clock.advance(1.0 / fps as f32) {
                    tick += 1;
                    trail.step(pose(tick as f32 * 101.666_664 / 60.0, 0.0), true, 101.666_664);
                }
            }
            let positions: Vec<_> = trail.samples().map(|p| p.w_axis).collect();
            if let Some(expected) = &expected {
                assert_eq!(&positions, expected);
            }
            expected = Some(positions);
        }
    }
}
