use std::sync::Arc;

use super::{Gpu, near};
use crate::gpu::post::{PostChain, colour_cube};
use crate::{PostSettings, Tonemap};
use blackbox_gfx::{ColourCube, ColourGrade};

/// A cube that swaps and bends the channels, so a wrong axis or channel order shows.
fn bent() -> ColourCube {
    let n = 16usize;
    let mut rgb = Vec::new();
    for z in 0..n {
        for y in 0..n {
            for x in 0..n {
                let (r, g, b) = (x as f32 / 15.0, y as f32 / 15.0, z as f32 / 15.0);
                rgb.extend([g, b * b, 1.0 - r].map(|v| (v * 255.0).round() as u8));
            }
        }
    }
    ColourCube::new(16, rgb).unwrap()
}

/// A chart: red from the column, green from the row, blue 0.4.
const CHART: &str = "return vec4<f32>(floor(pos.x) / 15.0, floor(pos.y) / 15.0, 0.4, 1.0);";

#[test]
#[ignore = "needs a Vulkan GPU"]
fn colour_cube_vulkan() {
    let _guard = crate::gpu::test_support::serial();
    let gpu = Gpu::new(wgpu::Backends::VULKAN);
    let mut chain = PostChain::new(&gpu.device, None);
    // Nothing is in the chain until a grade comes, and a grade that draws nothing is no change.
    assert_eq!(chain.set_colour_grade(&gpu.device, &gpu.queue, None), None);
    assert!(chain.names().is_empty() && !chain.has_passes());
    let identity = ColourGrade::single(Arc::new(ColourCube::identity(16).unwrap()), 1.0);
    let off = ColourGrade { strength: 0.0, ..identity.clone() };
    assert_eq!(chain.set_colour_grade(&gpu.device, &gpu.queue, Some(&off)), None);

    // The identity cube leaves the chart as it is, to a 255th.
    assert_eq!(chain.set_colour_grade(&gpu.device, &gpu.queue, Some(&identity)), Some(true));
    assert_eq!(chain.names(), [colour_cube::NAME]);
    assert_eq!(
        chain.set_colour_grade(&gpu.device, &gpu.queue, Some(&identity)),
        None,
        "the same grade changes nothing"
    );
    let plain = gpu.run(&mut chain, (16, 16), CHART);
    for y in 0..16usize {
        for x in 0..16usize {
            let expected = [(x * 17) as u8, (y * 17) as u8, 102, 255];
            assert!(near(plain[y * 16 + x], expected, 1), "({x},{y}) {:?} vs {expected:?}", plain[y * 16 + x]);
        }
    }

    // A cube that bends the colours matches the CPU reference.
    let grade = ColourGrade::single(Arc::new(bent()), 1.0);
    assert_eq!(chain.set_colour_grade(&gpu.device, &gpu.queue, Some(&grade)), Some(false), "still grading");
    let graded = gpu.run(&mut chain, (16, 16), CHART);
    for y in 0..16usize {
        for x in 0..16usize {
            let want =
                grade.apply([x as f32 / 15.0, y as f32 / 15.0, 0.4]).map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
            assert!(
                near(graded[y * 16 + x], [want[0], want[1], want[2], 255], 2),
                "({x},{y}) {:?} vs {want:?}",
                graded[y * 16 + x]
            );
        }
    }

    // After tone mapping: the pass is the last of the effects and the chain keeps tone mapping first.
    chain.set_effects(&gpu.device, PostSettings { tonemap: Tonemap::Aces, ..Default::default() });
    let names = chain.names();
    assert_eq!(names.last(), Some(&colour_cube::NAME), "{names:?}");
    assert!(names.contains(&"tonemap"));

    // No grade takes the work out of the chain (the pass stays, inactive).
    assert_eq!(chain.set_colour_grade(&gpu.device, &gpu.queue, None), Some(true));
    assert!(!chain.names().contains(&colour_cube::NAME));
}
