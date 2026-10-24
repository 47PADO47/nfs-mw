//! Parses and validates the WGSL sources with naga, so a broken shader fails `cargo test` without a GPU.

use naga::valid::{Capabilities, ValidationFlags, Validator};

const SHADERS: [(&str, &str); 15] = [
    ("screen displacement", include_str!("shaders/screen_distortion.wgsl")),
    ("scene", include_str!("shaders/scene.wgsl")),
    ("resolve", include_str!("shaders/resolve.wgsl")),
    ("contact", include_str!("shaders/contact.wgsl")),
    ("contact prepass", include_str!("shaders/contact_prepass.wgsl")),
    ("irradiance", include_str!("shaders/irradiance.wgsl")),
    ("shadow depth", include_str!("shaders/shadow_depth.wgsl")),
    ("sky", include_str!("shaders/sky.wgsl")),
    ("enhanced sky", include_str!("shaders/sky_enhanced.wgsl")),
    ("sky filter", include_str!("shaders/sky_filter.wgsl")),
    ("sky noise", include_str!("shaders/sky_noise.wgsl")),
    ("normal", include_str!("shaders/normal.wgsl")),
    ("window", include_str!("shaders/window.wgsl")),
    ("motion", include_str!("shaders/motion.wgsl")),
    ("reflection depth", include_str!("shaders/reflection_depth.wgsl")),
];

fn entry_points(name: &str) -> Vec<String> {
    let source = SHADERS.iter().find(|(n, _)| *n == name).expect("known shader").1;
    let source = match name {
        "enhanced sky" => format!("{}\n{}", source, include_str!("shaders/sky_clouds.wgsl")),
        "scene" => format!(
            "{}\n{}\n{}",
            source,
            include_str!("shaders/world_surface.wgsl"),
            include_str!("shaders/shadow_receiver.wgsl").replace("SHADOW_GROUP", "2")
        ),
        "normal" => format!(
            "{}\n{}\n{}",
            source,
            include_str!("shaders/shadow_receiver.wgsl").replace("SHADOW_GROUP", "2"),
            include_str!("shaders/road_reflection.wgsl")
        ),
        "window" => {
            format!("{}\n{}", source, include_str!("shaders/shadow_receiver.wgsl").replace("SHADOW_GROUP", "2"))
        }
        _ => source.to_string(),
    };
    let source = source.as_str();
    let module =
        naga::front::wgsl::parse_str(source).unwrap_or_else(|e| panic!("{name}: {}", e.emit_to_string(source)));
    Validator::new(ValidationFlags::all(), Capabilities::default())
        .validate(&module)
        .unwrap_or_else(|e| panic!("{name}: {e:?}"));
    module.entry_points.iter().map(|e| e.name.clone()).collect()
}

#[test]
fn every_shader_parses_and_validates() {
    for (name, source) in crate::gpu::ao::sources() {
        let module =
            naga::front::wgsl::parse_str(&source).unwrap_or_else(|e| panic!("{name}: {}", e.emit_to_string(&source)));
        Validator::new(ValidationFlags::all(), Capabilities::default())
            .validate(&module)
            .unwrap_or_else(|e| panic!("{name}: {e:?}"));
    }
    for (name, _) in SHADERS {
        assert!(!entry_points(name).is_empty(), "{name} has entry points");
    }
}

#[test]
fn the_effects_shader_with_the_shadow_receiver_validates_and_keeps_its_entry_points() {
    use blackbox_gpu_passes::{EFFECTS_BASE_WGSL, EFFECTS_SHADOWED_WGSL};
    let receiver = include_str!("shaders/shadow_receiver.wgsl").replace("SHADOW_GROUP", "1");
    let source = format!("{receiver}\n{EFFECTS_BASE_WGSL}{EFFECTS_SHADOWED_WGSL}");
    let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|e| panic!("{}", e.emit_to_string(&source)));
    Validator::new(ValidationFlags::all(), Capabilities::default()).validate(&module).unwrap();
    for entry in ["vs_main", "fs_streak", "fs_streak_srgb", "fs_glow", "fs_glow_srgb", "fs_textured"] {
        assert!(module.entry_points.iter().any(|e| e.name == entry), "{entry} survives");
    }
}
