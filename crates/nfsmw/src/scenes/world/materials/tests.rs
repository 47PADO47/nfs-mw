use super::*;

/// No texture is known: every group keeps both faces.
fn no_sides(_: u32) -> Option<Sidedness> {
    None
}

#[test]
fn groups_cull_by_their_texture_and_the_car_effect_culls_blended_ones_too() {
    let sides = |hash| match hash {
        1 => Some(Sidedness { two_sided: false, blended: false }),
        2 => Some(Sidedness { two_sided: false, blended: true }),
        _ => Some(Sidedness { two_sided: true, blended: false }),
    };
    let lookup = Lookup(|_| None, false, Vec::new(), sides);
    assert!(!lookup.two_sided(1, 0), "an opaque world texture is single-sided");
    assert!(lookup.two_sided(2, 0), "blended world surfaces keep both faces");
    assert!(!lookup.two_sided(2, 4), "the car effect culls blended surfaces");
    assert!(lookup.two_sided(3, 0) && lookup.two_sided(4, 4), "flagged and cut-out textures keep both faces");
    assert!(Lookup(|_| None, false, Vec::new(), no_sides).two_sided(1, 0), "unknown textures keep both faces");
    assert!(Lookup(|_| None, true, Vec::new(), sides).two_sided(1, 0), "sky domes keep both faces");
}

#[test]
fn original_effects_have_distinct_routing_and_unsupported_families_stay_explicit() {
    let lookup = Lookup(|_| None, false, Vec::new(), no_sides);
    assert_eq!(lookup.effect_shading(0), Some(Shading::World));
    assert_eq!(lookup.effect_shading(6), Some(Shading::Tree));
    assert_eq!(lookup.effect_shading(8), Some(Shading::WorldNoFog));
    assert_eq!(lookup.effect_shading(17), Some(Shading::WorldPrelit));
    for effect in [1, 2, 3, 4, 5, 7, 19, u32::MAX] {
        assert_eq!(lookup.effect_shading(effect), None);
    }
    let sky = Lookup(|_| None, true, Vec::new(), no_sides);
    for effect in [0, 6, 8, 17, 19] {
        assert_eq!(sky.effect_shading(effect), None);
    }
}

fn solid() -> Solid {
    let group = blackbox_solid::ShadingGroup {
        bounds_min: [0.0; 3],
        bounds_max: [0.0; 3],
        texture_slots: [0, 1, 0xFF, 2, 0xFF],
        light_material: 0xFF,
        effect_id: 3,
        flags: 0,
        num_vertices: 3,
        num_triangles: 1,
        first_index: 0,
        num_indices: 3,
        base_vertex: 0,
        vertex_buffer: 0,
        bone_table: None,
    };
    Solid {
        name: "NORMAL".into(),
        name_hash: 1,
        version: 0x16,
        flags: 0,
        bounds_min: [0.0; 3],
        bounds_max: [0.0; 3],
        transform: [0.0; 16],
        num_polys: 1,
        density: 0.0,
        texture_hashes: vec![10, 20, 30],
        light_material_hashes: vec![],
        markers: vec![],
        vertex_buffers: vec![blackbox_solid::VertexBuffer { stride: 60, first_vertex: 0, vertex_count: 3 }],
        vertices: vec![
            blackbox_solid::Vertex {
                position: [0.0; 3],
                normal: [0.0, 0.0, 1.0],
                color_bgra: [255; 4],
                uv: [0.0; 2]
            };
            3
        ],
        secondary_uvs: vec![[0.0; 2]; 3],
        tangents: vec![[1.0, 0.0, 0.0, -1.0]; 3],
        indices: vec![0, 1, 2],
        groups: vec![group.clone(), group],
        skin: None,
    }
}

#[test]
fn normal_groups_require_complete_inputs_cache_triplets_and_release_idempotently() {
    use crate::app::test_backend::NullBackend;
    let mut backend = NullBackend::new(crate::settings::test_caps::native()).with_normal_tracking();
    let mut resources = super::super::resident::SectionResources::default();
    let lookup = |hash| Some((TextureHandle::from_raw(hash as usize), BlendMode::AlphaTest));
    let mut model = solid();
    let handles = normal_materials(&mut backend, &model, &lookup, &mut resources.normal_materials);
    assert_eq!(handles[0], handles[1]);
    assert_eq!(resources.normal_materials.len(), 1);
    let materials = Lookup(lookup, false, handles, no_sides);
    let draws = blackbox_scene::solid_mesh(&model, &materials, Shading::Prelit);
    assert!(draws.iter().all(|d| matches!(d.shading, Shading::WorldNormal(_)) && d.blend == BlendMode::AlphaTest));
    model.groups[1].texture_slots[3] = 0xFF;
    let handles = normal_materials(&mut backend, &model, &lookup, &mut resources.normal_materials);
    assert!(handles[0].is_some());
    assert!(handles[1].is_none());
    let draws = blackbox_scene::solid_mesh(&model, &Lookup(lookup, false, handles, no_sides), Shading::Prelit);
    assert_eq!(draws[1].shading, Shading::Prelit);
    resources.release_normal_materials(&mut backend);
    resources.release_normal_materials(&mut backend);
    let counts = backend.resources.lock().unwrap();
    assert_eq!((counts.normal_materials, counts.normal_destructions), (0, 1));
}

#[test]
fn unsupported_backend_leaves_normal_groups_on_legacy_fallback_without_allocating() {
    let mut backend = crate::app::test_backend::NullBackend::new(crate::settings::test_caps::native());
    let mut cache = HashMap::new();
    let handles = normal_materials(
        &mut backend,
        &solid(),
        &|_| Some((TextureHandle::from_raw(0), BlendMode::Opaque)),
        &mut cache,
    );
    assert_eq!(handles, [None, None]);
    assert!(cache.is_empty());
}

#[test]
fn reflection_and_bump_cache_independently_even_with_identical_texture_triplets() {
    let mut backend =
        crate::app::test_backend::NullBackend::new(crate::settings::test_caps::native()).with_normal_tracking();
    let mut cache = HashMap::new();
    let mut model = solid();
    model.groups[1].effect_id = 1;
    let materials = normal_materials(
        &mut backend,
        &model,
        &|hash| Some((TextureHandle::from_raw(hash as usize), BlendMode::Opaque)),
        &mut cache,
    );
    let [Some(Shading::WorldNormal(bump)), Some(Shading::WorldNormal(reflective))] = materials[..] else {
        panic!("both complete material families must resolve");
    };
    assert_ne!(bump, reflective);
    assert_eq!(backend.normal_descriptors[&bump].model, NormalModel::Bump);
    assert_eq!(backend.normal_descriptors[&reflective].model, NormalModel::Reflective);
    assert_eq!(cache.len(), 2);
}

#[test]
fn windows_require_global_reflection_instead_of_aliasing_a_group_texture_slot() {
    let mut backend =
        crate::app::test_backend::NullBackend::new(crate::settings::test_caps::native()).with_normal_tracking();
    let mut cache = HashMap::new();
    let mut model = solid();
    model.groups[0].effect_id = 5;
    let diffuse_only = |hash| (hash == 10).then_some((TextureHandle::from_raw(10), BlendMode::AlphaBlend));
    assert_eq!(window_materials(&mut backend, &model, &diffuse_only, &mut cache), [None, None]);
    assert!(cache.is_empty());
    let lookup = |hash| {
        if hash == blackbox_hash::bstring_hash("WINDOWREFLECTION") {
            return Some((TextureHandle::from_raw(99), BlendMode::Opaque));
        }
        diffuse_only(hash)
    };
    let result = window_materials(&mut backend, &model, &lookup, &mut cache);
    let Some(Shading::WorldWindow(handle)) = result[0] else {
        panic!("complete global reflection binding");
    };
    assert_eq!(backend.window_descriptors[&handle].reflection, TextureHandle::from_raw(99));
    assert_eq!(result[1], None);
    assert_eq!(cache.len(), 1);
    let mut unsupported = crate::app::test_backend::NullBackend::new(crate::settings::test_caps::native());
    assert_eq!(window_materials(&mut unsupported, &model, &lookup, &mut HashMap::new()), [None, None]);
}
