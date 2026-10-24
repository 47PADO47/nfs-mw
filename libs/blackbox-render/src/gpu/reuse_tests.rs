//! GPU checks of showing a kept scene image again.

use glam::{Mat4, Vec3};

use super::super::test_support;
use crate::FrameParams;
use blackbox_gfx::{Projection, RenderBackend, SceneReuse};

fn clear_frame(color: [f32; 3]) -> FrameParams {
    FrameParams {
        view: Mat4::IDENTITY,
        projection: Projection::Identity,
        camera_position: Vec3::ZERO,
        light_dir: Vec3::NEG_Z,
        clear_color: color,
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
    }
}

#[test]
#[ignore = "needs a GPU"]
fn a_reuse_frame_shows_the_kept_image_and_anything_else_ends_it() {
    let _gpu = test_support::serial();
    let Some(mut renderer) = test_support::headless((64, 36)) else { return };
    let pixel = |renderer: &mut crate::Renderer| renderer.read_output().unwrap().unwrap().pixel(30, 17);
    assert!(!renderer.scene_reuse_ready(), "nothing is kept yet");

    renderer.set_scene_reuse(SceneReuse::Keep);
    renderer.render(&clear_frame([0.2, 0.4, 0.6]), &[]).unwrap();
    assert!(renderer.scene_reuse_ready());
    assert_eq!(pixel(&mut renderer), Some([51, 102, 153, 255]));

    renderer.set_scene_reuse(SceneReuse::Reuse);
    renderer.render(&clear_frame([1.0, 0.0, 0.0]), &[]).unwrap();
    assert_eq!(pixel(&mut renderer), Some([51, 102, 153, 255]), "the scene is not drawn again");
    assert!(renderer.scene_reuse_ready(), "a reuse frame leaves the image kept");

    // A frame that asks for nothing draws normally and forgets the image.
    renderer.render(&clear_frame([1.0, 0.0, 0.0]), &[]).unwrap();
    assert_eq!(pixel(&mut renderer), Some([255, 0, 0, 255]));
    assert!(!renderer.scene_reuse_ready());

    // A reuse request with nothing kept draws the frame.
    renderer.set_scene_reuse(SceneReuse::Reuse);
    renderer.render(&clear_frame([0.0, 1.0, 0.0]), &[]).unwrap();
    assert_eq!(pixel(&mut renderer), Some([0, 255, 0, 255]));

    // A resize ends a kept image.
    renderer.set_scene_reuse(SceneReuse::Keep);
    renderer.render(&clear_frame([0.0, 0.0, 1.0]), &[]).unwrap();
    assert!(renderer.scene_reuse_ready());
    renderer.resize(32, 18);
    assert!(!renderer.scene_reuse_ready());
}
