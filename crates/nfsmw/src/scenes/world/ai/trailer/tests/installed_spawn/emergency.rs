//! Installed cop markers and authored emergency profiles reach the flare renderer.

use super::*;
use crate::scenes::world::car_light_flares::{FlareView, emergency};
use crate::scenes::world::car_lights::LampState;
use std::f32::consts::PI;

fn draw(rig: &CarRig, time: f32, enabled: u32, broken: u32, view: FlareView) -> Vec<blackbox_gfx::TexturedEffect> {
    let mut out = Vec::new();
    rig.flares().draw(
        rig.lights(),
        glam::Mat4::IDENTITY,
        LampState { time, emergency: enabled, broken: broken | 0xff, ..Default::default() },
        view,
        &mut out,
    );
    out
}

#[test]
#[ignore = "needs installed cop markers, authored emergency profiles and textures; CPU-only"]
fn installed_cop_emergency_flares_follow_native_phase_damage_and_draw_order() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let data = PhysicsData::load(&dir).unwrap();
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    let raw =
        nfsmw_data::car::load(&dir, "COPMIDSIZE", &LoadOptions { lod: 'A', all_parts: false, preset: None }).unwrap();
    let profiles = &raw.flare_profiles;
    for profile in [
        &profiles.cop_red,
        &profiles.cop_blue,
        &profiles.cop_white,
        &profiles.cop_bright_red,
        &profiles.cop_bright_blue,
        &profiles.cop_orange,
    ] {
        assert_eq!(profile.len(), 1, "each installed emergency flare profile must decode");
        assert!(raw.textures.contains_key(&profile[0].texture), "authored emergency texture must load");
    }
    assert!(profiles.cop_red[0].color[3] > 0.0 && profiles.cop_blue[0].color[3] > 0.0);
    // Some stock bright/white profiles intentionally have zero colour. Their decoded presence
    // witnesses discovery, not visible output; preserve those values rather than brighten them.
    let model = model(&dir, &data, "copmidsize", &mut renderer);
    let lamps = model.rig.lights().lamps();
    for (mask, on_time, off_time, channel) in [(emergency::RED, 0.0, PI / 8.0, 0), (emergency::BLUE, PI / 8.0, 0.0, 1)]
    {
        let lamp = lamps.iter().find(|lamp| lamp.flags & mask != 0).expect("stock cop has red and blue markers");
        let direction = lamp.direction.normalize_or(Vec3::X);
        let view =
            FlareView { camera: lamp.at + direction * 20.0, forward: -direction, fov_degrees: 70.0, height: 1080 };
        assert!(emergency::intensities(on_time, mask, 0)[channel] > 0.99);
        assert_eq!(emergency::intensities(off_time, mask, 0)[channel], 0.0);
        let enabled = draw(&model.rig, on_time, mask, 0, view);
        let disabled = draw(&model.rig, on_time, 0, 0, view);
        let off_phase = draw(&model.rig, off_time, mask, 0, view);
        let damaged = draw(&model.rig, on_time, mask, mask, view);
        let vertices =
            |batches: &[blackbox_gfx::TexturedEffect]| batches.iter().map(|b| b.vertices.len()).sum::<usize>();
        assert!(vertices(&enabled) > vertices(&disabled), "installed colour must emit visible authored flare geometry");
        assert_eq!(vertices(&off_phase), vertices(&disabled));
        assert_eq!(vertices(&damaged), vertices(&disabled));
        let behind = FlareView { camera: lamp.at - direction * 20.0, forward: direction, ..view };
        assert!(
            vertices(&draw(&model.rig, on_time, mask, 0, behind)) > vertices(&draw(&model.rig, on_time, 0, 0, behind)),
            "native emergency marker flag bypasses the ordinary facing-dot rejection"
        );
        // Rendering intervening frames must not change phase or consume state at this sample.
        for fps in [30, 60, 144] {
            for frame in 0..fps {
                let _ = draw(&model.rig, frame as f32 / fps as f32, mask, 0, view);
            }
            let repeated = draw(&model.rig, on_time, mask, 0, view);
            assert_eq!(repeated.len(), enabled.len());
            for (a, b) in repeated.iter().zip(&enabled) {
                assert_eq!(a.texture, b.texture);
                assert_eq!(a.vertices, b.vertices, "render cadence must not advance a lamp toggle");
            }
        }
    }
}
