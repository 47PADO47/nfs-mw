//! Back-face culling: which faces of a draw are kept, from the draw's sidedness, its instance's mirroring and the
//! view's. See `docs/specs/backface-culling.md`.
//!
//! Front faces are counter-clockwise on screen (wgpu's default `FrontFace::Ccw`). A single-sided draw keeps them; a
//! mirroring instance or view reverses the winding on screen, so it keeps the clockwise faces instead, and both
//! mirrors together cancel.

use blackbox_gfx::DrawRange;

/// Which faces of a draw are rasterised.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(super) enum Face {
    /// Both faces: two-sided draws.
    #[default]
    Both,
    /// The counter-clockwise faces only (back faces culled).
    Front,
    /// The clockwise faces only (front faces culled): a single-sided draw seen through one mirror.
    Back,
}

/// Every face setting, in pipeline-index order.
pub(super) const FACES: [Face; 3] = [Face::Both, Face::Front, Face::Back];

impl Face {
    /// The faces a draw keeps for an instance that mirrors (`mirrored`) or not, before the view.
    pub(super) fn of(draw: &DrawRange, mirrored: bool) -> Self {
        if draw.two_sided {
            return Face::Both;
        }
        if mirrored {
            return Face::Back;
        }
        Face::Front
    }

    /// The faces kept when seen through a view that mirrors (`mirrored_view`): the kept side swaps.
    pub(super) fn seen(self, mirrored_view: bool) -> Self {
        match (self, mirrored_view) {
            (Face::Front, true) => Face::Back,
            (Face::Back, true) => Face::Front,
            (face, _) => face,
        }
    }

    /// Position in [`FACES`].
    pub(super) fn index(self) -> usize {
        self as usize
    }

    /// The pipeline's cull mode.
    pub(super) fn cull_mode(self) -> Option<wgpu::Face> {
        match self {
            Face::Both => None,
            Face::Front => Some(wgpu::Face::Back),
            Face::Back => Some(wgpu::Face::Front),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_gfx::{BlendMode, Shading};

    fn draw(two_sided: bool) -> DrawRange {
        DrawRange {
            first_index: 0,
            index_count: 3,
            base_vertex: 0,
            texture: None,
            blend: BlendMode::Opaque,
            shading: Shading::Lit,
            two_sided,
        }
    }

    #[test]
    fn two_sided_draws_keep_both_faces_through_any_mirror() {
        for (mirrored, view) in [(false, false), (true, false), (false, true), (true, true)] {
            assert_eq!(Face::of(&draw(true), mirrored).seen(view), Face::Both);
        }
    }

    #[test]
    fn each_mirror_swaps_the_kept_side_and_two_cancel() {
        let single = draw(false);
        assert_eq!(Face::of(&single, false).seen(false), Face::Front);
        assert_eq!(Face::of(&single, true).seen(false), Face::Back);
        assert_eq!(Face::of(&single, false).seen(true), Face::Back);
        assert_eq!(Face::of(&single, true).seen(true), Face::Front);
    }

    #[test]
    fn faces_map_to_the_cull_mode_of_the_other_side() {
        assert_eq!(Face::Both.cull_mode(), None);
        assert_eq!(Face::Front.cull_mode(), Some(wgpu::Face::Back));
        assert_eq!(Face::Back.cull_mode(), Some(wgpu::Face::Front));
        assert!(FACES.iter().enumerate().all(|(i, f)| f.index() == i));
    }
}
