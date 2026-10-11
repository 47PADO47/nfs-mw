//! A recording backend: proves the trait is object safe and implementable, and that callers can drive it.

use std::collections::HashMap;

use glam::{Mat4, Vec3};

use super::*;
use crate::{BlendMode, DrawRange, GraphicsApi, PixelFormat, Shading, Vertex};

struct Recorder {
    info: BackendInfo,
    caps: Capabilities,
    settings: GraphicsSettings,
    size: [u32; 2],
    next: usize,
    textures: usize,
    meshes: usize,
    frames: Vec<usize>,
    captures: HashMap<u64, RgbaImage>,
}

impl Recorder {
    fn new() -> Self {
        let mut caps = Capabilities::baseline("recorder", GraphicsApi::Vulkan);
        caps.upscalers = UpscalerSet::of(&[crate::Upscaler::Off, crate::Upscaler::Bilinear]);
        caps.render_scale = Capabilities::full_render_scale();
        let info = BackendInfo {
            renderer: "recorder",
            api: GraphicsApi::Vulkan,
            adapter: "none".into(),
            driver: String::new(),
        };
        Self {
            info,
            caps,
            settings: GraphicsSettings::default(),
            size: [1280, 720],
            next: 1,
            textures: 0,
            meshes: 0,
            frames: Vec::new(),
            captures: HashMap::new(),
        }
    }

    fn take(&mut self) -> usize {
        self.next += 1;
        self.next - 1
    }
}

impl RenderBackend for Recorder {
    fn info(&self) -> &BackendInfo {
        &self.info
    }
    fn capabilities(&self) -> &Capabilities {
        &self.caps
    }
    fn resize(&mut self, size: [u32; 2]) {
        if size.contains(&0) {
            return;
        }
        self.size = size;
    }
    fn surface_size(&self) -> [u32; 2] {
        self.size
    }
    fn set_vsync(&mut self, _vsync: bool) {}
    fn apply_graphics(&mut self, requested: &GraphicsSettings) -> Resolved {
        let resolved = crate::resolve(requested, &self.caps);
        self.settings = resolved.effective;
        resolved
    }
    fn graphics(&self) -> &GraphicsSettings {
        &self.settings
    }
    fn render_size(&self) -> [u32; 2] {
        let (w, h) = crate::scaled_size((self.size[0], self.size[1]), self.settings.render_scale);
        [w, h]
    }
    fn create_texture(&mut self, _desc: &TextureDesc<'_>) -> TextureHandle {
        self.textures += 1;
        TextureHandle::from_raw(self.take())
    }
    fn destroy_texture(&mut self, _handle: TextureHandle) {
        self.textures -= 1;
    }
    fn redirect_texture(&mut self, _from: TextureHandle, _to: Option<TextureHandle>) {}
    fn create_mesh(&mut self, _desc: &MeshDesc<'_>) -> MeshHandle {
        self.meshes += 1;
        MeshHandle::from_raw(self.take())
    }
    fn destroy_mesh(&mut self, _handle: MeshHandle) {
        self.meshes -= 1;
    }
    fn create_glossy_material(&mut self, _material: &GlossyMaterial) -> GlossyMaterialHandle {
        GlossyMaterialHandle::from_raw(self.take())
    }
    fn destroy_glossy_material(&mut self, _handle: GlossyMaterialHandle) {}
    fn set_lighting_rig(&mut self, _rig: &LightingRig) {}
    fn set_environment(&mut self, _environment: Environment<'_>) {}
    fn set_effects(&mut self, _layer: &EffectLayer) {}
    fn update_ui_texture(&mut self, _patch: &UiTexturePatch<'_>) {}
    fn free_ui_texture(&mut self, _id: UiTextureId) {}
    fn set_ui_layer(&mut self, _layer: UiLayer) {}
    fn render(&mut self, _frame: &FrameParams, instances: &[Instance]) -> Result<FrameStatus, RenderError> {
        self.frames.push(instances.len());
        Ok(FrameStatus::Presented)
    }
    fn request_capture(
        &mut self,
        size: [u32; 2],
        _frame: &FrameParams,
        _instances: &[Instance],
    ) -> Result<CaptureId, RenderError> {
        let image = RgbaImage::new(size[0], size[1], vec![0; (size[0] * size[1] * 4) as usize])?;
        let id = self.take() as u64;
        self.captures.insert(id, image);
        Ok(CaptureId::from_raw(id))
    }
    fn poll_capture(&mut self, id: CaptureId) -> Option<Result<RgbaImage, RenderError>> {
        self.captures.remove(&id.raw()).map(Ok)
    }
    fn stats(&self) -> RenderStats {
        RenderStats { meshes: self.meshes, textures: self.textures, ..RenderStats::default() }
    }
}

fn frame() -> FrameParams {
    FrameParams {
        view: Mat4::IDENTITY,
        projection: crate::Projection::Identity,
        camera_position: Vec3::ZERO,
        light_dir: Vec3::NEG_Z,
        clear_color: [0.0; 3],
        fog: None,
        camera_cut: false,
    }
}

/// What the scene code in `blackbox-scene` does: it only knows the trait object.
fn upload(backend: &mut dyn RenderBackend) -> (TextureHandle, MeshHandle) {
    let mip = [255u8; 16];
    let texture = backend.create_texture(&TextureDesc {
        label: "t",
        width: 2,
        height: 2,
        format: PixelFormat::Rgba8,
        mips: vec![&mip],
    });
    let vertex = Vertex { position: [0.0; 3], normal: [0.0, 0.0, 1.0], color_bgra: [128; 4], uv: [0.0; 2] };
    let draw = DrawRange {
        first_index: 0,
        index_count: 3,
        base_vertex: 0,
        texture: Some(texture),
        blend: BlendMode::Opaque,
        shading: Shading::Prelit,
    };
    let mesh =
        backend.create_mesh(&MeshDesc { label: "m", vertices: &[vertex; 3], indices: &[0, 1, 2], draws: vec![draw] });
    (texture, mesh)
}

#[test]
fn a_backend_is_usable_as_a_trait_object() {
    let mut boxed: Box<dyn RenderBackend> = Box::new(Recorder::new());
    let (texture, mesh) = upload(boxed.as_mut());
    assert_ne!(texture.raw(), mesh.raw(), "handles are distinct");
    assert_eq!(boxed.stats().textures, 1);
    let instances = [Instance::new(mesh, Mat4::IDENTITY); 3];
    assert_eq!(boxed.render(&frame(), &instances).unwrap(), FrameStatus::Presented);
    boxed.destroy_mesh(mesh);
    boxed.destroy_texture(texture);
    assert_eq!(boxed.stats(), RenderStats::default());
}

#[test]
fn the_aspect_ratio_comes_from_the_surface_size() {
    let mut backend = Recorder::new();
    assert!((backend.aspect_ratio() - 16.0 / 9.0).abs() < 1e-6);
    backend.resize([800, 800]);
    assert_eq!(backend.aspect_ratio(), 1.0);
    backend.resize([0, 600]);
    assert_eq!(backend.surface_size(), [800, 800], "a zero size is ignored");
}

#[test]
fn applying_settings_returns_the_resolution_and_updates_the_effective_settings() {
    let mut backend = Recorder::new();
    let request = GraphicsSettings {
        upscaler: crate::Upscaler::Dlss,
        upscale_quality: crate::UpscaleQuality::Performance,
        ..GraphicsSettings::default()
    };
    let resolved = backend.apply_graphics(&request);
    assert!(!resolved.is_exact());
    assert_eq!(backend.graphics(), &resolved.effective);
    assert_eq!(backend.graphics().upscaler, crate::Upscaler::Bilinear);
    assert_eq!(backend.render_size(), [640, 360]);
}

#[test]
fn captures_are_requested_then_polled_once() {
    let mut backend = Recorder::new();
    let id = backend.request_capture([4, 2], &frame(), &[]).unwrap();
    let image = backend.poll_capture(id).expect("ready").unwrap();
    assert_eq!((image.width, image.height, image.rgba.len()), (4, 2, 32));
    assert!(backend.poll_capture(id).is_none(), "returned once");
}
