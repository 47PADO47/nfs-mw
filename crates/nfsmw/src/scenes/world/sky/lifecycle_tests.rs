//! Renderer reuse must release each old sky while failed replacements keep it alive.

use super::*;
use crate::app::{Host, test_backend::NullBackend};
use crate::frontend::{MenuScene, Pausable, PauseFlag};
use crate::input::ActionState;
use crate::viewer::Scene;
use blackbox_gfx::FrameParams;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

struct OwnedSky {
    sky: Sky,
    fail: bool,
    closed: Arc<AtomicUsize>,
}

impl OwnedSky {
    fn new(fail: bool) -> Self {
        Self { sky: Sky::default(), fail, closed: Default::default() }
    }
}

impl Scene for OwnedSky {
    fn title(&self) -> String {
        "sky resource fixture".into()
    }

    fn init(&mut self, renderer: &mut dyn RenderBackend) -> anyhow::Result<()> {
        let mut meshes = Vec::new();
        for kind in [SkyLayerKind::Base, SkyLayerKind::Cloud2, SkyLayerKind::Cloud3] {
            self.sky.materials.push(
                renderer
                    .create_sky_material(&SkyMaterialDesc {
                        kind,
                        diffuse: TextureHandle::from_raw(0),
                        miscellaneous: TextureHandle::from_raw(0),
                    })
                    .unwrap(),
            );
            meshes.push(renderer.create_mesh(&MeshDesc {
                label: "owned sky fixture",
                vertices: &[],
                secondary_uvs: None,
                tangents: None,
                indices: &[],
                draws: vec![],
            }));
        }
        self.sky.layers = Some(meshes.try_into().unwrap());
        if self.fail {
            anyhow::bail!("injected scene initialization failure");
        }
        Ok(())
    }

    fn close(&mut self, renderer: &mut dyn RenderBackend) {
        self.closed.fetch_add(1, Ordering::Relaxed);
        self.sky.release(renderer);
    }

    fn update(&mut self, _: &mut dyn RenderBackend, _: &ActionState, _: f32) {}
    fn frame(&mut self, _: f32) -> (FrameParams, &[Instance]) {
        unreachable!("lifecycle fixture does not draw")
    }
}

#[test]
fn failed_replacement_retains_old_sky_and_successful_replacement_releases_once_through_pause_wrapper() {
    let mut renderer = NullBackend::new(crate::settings::test_caps::native()).with_sky_tracking();
    let counts = renderer.resources.clone();
    let mut old = OwnedSky::new(false);
    let old_closed = old.closed.clone();
    old.init(&mut renderer).unwrap();
    let old = Pausable::new(Box::new(old), PauseFlag::default());
    let mut host =
        Host::new(Box::new(old), &crate::settings::Settings::from(crate::settings::Partial::default()), None);
    host.renderer = Some(Box::new(renderer));

    let failed = OwnedSky::new(true);
    let failed_closed = failed.closed.clone();
    assert!(host.replace_scene(Box::new(failed)).is_err());
    assert_eq!(host.scene_changes, 0);
    assert_eq!(old_closed.load(Ordering::Relaxed), 0);
    assert_eq!(failed_closed.load(Ordering::Relaxed), 1);
    {
        let values = counts.lock().unwrap();
        assert_eq!((values.meshes, values.sky_materials), (3, 3));
        assert_eq!((values.mesh_destructions, values.sky_destructions), (3, 3));
    }

    let next = OwnedSky::new(false);
    let next_closed = next.closed.clone();
    host.replace_scene(Box::new(next)).unwrap();
    assert_eq!(old_closed.load(Ordering::Relaxed), 1);
    assert_eq!(next_closed.load(Ordering::Relaxed), 0);
    {
        let values = counts.lock().unwrap();
        assert_eq!((values.meshes, values.sky_materials), (3, 3));
        assert_eq!((values.mesh_destructions, values.sky_destructions), (6, 6));
    }
    host.replace_scene(Box::new(MenuScene::default())).unwrap();
    assert_eq!(next_closed.load(Ordering::Relaxed), 1);
    let values = counts.lock().unwrap();
    assert_eq!((values.meshes, values.sky_materials), (0, 0));
    assert_eq!((values.mesh_destructions, values.sky_destructions), (9, 9));
}

#[test]
fn repeated_sky_release_is_idempotent() {
    let mut renderer = NullBackend::new(crate::settings::test_caps::native()).with_sky_tracking();
    let mut fixture = OwnedSky::new(false);
    fixture.init(&mut renderer).unwrap();
    fixture.sky.release(&mut renderer);
    fixture.sky.release(&mut renderer);
    let counts = renderer.resources.lock().unwrap();
    assert_eq!((counts.meshes, counts.sky_materials), (0, 0));
    assert_eq!((counts.mesh_destructions, counts.sky_destructions), (3, 3));
}
