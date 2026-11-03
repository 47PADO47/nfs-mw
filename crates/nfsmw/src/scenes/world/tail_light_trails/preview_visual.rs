//! Installed-game validation of the stationary F8 preview.
use crate::scenes::world::*;
use crate::settings::TailPreview;
use crate::viewer::Scene;

pub(super) fn check(scene: &mut WorldScene, renderer: &mut blackbox_render::Renderer, aspect: f32) {
    let idle = ActionState::default();
    let position = scene.players.lead().unwrap().physics_pose().position;
    for lamp in scene.players.lead().unwrap().lamp_source().0.lamps() {
        if lamp.kind == car_lights::Kind::Tail {
            println!("lens at {:?}: {:?}", lamp.at, scene.players.lead().unwrap().lamp_source().0.aperture(lamp.at));
        }
    }
    let nos = scene.players.lead_mut().unwrap().sim_mut().unwrap().telemetry().nos;
    for (label, preset) in [
        ("mw", crate::settings::TrailPreset::EnhancedMw),
        ("exposure", crate::settings::TrailPreset::LongExposure),
        ("pursuit", crate::settings::TrailPreset::Pursuit),
    ] {
        scene.tuning.tail_trail_style = preset.apply(scene.tuning.tail_trail_style);
        for bend in [0, 80, -80] {
            scene.set_tail_preview(Some(TailPreview { bend, ..Default::default() }));
            for _ in 0..3 {
                std::thread::sleep(std::time::Duration::from_millis(10));
                scene.update(renderer, &idle, 1.0 / 60.0);
            }
            let drive = scene.players.lead_mut().unwrap();
            assert_eq!(drive.physics_pose().position, position, "preview must not move physics");
            assert!(!drive.nos_burning());
            assert_eq!(drive.sim_mut().unwrap().telemetry().nos, nos, "preview must not consume NOS");
            assert!(drive.lamp_source().3.nos_glow > 0.0);
            assert!(drive.tail_trails.time() > 0.02, "paused preview must animate");
            assert!(drive.tail_trails.path(glam::Vec3::new(-2.0, 0.5, 0.7), 100, 100).len() > 2);
            let (frame, instances) = scene.frame(aspect);
            renderer.render(&frame, instances).unwrap();
            let image = renderer.read_output().unwrap().unwrap();
            super::visual_tests::save(&format!("{label}-bend-{bend}"), &image);
        }
    }
    scene.set_tail_preview(None);
    let drive = scene.players.lead().unwrap();
    assert!(!drive.tail_trails.preview_active());
    assert!(drive.tail_trails.path(glam::Vec3::ZERO, 100, 100).is_empty());
    assert_eq!(drive.lamp_source().3.nos_glow, 0.0);
    scene.update(renderer, &idle, 1.0 / 60.0);
    println!(
        "Stationary preview, both bend directions, wall-clock animation, no NOS use, and immediate cleanup passed."
    );
}
