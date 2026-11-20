//! The kept static depth of still lamps must give the pixels of a full redraw, and must actually be reused.
use super::*;
use crate::gpu::test_support;
use blackbox_gfx::{InstanceKey, LocalLight, RenderBackend};

const SIZE: (u32, u32) = (240, 160);

fn lamp(x: f32) -> LocalLight {
    LocalLight {
        position: [x, -1.0, 5.0],
        range: 12.0,
        direction: [-0.2, 0.2, -1.0],
        outer_cos: 0.2,
        inner_cos: 0.7,
        color: [1.8, 1.5, 1.2],
        casts_shadow: true,
        shadow_exclude: None,
        upper_plane: None,
    }
}

/// One step of the sequence: the casters' positions (`None` leaves one out) and the lamp's position.
type Step = ([Option<f32>; 3], f32);

const STEPS: [Step; 11] = [
    ([Some(0.0), Some(2.5), Some(-2.5)], 1.0),
    ([Some(0.0), Some(2.5), Some(-2.5)], 1.0),
    ([Some(0.0), Some(2.5), Some(-2.5)], 1.0),
    ([Some(0.0), Some(2.5), Some(-2.5)], 1.0),
    // One caster moves, then rests where it stopped.
    ([Some(0.8), Some(2.5), Some(-2.5)], 1.0),
    ([Some(1.6), Some(2.5), Some(-2.5)], 1.0),
    ([Some(1.6), Some(2.5), Some(-2.5)], 1.0),
    // A resting caster disappears.
    ([Some(1.6), None, Some(-2.5)], 1.0),
    ([Some(1.6), None, Some(-2.5)], 1.0),
    // The lamp moves, then rests.
    ([Some(1.6), None, Some(-2.5)], 1.6),
    ([Some(1.6), None, Some(-2.5)], 1.6),
];

/// Renders the sequence, with keyed casters (which may be kept) or anonymous ones (which never are). Returns each
/// step's image and how many lamps reused kept depth in it.
fn run(keyed: bool) -> Vec<(Vec<u8>, u32)> {
    let mut r = test_support::headless(SIZE).expect("lamp cache GPU");
    let (frame, ground, casters) = super::tests::scene(&mut r);
    STEPS
        .iter()
        .map(|&(positions, light)| {
            let placed: Vec<Instance> = positions
                .iter()
                .enumerate()
                .filter_map(|(index, x)| {
                    let transform = Mat4::from_translation(Vec3::new((*x)?, index as f32 - 1.0, 0.0));
                    Some(match keyed {
                        true => Instance::keyed(casters[0].mesh, transform, InstanceKey::new(7, index as u32)),
                        false => Instance::new(casters[0].mesh, transform),
                    })
                })
                .collect();
            r.set_shadow_casters(Some(&placed));
            r.set_local_lights(&[lamp(light)]);
            let image = r.capture(SIZE.0, SIZE.1, &frame, &ground).unwrap();
            (image, r.local_shadows.as_ref().map_or(0, |local| local.reused))
        })
        .collect()
}

#[test]
#[ignore = "requires GPU"]
fn kept_lamp_depth_matches_a_full_redraw_and_is_reused() {
    let _gpu = test_support::serial();
    let fresh = run(false);
    let kept = run(true);
    assert!(fresh.iter().all(|(_, reused)| *reused == 0), "anonymous casters are never kept");
    for (step, (fresh, kept)) in fresh.iter().zip(&kept).enumerate() {
        assert_eq!(fresh.0, kept.0, "step {step} must match a full redraw");
    }
    assert_ne!(fresh[3].0, fresh[4].0, "the moving caster must change the image");
    assert_ne!(fresh[6].0, fresh[7].0, "the removed caster must change the image");
    assert_ne!(fresh[8].0, fresh[9].0, "the moved lamp must change the image");
    let reused: Vec<u32> = kept.iter().map(|(_, reused)| *reused).collect();
    // A still lamp asks about its casters in its second frame, keeps the resting ones in its third and reuses them
    // from its fourth. A caster that starts or stops moving, a removed caster and a moved lamp each redraw once.
    assert_eq!(reused, [0, 0, 0, 1, 0, 1, 0, 0, 1, 0, 0]);
}
