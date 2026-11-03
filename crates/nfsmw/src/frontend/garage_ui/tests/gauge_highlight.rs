use super::*;
use blackbox_feng::fe_hash_upper;
use nfsmw_data::car::customization::ColorChoice;

#[test]
#[ignore = "requires installed original garage palettes"]
fn batch15_gauge_frame_follows_swatch_across_rows_channels_and_pages() {
    for backroom in [false, true] {
        let mut initial = model();
        let parts = initial.cars[0].parts.as_mut().unwrap();
        parts.gauge_skins.push(1);
        for palette in &mut parts.color_palettes {
            *palette = (0..12)
                .map(|i| ColorChoice { hash: 100 + i, rgb: [i as u8 * 15, 80, 120], upgrade_level: 0 })
                .collect();
        }
        initial.draft.gauges.skin = 1;
        let mut h = Harness::open(Page::GaugeColors { backroom }, initial);
        let at = |h: &Harness, hash| {
            h.screens.trees().pop().unwrap().nodes.iter().find(|n| n.name_hash == hash).unwrap().position
        };
        let frame = 0xb893_252a;
        let offset = glam::Vec3::ZERO;
        let delta = at(&h, frame) - at(&h, fe_hash_upper("Color_1"));
        assert!(
            delta.truncate().length() < 0.01,
            "frame must center on the swatch, not retain the authored offset: {delta:?}"
        );
        for (mask, index) in [(pad::DOWN, 1), (pad::DOWN, 2), (pad::RIGHT, 0), (pad::LEFT, 2), (pad::UP, 1)] {
            h.press(mask);
            let swatch = fe_hash_upper(&format!("Color_{}", index + 1));
            let delta = at(&h, frame) - at(&h, swatch) - offset;
            assert!(delta.truncate().length() < 0.01, "frame must follow selected swatch: {delta:?}");
        }
        for _ in 0..10 {
            h.press(pad::DOWN);
        }
        assert!((at(&h, frame) - at(&h, fe_hash_upper("Color_2")) - offset).truncate().length() < 0.01);
    }
}

#[test]
#[ignore = "requires installed gauge palettes and Vulkan GPU"]
fn pass_a_gpu_gauge_palette_captures_selected_rows() {
    use crate::input::{Bindings, InputDevice};
    for backroom in [false, true] {
        let mut initial = model();
        let parts = initial.cars[0].parts.as_mut().unwrap();
        parts.gauge_skins.push(1);
        for palette in &mut parts.color_palettes {
            *palette = (0..10)
                .map(|i| ColorChoice { hash: 100 + i, rgb: [i as u8 * 20, 80, 120], upgrade_level: 0 })
                .collect();
        }
        initial.draft.gauges.skin = 1;
        let mut h = Harness::open(Page::GaugeColors { backroom }, initial);
        for row in 0..3 {
            // Capture a bright point of the authored pulse without changing its animation.
            for _ in 0..120 {
                let tree = h.screens.trees().pop().unwrap();
                if tree.nodes.iter().find(|n| n.name_hash == 0xb893_252a).unwrap().world_colour[3] >= 220 {
                    break;
                }
                h.press(0);
            }
            let tree = h.screens.presented_trees(&Bindings::default(), InputDevice::Xbox).pop().unwrap();
            crate::frontend::tests::rendered_playtest::render(
                &tree,
                h.screens.assets(),
                (1920, 1080),
                &format!("gauge-{backroom}-{row}"),
            );
            h.press(pad::DOWN);
        }
    }
}
