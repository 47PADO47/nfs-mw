//! Level-of-detail choice from projected size. Implemented from
//! `docs/specs/scenery-lod.md`; the constants live in the game's layout.

use crate::layout::LodRules;

/// What the LOD choice needs to know about the most detailed model (slot 0).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LodModel {
    pub num_polys: u16,
    pub density: f32,
}

/// One instance's authored geometry inputs, independent of the view's policy.
#[derive(Clone, Copy)]
pub struct LodObject {
    pub position: [f32; 3],
    pub radius: f32,
    pub flags: u32,
    pub slots: [bool; 4],
    pub detailed: Option<LodModel>,
}

/// Optional view-local quality policy. The default retains the layout's rule.
#[derive(Clone, Copy)]
pub struct LodPolicy {
    pub coarse_only: bool,
    pub highest: bool,
    pub geometry_scale: f32,
    pub visibility_scale: f32,
    /// Relative density threshold band; zero disables hysteresis.
    pub hysteresis: f32,
    pub previous: Option<usize>,
}
impl Default for LodPolicy {
    fn default() -> Self {
        Self {
            coarse_only: false,
            highest: false,
            geometry_scale: 1.0,
            visibility_scale: 1.0,
            hysteresis: 0.0,
            previous: None,
        }
    }
}

/// The camera, for projected sizes.
#[derive(Debug, Clone, Copy)]
pub struct LodView {
    pub position: [f32; 3],
    /// Unit view direction.
    pub forward: [f32; 3],
    /// Projection scale in pixels (focal length for the vertical field of view).
    pub pixel_scale: f32,
}

impl LodView {
    /// Projection scale for a vertical field of view at a reference screen height.
    pub fn pixel_scale_for(fov_y_radians: f32, reference_height: f32) -> f32 {
        reference_height * 0.5 / (fov_y_radians * 0.5).tan()
    }

    /// Projected size in whole pixels of a sphere around `position`.
    pub fn projected_size(&self, position: [f32; 3], radius: f32) -> i32 {
        let to = [position[0] - self.position[0], position[1] - self.position[1], position[2] - self.position[2]];
        let ahead = to[0] * self.forward[0] + to[1] * self.forward[1] + to[2] * self.forward[2];
        if ahead < -radius {
            return 0;
        }
        let away = (to[0] * to[0] + to[1] * to[1] + to[2] * to[2]).sqrt() - radius;
        let size = if away > radius { radius * self.pixel_scale / away } else { self.pixel_scale };
        size as i32
    }
}

impl LodRules {
    /// The model slot to draw in the player view, or `None` to draw nothing.
    ///
    /// `info_radius` is the scenery info's radius, `flags` the instance's exclude
    /// flags, `slots` which of the four slots have a model, and `detailed` the
    /// slot-0 solid if there is one.
    pub fn choose(
        &self,
        view: &LodView,
        position: [f32; 3],
        info_radius: f32,
        flags: u32,
        slots: [bool; 4],
        detailed: Option<LodModel>,
    ) -> Option<usize> {
        self.choose_with(
            view,
            LodObject { position, radius: info_radius, flags, slots, detailed },
            LodPolicy::default(),
        )
    }

    pub fn choose_with(&self, view: &LodView, object: LodObject, policy: LodPolicy) -> Option<usize> {
        // Full detail relies on the caller's frustum/section culling, not projected-size rejection.
        if policy.highest && !policy.coarse_only {
            return [self.detailed_slot, self.coarse_slot].into_iter().find(|&slot| object.slots[slot]);
        }
        let size = view.projected_size(object.position, object.radius + self.radius_pad);
        let visible = (size as f32 * policy.visibility_scale) as i32;
        if visible < self.min_size {
            return None;
        }
        let boost = match object.flags & self.boost_flag != 0 {
            true => self.boost,
            false => 0,
        };
        if visible + boost <= self.draw_threshold {
            return None;
        }
        if policy.coarse_only {
            return object.slots[self.coarse_slot].then_some(self.coarse_slot);
        }
        let score = match object.detailed {
            Some(m) if m.num_polys > self.poly_threshold => {
                (size as f32 * policy.geometry_scale + boost as f32) / m.density.max(self.density_floor)
            }
            _ => f32::INFINITY,
        };
        let band = policy.hysteresis.clamp(0.0, 0.5);
        let threshold = match policy.previous {
            Some(slot) if slot == self.detailed_slot => self.density_threshold * (1.0 - band),
            Some(slot) if slot == self.coarse_slot => self.density_threshold * (1.0 + band),
            _ => self.density_threshold,
        };
        let distance_sq: f32 = object.position.iter().zip(view.position).map(|(a, b)| (a - b).powi(2)).sum();
        if score < threshold && distance_sq >= self.coarse_distance.powi(2) {
            return object.slots[self.coarse_slot].then_some(self.coarse_slot);
        }
        object.slots[self.detailed_slot].then_some(self.detailed_slot)
    }
}

#[cfg(test)]
#[path = "lod/policy_tests.rs"]
mod policy_tests;
#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::MOST_WANTED;

    const RULES: &LodRules = &MOST_WANTED.lod;

    fn view() -> LodView {
        LodView { position: [0.0; 3], forward: [1.0, 0.0, 0.0], pixel_scale: 343.0 }
    }

    #[test]
    fn projected_size() {
        let v = view();
        assert_eq!(v.projected_size([-100.0, 0.0, 0.0], 5.0), 0); // behind
        assert_eq!(v.projected_size([3.0, 0.0, 0.0], 5.0), 343); // camera inside
        assert_eq!(v.projected_size([105.0, 0.0, 0.0], 5.0), 17); // 5 * 343 / 100
    }

    #[test]
    fn choice() {
        let v = view();
        let all = [true; 4];
        // Close, simple model: detailed slot.
        assert_eq!(RULES.choose(&v, [20.0, 0.0, 0.0], 4.0, 0, all, None), Some(0));
        // Far beyond the 17 px threshold: nothing.
        assert_eq!(RULES.choose(&v, [500.0, 0.0, 0.0], 4.0, 0, all, None), None);
        // Dense detailed model at mid range: coarse slot.
        let dense = Some(LodModel { num_polys: 400, density: 30.0 });
        assert_eq!(RULES.choose(&v, [80.0, 0.0, 0.0], 4.0, 0, all, dense), Some(2));
        assert_eq!(RULES.choose(&v, [80.0, 0.0, 0.0], 4.0, 0, [true, false, false, false], dense), None);
        // The boost flag keeps a small object visible a little longer.
        assert_eq!(RULES.choose(&v, [295.0, 0.0, 0.0], 4.0, 0, all, None), None);
        assert_eq!(RULES.choose(&v, [295.0, 0.0, 0.0], 4.0, 0x0200_0000, all, None), Some(0));
    }
}
