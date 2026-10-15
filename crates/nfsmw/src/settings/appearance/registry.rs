//! Graphics coverage ledger shared by F8 diagnostics and profile documentation.
#[derive(Clone, Copy)]
pub struct Feature {
    pub name: &'static str,
    pub origin: &'static str,
    pub coverage: &'static str,
    pub gap: &'static str,
}
pub const FEATURES: &[Feature] = &[
    Feature {
        name: "World materials",
        origin: "Original assets / simplified renderer",
        coverage: "Partial",
        gap: "Original effect families, normal/detail maps, blending",
    },
    Feature {
        name: "Car materials and lighting",
        origin: "Original parameters / custom light rig",
        coverage: "Partial",
        gap: "Original light rigs and remaining paint/glass behavior",
    },
    Feature {
        name: "Authored shadow overlays",
        origin: "Original scenery assets",
        coverage: "Available",
        gap: "Not the original dynamic shadow renderer",
    },
    Feature {
        name: "Dynamic sun / contact shadows",
        origin: "Custom cascades, PCSS and screen-space detail",
        coverage: "Enhancement",
        gap: "Original car/world shadow path remains incomplete",
    },
    Feature {
        name: "Visual Treatment",
        origin: "Original midday profile / custom strength blend",
        coverage: "Partial",
        gap: "Tunnel exposure, pursuit/cinematic profiles and transitions",
    },
    Feature {
        name: "Bloom / tone mapping",
        origin: "Custom bloom and ACES",
        coverage: "Enhancement",
        gap: "Original Over Bright is not implemented",
    },
    Feature {
        name: "Car reflections",
        origin: "Original-style cubemap / custom capture budgets",
        coverage: "Partial",
        gap: "Exact retail capture policy; wet-road and water reflection paths",
    },
    Feature {
        name: "Ambient lighting / clearcoat / AO",
        origin: "Custom environment probe, coat and GTAO",
        coverage: "Enhancement",
        gap: "Not evidence of original lighting parity",
    },
    Feature {
        name: "Sky and fog",
        origin: "Original assets/formulas with approximations",
        coverage: "Partial",
        gap: "Sky effect composition and dynamic location blend",
    },
    Feature {
        name: "Weather / time",
        origin: "Partial original weather / custom clock and drops",
        coverage: "Partial",
        gap: "Regional rain, wind/splash integration; custom night rendering",
    },
    Feature {
        name: "Player lights",
        origin: "Original profiles and lamp textures",
        coverage: "Partial",
        gap: "Occlusion, damage flags and auxiliary reflection integration",
    },
    Feature {
        name: "Speedbreaker",
        origin: "Original profile and mask",
        coverage: "Partial",
        gap: "Full composited image/transition validation",
    },
    Feature {
        name: "Particles",
        origin: "Original-derived collisions/exhaust; custom tire visuals",
        coverage: "Mixed",
        gap: "Complete authored smoke/skid/particle parity",
    },
    Feature {
        name: "AA / upscaling",
        origin: "Modern FXAA, SMAA and spatial FSR1",
        coverage: "Enhancement",
        gap: "Original PC AA and backend-dependent temporal methods",
    },
];
pub const REFERENCE_POLICY: &str = "Modded PC effects may guide the reference when reproducible with stock assets. Mod-specific textures, shaders and packs are not required.";
