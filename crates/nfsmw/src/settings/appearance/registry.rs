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
        name: "Sky and clouds",
        origin: "Authored PC layers or independent Enhanced atmosphere and cloud volume on native",
        coverage: "Partial / enhancement",
        gap: "Full night lighting, cloud shadows on scenery and Bevy Enhanced sky",
    },
    Feature {
        name: "World / car geometry detail",
        origin: "Adaptive scenery slots with fixed reference projection; full texture mips; car LOD A",
        coverage: "Partial / custom budgets",
        gap: "Original world mesh filters/mip levels, PC density distance gate, calibrated projection and dynamic car LOD; current High is not original maximum",
    },
    Feature {
        name: "World materials",
        origin: "Original common, normal, reflective and window families on native; legacy fallback selectable",
        coverage: "Partial",
        gap: "Full projected caster/filter parity, sampler states, remaining capture effects and matched scenes",
    },
    Feature {
        name: "World normal maps",
        origin: "Original tangent-space normal/specular shading on native",
        coverage: "Partial",
        gap: "Bevy fallback; complete projected-shadow filtering and remaining sampler states",
    },
    Feature {
        name: "Car materials and lighting",
        origin: "Original PC material families and gameplay/default frontend rigs on native",
        coverage: "Partial",
        gap: "Bevy custom shader fallback; skin composition, frontend context and full projected-shadow classification",
    },
    Feature {
        name: "Authored shadow overlays",
        origin: "Original scenery assets",
        coverage: "Available",
        gap: "Not the original dynamic shadow renderer",
    },
    Feature {
        name: "Dynamic sun / contact shadows",
        origin: "Recovered single-map perspective fit and sun/cloud strength; separate custom cascades/PCSS/contact",
        coverage: "Partial / enhancement",
        gap: "Original caster selection, draw strength/classification, vendor filtering and bias",
    },
    Feature {
        name: "Visual Treatment",
        origin: "Original midday profile; custom strength and supplied Reference (mod) parameters",
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
        gap: "Exact retail capture policy and separate water reflection paths",
    },
    Feature {
        name: "Wet-road reflections",
        origin: "Original authored projected scenery/sky, attached vehicles and blur; elapsed-time wetness adaptation",
        coverage: "Partial / resolution enhancement",
        gap: "Bevy unsupported; complete original eligibility/LOD/effects, sampler state and matched live routes",
    },
    Feature {
        name: "Ambient lighting / clearcoat / AO",
        origin: "Enhanced car environment light/material coat, legacy prelit world ambient and GTAO",
        coverage: "Enhancement",
        gap: "Car terms bypass Original/Simple; authored world families bypass legacy ambient; not original parity",
    },
    Feature {
        name: "Sky and fog",
        origin: "Original PC sky layers and location/cloud-driven atmosphere",
        coverage: "Partial",
        gap: "Full sky on native only; reflection textures, scripted weather and scene matching",
    },
    Feature {
        name: "Weather / time",
        origin: "Original sun cycle/cloud map; custom clock (including dusk-lit night extension) and drops",
        coverage: "Partial",
        gap: "Regional storms, tunnel exposure and wind/splashes; reference lighting data may differ",
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
