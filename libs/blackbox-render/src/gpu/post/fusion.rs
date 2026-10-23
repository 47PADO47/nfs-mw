//! Fusing the tone-mapping pass into the pass before it. Bloom's composite and the colour treatment's composite
//! each write a full-size image that the tone mapping then reads back and rewrites; run as one draw, the pair
//! costs one full-screen pass less. The treatment cannot join the bloom: the bloom chain reads the graded image
//! itself, so the grade has to be written first.
//!
//! The passes stay in the chain under their own names (so the settings, `names()` and the tests see the same
//! list); only the encoding of a frame skips the absorbed pass.

use super::PostPass;

/// Let each pass of `chain` that can run the tone mapping of the pass right after it do so, and take that pass
/// out of `chain`. A pass that is not followed by a tone-mapping pass is told so (`absorb_tonemap(None)`), which
/// clears what an earlier frame's fusion left in it. With `enabled` false nothing is fused.
pub(super) fn fuse_tonemap(chain: &mut Vec<&mut dyn PostPass>, enabled: bool) {
    let mut index = 0;
    while index < chain.len() {
        let next = chain.get(index + 1).and_then(|pass| pass.tonemap_exposure()).filter(|_| enabled);
        let fused = chain[index].absorb_tonemap(next);
        index += 1;
        if fused && next.is_some() {
            chain.remove(index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::post::{PassContext, PassIo};

    /// A pass that records what it was told.
    struct Probe {
        name: &'static str,
        exposure: Option<f32>,
        absorbs: bool,
        told: Option<Option<f32>>,
    }

    impl Probe {
        fn new(name: &'static str, exposure: Option<f32>, absorbs: bool) -> Self {
            Self { name, exposure, absorbs, told: None }
        }
    }

    impl PostPass for Probe {
        fn name(&self) -> &'static str {
            self.name
        }
        fn tonemap_exposure(&self) -> Option<f32> {
            self.exposure
        }
        fn absorb_tonemap(&mut self, exposure: Option<f32>) -> bool {
            self.told = Some(exposure);
            self.absorbs
        }
        fn encode(&mut self, _: &PassContext<'_>, _: &PassIo<'_>, _: &mut wgpu::CommandEncoder) {}
    }

    fn names(chain: &[&mut dyn PostPass]) -> Vec<&'static str> {
        chain.iter().map(|pass| pass.name()).collect()
    }

    #[test]
    fn a_tonemap_after_a_pass_that_absorbs_it_leaves_the_chain() {
        let mut bloom = Probe::new("bloom", None, true);
        let mut tonemap = Probe::new("tonemap", Some(2.0), false);
        let mut fxaa = Probe::new("fxaa", None, false);
        let mut chain: Vec<&mut dyn PostPass> = vec![&mut bloom, &mut tonemap, &mut fxaa];
        fuse_tonemap(&mut chain, true);
        assert_eq!(names(&chain), ["bloom", "fxaa"]);
        drop(chain);
        assert_eq!(bloom.told, Some(Some(2.0)));
        assert_eq!(fxaa.told, Some(None), "a pass with no tone mapping after it is told to clear");
    }

    #[test]
    fn a_pass_that_cannot_absorb_keeps_the_tonemap() {
        let mut fxaa = Probe::new("fxaa", None, false);
        let mut tonemap = Probe::new("tonemap", Some(1.0), false);
        let mut chain: Vec<&mut dyn PostPass> = vec![&mut fxaa, &mut tonemap];
        fuse_tonemap(&mut chain, true);
        assert_eq!(names(&chain), ["fxaa", "tonemap"]);
    }

    #[test]
    fn a_leading_tonemap_and_two_in_a_row_are_not_lost() {
        let mut first = Probe::new("tonemap", Some(1.0), false);
        let mut treatment = Probe::new("scene treatment", None, true);
        let mut second = Probe::new("tonemap", Some(3.0), false);
        let mut chain: Vec<&mut dyn PostPass> = vec![&mut first, &mut treatment, &mut second];
        fuse_tonemap(&mut chain, true);
        assert_eq!(names(&chain), ["tonemap", "scene treatment"]);
        drop(chain);
        assert_eq!(treatment.told, Some(Some(3.0)));
    }

    #[test]
    fn disabled_fusion_keeps_every_pass_and_clears_the_absorbers() {
        let mut bloom = Probe::new("bloom", None, true);
        let mut tonemap = Probe::new("tonemap", Some(2.0), false);
        let mut chain: Vec<&mut dyn PostPass> = vec![&mut bloom, &mut tonemap];
        fuse_tonemap(&mut chain, false);
        assert_eq!(names(&chain), ["bloom", "tonemap"]);
        drop(chain);
        assert_eq!(bloom.told, Some(None));
    }

    #[test]
    fn an_empty_chain_is_fine() {
        let mut chain: Vec<&mut dyn PostPass> = Vec::new();
        fuse_tonemap(&mut chain, true);
        assert!(chain.is_empty());
    }
}
