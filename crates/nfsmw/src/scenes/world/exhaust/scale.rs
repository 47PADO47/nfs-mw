//! How big the burning nitrous flame is drawn. The group's own emitters (`fxcar_nos`: a fire of 0.35 to 0.45 m and a glow that grows
//! to 1.5 m and is thrown out at 16 m/s) make a plume that fills the view behind the car. The nitrous of the 2012 game is a small
//! glow in each pipe, 0.3 m wide growing to 0.7 m at the peak of its alpha (`docs/formats/chameleon-nitrous.md`), so the host
//! draws the stock flame smaller. This is the rewrite's own choice, not a reading of the 2005 game; it changes the blue lanes
//! only, so the blow-off and the backfire keep the group's numbers.

use blackbox_particles::EmitterSpec;

/// The factor on the width of the nitrous sprites.
pub(super) const NITROUS_SIZE: f32 = 0.5;
/// The factor on the speed the nitrous sprites leave the pipe at. The glow's drag (quadratic) keeps its reach under a metre.
pub(super) const NITROUS_REACH: f32 = 0.15;

/// `spec` for the burning nitrous: narrower sprites that stay nearer the pipe.
pub(super) fn compact_nitrous(spec: &EmitterSpec) -> EmitterSpec {
    let mut spec = spec.clone();
    spec.size = spec.size.map(|width| width * NITROUS_SIZE);
    spec.speed *= NITROUS_REACH;
    spec.velocity_start *= NITROUS_REACH;
    spec.velocity_delta *= NITROUS_REACH;
    spec
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn the_nitrous_sprites_are_narrower_and_slower_and_everything_else_is_kept() {
        let spec = EmitterSpec {
            size: [0.0, 0.4, 1.0, 1.5],
            speed: 16.0,
            velocity_start: Vec3::new(1.0, 2.0, 3.0),
            velocity_delta: Vec3::splat(0.5),
            rate: 50.0,
            life: 0.5,
            ..EmitterSpec::default()
        };
        let small = compact_nitrous(&spec);
        assert_eq!(small.size, [0.0, 0.2, 0.5, 0.75]);
        assert!((small.speed - 2.4).abs() < 1e-5);
        assert!((small.velocity_start - Vec3::new(0.15, 0.3, 0.45)).length() < 1e-6);
        assert_eq!((small.rate, small.life), (50.0, 0.5));
        assert_eq!(spec.size[3], 1.5, "the source is untouched");
    }
}
