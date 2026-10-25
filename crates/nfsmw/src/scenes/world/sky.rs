//! Dedicated camera-relative stock sky placement. Spec: `world-atmosphere.md`.
//! Native sky materials use the independently specified original shader arithmetic.

use blackbox_gfx::{
    BlendMode, DrawRange, Instance, InstanceKey, MeshDesc, MeshHandle, RenderBackend, Shading, SkyLayerKind,
    SkyMaterialDesc, SkyState, TextureHandle,
};
use blackbox_hash::bstring_hash;
use glam::{Mat4, Vec3};

use super::WorldScene;
use super::resident::SectionResources;

impl WorldScene {
    /// Refresh before captures, including frames whose simulation is paused.
    pub(super) fn update_sky(&mut self, renderer: &mut dyn RenderBackend) {
        let options = renderer.render_experiments();
        self.original_sky = options.original_world_materials || options.enhanced_sky;
        let mut sky = state(self.atmosphere.intensity(), self.clock);
        sky.enhanced = options.enhanced_sky;
        if sky.enhanced {
            sky.scroll += self.day.cloud_seek_seconds() * 0.02;
        }
        sky.sky_wind_speed = options.sky_wind_speed;
        sky.sky_sun_size = options.sky_sun_size;
        sky.sky_sun_brightness = options.sky_sun_brightness;
        sky.sky_brightness = options.sky_brightness;
        sky.sky_sunset_warmth = options.sky_sunset_warmth;
        renderer.set_sky_state(sky);
        let appearance = [
            ("SUNMAJORRAYS", 0.16, 1.0),
            ("SUNCENTER", 0.04, 1.4),
            ("SUNMINORRAYS", 0.09, 0.25),
            ("SUNHALO", 0.065, 0.035),
            ("SUNRING", 0.09, 0.025),
        ];
        let layers = appearance
            .into_iter()
            .filter_map(|(name, radius, brightness)| {
                let texture = self.residency.shared.materials.get(&bstring_hash(name))?.0;
                Some(blackbox_gfx::SunGlareLayer {
                    texture,
                    radius: radius * options.sky_sun_size * 0.5,
                    brightness,
                    rotation: 0.0,
                })
            })
            .collect();
        let dusk = (1.0 - (-self.sun.direction().z / 0.5)).clamp(0.0, 1.0);
        renderer.set_sun_glare(Some(blackbox_gfx::SunGlare {
            layers,
            colour: [1.0, 0.99 - 0.19 * dusk, 0.95 - 0.45 * dusk],
            strength: options.sky_sun_brightness,
            weather_visibility: (1.0 - sky.cloud_coverage).clamp(0.0, 1.0).powi(2),
            angular_radius: 0.27 * options.sky_sun_size,
        }));
    }
}

#[cfg(test)]
mod lifecycle_tests;

/// The renderer owns these meshes/materials for the shared world's lifetime.
#[derive(Default)]
pub(super) struct Sky {
    layers: Option<[MeshHandle; 3]>,
    materials: Vec<blackbox_gfx::SkyMaterialHandle>,
}

impl Sky {
    pub(super) fn upload<'a>(
        renderer: &mut dyn RenderBackend,
        solids: impl IntoIterator<Item = &'a blackbox_solid::Solid>,
        resources: &SectionResources,
    ) -> Self {
        let Some(solid) = solids.into_iter().find(|solid| solid.name == "SKYDOME_XENON") else {
            return Self::default();
        };
        match upload(renderer, solid, resources) {
            Some((layers, materials)) => Self { layers: Some(layers), materials },
            None => {
                log::warn!("original layered sky unavailable: using camera-relative legacy sky");
                Self::default()
            }
        }
    }

    pub(super) fn release(&mut self, renderer: &mut dyn RenderBackend) {
        if let Some(layers) = self.layers.take() {
            for mesh in layers {
                renderer.destroy_mesh(mesh);
            }
        }
        for material in self.materials.drain(..) {
            renderer.destroy_sky_material(material);
        }
    }

    pub(super) fn place(&self, resources: &SectionResources, instances: &mut Vec<Instance>, camera: Vec3, view: View) {
        remove_fixed(resources, instances);
        if let Some(layers) = self.layers {
            for (index, mesh) in layers.into_iter().enumerate() {
                instances.push(Instance::keyed(
                    mesh,
                    transform(camera, view),
                    InstanceKey::new(0x7FFF_FFFE, index as u32),
                ));
            }
            return;
        }
        let Some(&(mesh, _, _)) = resources.meshes.get(&bstring_hash("SKYDOME")) else { return };
        instances.push(Instance::keyed(mesh, transform(camera, view), InstanceKey::new(0x7FFF_FFFE, 0)));
    }
}

fn upload(
    renderer: &mut dyn RenderBackend,
    solid: &blackbox_solid::Solid,
    resources: &SectionResources,
) -> Option<([MeshHandle; 3], Vec<blackbox_gfx::SkyMaterialHandle>)> {
    let texture = |name: &str| resources.materials.get(&bstring_hash(name)).map(|entry| entry.0);
    let miscellaneous = [texture("SKY_NEXGEN_CAP_A")?, texture("SKY_NEXGEN_CLOUDS_B")?, texture("SKY_NEXGEN_CAP_B")?];
    let diffuse: Vec<_> = solid
        .groups
        .iter()
        .map(|group| {
            let hash = group.diffuse_texture(solid)?;
            resources.materials.get(&hash).map(|entry| entry.0)
        })
        .collect::<Option<_>>()?;
    if solid.vertices.is_empty()
        || solid.indices.is_empty()
        || solid.secondary_uvs.as_ref().is_none_or(|uvs| uvs.len() != solid.vertices.len())
        || solid.vertex_buffers.iter().any(|buffer| buffer.stride != 44)
    {
        return None;
    }
    let vertices = blackbox_scene::solid_vertices(solid);
    let mut materials = Vec::new();
    let mut layers = Vec::new();
    for (index, kind) in [SkyLayerKind::Base, SkyLayerKind::Cloud2, SkyLayerKind::Cloud3].into_iter().enumerate() {
        let mut draws = Vec::new();
        for (group, &diffuse) in solid.groups.iter().zip(&diffuse) {
            let desc = descriptor(kind, diffuse, miscellaneous);
            let Some(material) = renderer.create_sky_material(&desc) else {
                for mesh in layers {
                    renderer.destroy_mesh(mesh);
                }
                for material in materials {
                    renderer.destroy_sky_material(material);
                }
                return None;
            };
            materials.push(material);
            draws.push(DrawRange {
                first_index: group.first_index,
                index_count: group.num_indices,
                base_vertex: group.base_vertex as i32,
                texture: Some(desc.diffuse),
                blend: if index == 0 { BlendMode::Opaque } else { BlendMode::AlphaBlend },
                shading: Shading::SkyLayer(material),
                two_sided: true,
            });
        }
        layers.push(renderer.create_mesh(&MeshDesc {
            label: "original sky layer",
            vertices,
            secondary_uvs: solid.secondary_uvs.as_deref(),
            tangents: None,
            indices: &solid.indices,
            draws,
        }));
    }
    Some((layers.try_into().ok()?, materials))
}

fn descriptor(kind: SkyLayerKind, diffuse: TextureHandle, miscellaneous: [TextureHandle; 3]) -> SkyMaterialDesc {
    match kind {
        SkyLayerKind::Base => SkyMaterialDesc { kind, diffuse, miscellaneous: miscellaneous[0] },
        SkyLayerKind::Cloud2 => SkyMaterialDesc { kind, diffuse: miscellaneous[1], miscellaneous: miscellaneous[0] },
        SkyLayerKind::Cloud3 => SkyMaterialDesc { kind, diffuse, miscellaneous: miscellaneous[2] },
    }
}

pub(super) fn state(cloud: f32, seconds: f32) -> SkyState {
    let coverage = match cloud.is_finite() {
        true => cloud.clamp(0.0, 1.0),
        false => 0.0,
    };
    let cloud = if cloud.is_finite() && cloud > 0.0 { cloud.clamp(0.0, 1.0) } else { 0.1 };
    let squared = cloud * cloud;
    let scroll = if seconds.is_finite() { seconds * 0.02 } else { 0.0 };
    SkyState {
        cloud_intensity: [
            (cloud * (10.0 / 3.0)).clamp(0.0, 1.0),
            (2.0 * squared).clamp(0.0, 0.8),
            squared.clamp(0.0, 0.8),
            0.0,
        ],
        scroll,
        cloud_coverage: coverage,
        enhanced: false,
        ..Default::default()
    }
}

#[derive(Clone, Copy)]
pub(super) enum View {
    Main,
    Mirror,
    Environment,
    RoadReflection,
}

impl View {
    fn scale(self) -> f32 {
        match self {
            Self::Main => 1.0,
            Self::Mirror => 0.03,
            Self::Environment => 0.0065,
            Self::RoadReflection => 0.06,
        }
    }
}

/// Sky is not ordinary scenery: use the active camera, independent of its city position.
fn remove_fixed(resources: &SectionResources, instances: &mut Vec<Instance>) {
    let legacy: Vec<_> = ["SKYDOME", "SKYDOME_XENON", "SKY_SPECULAR"]
        .iter()
        .filter_map(|name| resources.meshes.get(&bstring_hash(name)).map(|entry| entry.0))
        .collect();
    instances.retain(|instance| !legacy.contains(&instance.mesh));
}

fn transform(camera: Vec3, view: View) -> Mat4 {
    if matches!(view, View::RoadReflection) {
        return Mat4::from_scale_rotation_translation(
            Vec3::new(0.06, 0.06, -0.06),
            glam::Quat::IDENTITY,
            camera + Vec3::Z * 130.0,
        );
    }
    Mat4::from_scale_rotation_translation(Vec3::splat(view.scale()), glam::Quat::IDENTITY, camera)
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_gfx::MeshHandle;

    #[test]
    fn procedural_weather_preserves_clear_zero_and_sanitizes_invalid_inputs() {
        assert_eq!(state(0.0, 0.0).cloud_coverage, 0.0);
        assert_eq!(state(2.0, 0.0).cloud_coverage, 1.0);
        for value in [f32::NAN, f32::INFINITY, -1.0] {
            assert_eq!(state(value, value).cloud_coverage, 0.0);
        }
    }

    #[test]
    fn road_reflection_sky_uses_source_local_scale_and_camera_offset() {
        let camera = Vec3::new(2500.0, 3000.0, 200.0);
        let reflected = transform(camera, View::RoadReflection);
        assert_eq!(reflected.x_axis.x, 0.06);
        assert_eq!(reflected.y_axis.y, 0.06);
        assert_eq!(reflected.z_axis.z, -0.06);
        assert_eq!(reflected.w_axis.truncate(), camera + Vec3::Z * 130.0);
        assert_ne!(reflected, transform(camera, View::Environment));
    }
    use blackbox_scenery::LodModel;

    #[test]
    fn main_sky_follows_translation_without_parallax_and_views_keep_original_scale() {
        let a = Vec3::new(100.0, -500.0, 140.0);
        let b = Vec3::new(-1200.0, 3000.0, 200.0);
        let vertex = Vec3::new(30.0, 40.0, 50.0);
        for (view, scale) in [(View::Main, 1.0), (View::Mirror, 0.03), (View::Environment, 0.0065)] {
            let first = transform(a, view).transform_point3(vertex) - a;
            let second = transform(b, view).transform_point3(vertex) - b;
            assert!((first - second).length() < 0.0003);
            assert!((first - vertex * scale).length() < 0.0001);
        }
    }

    #[test]
    fn fixed_duplicates_are_replaced_once_and_scenery_is_preserved() {
        let mut resources = SectionResources::default();
        let lod = LodModel { num_polys: 1, density: 1.0 };
        for (index, name) in ["SKYDOME", "SKYDOME_XENON", "SKY_SPECULAR"].iter().enumerate() {
            resources.meshes.insert(bstring_hash(name), (MeshHandle::from_raw(index + 1), lod, Default::default()));
        }
        let mut instances = (1..=4).map(|mesh| Instance::new(MeshHandle::from_raw(mesh), Mat4::IDENTITY)).collect();
        Sky::default().place(&resources, &mut instances, Vec3::Z * 140.0, View::Main);
        assert_eq!(instances.len(), 2);
        assert_eq!(instances[0].mesh, MeshHandle::from_raw(4));
        assert_eq!(instances[1].mesh, MeshHandle::from_raw(1));
        assert_eq!(instances[1].transform.w_axis.truncate(), Vec3::Z * 140.0);
        Sky::default().place(&resources, &mut instances, Vec3::X * 5000.0, View::Main);
        assert_eq!(instances.len(), 2);
        assert!(!instances[1].key.is_transient());
    }

    #[test]
    fn native_layers_select_the_original_miscellaneous_bindings() {
        let diffuse = TextureHandle::from_raw(1);
        let misc = [TextureHandle::from_raw(2), TextureHandle::from_raw(3), TextureHandle::from_raw(4)];
        let base = descriptor(SkyLayerKind::Base, diffuse, misc);
        assert_eq!((base.diffuse, base.miscellaneous), (diffuse, misc[0]));
        assert_eq!(descriptor(SkyLayerKind::Cloud2, diffuse, misc).diffuse, misc[1]);
        assert_eq!(descriptor(SkyLayerKind::Cloud3, diffuse, misc).miscellaneous, misc[2]);
    }

    #[test]
    fn sky_cloud_weights_follow_the_runtime_polynomial_and_scroll_is_not_per_view() {
        let clear = state(0.0, 10.0);
        assert!((clear.cloud_intensity[0] - 1.0 / 3.0).abs() < 0.000001);
        assert!((clear.cloud_intensity[1] - 0.02).abs() < 0.000001);
        assert!((clear.cloud_intensity[2] - 0.01).abs() < 0.000001);
        assert_eq!(state(1.0, 50.0).cloud_intensity, [1.0, 0.8, 0.8, 0.0]);
        assert_eq!(state(0.5, 50.0).cloud_intensity, [1.0, 0.5, 0.25, 0.0]);
        assert_eq!(state(0.5, 50.0).scroll, 1.0);
        for _ in 0..6 {
            assert_eq!(state(0.5, 50.0).scroll, 1.0);
        }
        assert_eq!(state(f32::NAN, f32::INFINITY), state(0.0, 0.0));
    }

    #[test]
    #[ignore = "needs original stock sky models and textures selected by NFSMW_GAME_DIR"]
    fn installed_sky_has_complete_stock_bindings_and_independent_secondary_uvs() {
        let dir = game_install::GameDir::open(std::env::var("NFSMW_GAME_DIR").unwrap()).unwrap();
        let globals = nfsmw_data::world::load_global_textures(&dir).unwrap();
        let index = nfsmw_data::world::WorldIndex::open(&dir, nfsmw_data::world::DEFAULT_TRACK).unwrap();
        let section = &index.sections[index.by_name("X0").expect("shared world models")];
        let bytes = dir.read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize).unwrap();
        let solids = blackbox_solid::read_solids(&bytes).unwrap();
        let solid = solids.iter().find(|solid| solid.name == "SKYDOME_XENON").expect("programmable PC sky model");
        assert!(!solid.vertices.is_empty() && !solid.groups.is_empty());
        let secondary_uvs = solid.secondary_uvs.as_ref().expect("the sky model has second UVs");
        assert_eq!(solid.vertices.len(), secondary_uvs.len());
        assert!(solid.vertex_buffers.iter().all(|buffer| buffer.stride == 44));
        assert!(solid.vertices.iter().zip(secondary_uvs).any(|(vertex, &secondary)| vertex.uv != secondary));
        assert!(solid.groups.iter().all(|group| group.effect_id == 19));
        for group in &solid.groups {
            let hash = group.diffuse_texture(solid).expect("authored programmable sky diffuse");
            assert!(globals.textures.iter().any(|texture| texture.name_hash == hash));
        }
        for name in ["SKY_NEXGEN_CAP_A", "SKY_NEXGEN_CLOUDS_B", "SKY_NEXGEN_CAP_B", "SKY_NEXGEN_CLOUDS_A"] {
            assert!(globals.textures.iter().any(|texture| texture.name_hash == bstring_hash(name)), "{name}");
        }
        let mut renderer =
            crate::app::test_backend::NullBackend::new(crate::settings::test_caps::native()).with_sky_tracking();
        let mut resources = SectionResources::default();
        resources.upload_texture_list(&mut renderer, &globals.textures, &[]);
        let mut sky = Sky::upload(&mut renderer, &solids, &resources);
        assert!(sky.layers.is_some(), "installed shared model and global textures must create all native layers");
        assert_eq!(renderer.resources.lock().unwrap().meshes, 3);
        assert_eq!(renderer.resources.lock().unwrap().sky_materials, solid.groups.len() * 3);
        sky.release(&mut renderer);
        sky.release(&mut renderer);
        let counts = renderer.resources.lock().unwrap();
        assert_eq!((counts.meshes, counts.sky_materials), (0, 0));
        assert_eq!(counts.mesh_destructions, 3);
        assert_eq!(counts.sky_destructions, solid.groups.len() * 3);
    }
}
