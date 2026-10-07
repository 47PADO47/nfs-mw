//! Level-of-detail choice from projected size. Implemented from
//! `docs/specs/scenery-lod.md`; the constants live in the game's layout.

use crate::layout::LodRules;

/// What the LOD choice needs to know about the most detailed model (slot 0).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LodModel {
    pub num_polys: u16,
    pub density: f32,
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
        let mut size = view.projected_size(position, info_radius + self.radius_pad);
        if size < self.min_size {
            return None;
        }
        if flags & self.boost_flag != 0 {
            size += self.boost;
        }
        if size <= self.draw_threshold {
            return None;
        }
        let score = match detailed {
            Some(m) if m.num_polys > self.poly_threshold => size as f32 / m.density.max(self.density_floor),
            _ => self.density_threshold,
        };
        let slot = if score < self.density_threshold { self.coarse_slot } else { self.detailed_slot };
        slots[slot].then_some(slot)
    }
}

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
