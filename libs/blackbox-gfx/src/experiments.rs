//! Session-only rendering experiments, deliberately separate from persistent graphics settings.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum LightingDebug {
    #[default]
    Off,
    SunVisibility,
    AmbientVisibility,
    Cascades,
    ShadowBlockers,
    ShadowPenumbra,
}

impl std::fmt::Display for LightingDebug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Off => "off",
            Self::SunVisibility => "sun visibility",
            Self::AmbientVisibility => "AO visibility",
            Self::Cascades => "cascade coverage",
            Self::ShadowBlockers => "shadow blocker count",
            Self::ShadowPenumbra => "shadow filter radius",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderExperiments {
    /// Reconstructed authored world material families instead of legacy prelit fallback.
    pub original_world_materials: bool,
    pub sun_shadows: bool,
    /// None uses custom cascades; Some fits one perspective shadow projection.
    pub shadow_projection: Option<crate::ShadowProjection>,
    pub far_shadows: bool,
    pub contact_shadows: bool,
    pub clearcoat: bool,
    pub car_skylight: bool,
    pub world_ambient: bool,
    /// Minimum ambient share of authored prelit illumination; dark baked art receives a larger share.
    pub world_ambient_share: f32,
    /// Strength of the scene ambient colour in additional shaded regions.
    pub world_ambient_tint: f32,
    pub freeze_sun: bool,
    pub lighting_debug: LightingDebug,
    pub sky_strength: f32,
    pub ground_strength: f32,
    pub soft_shadows: bool,
    /// Integrate wide penumbras with a dense tent footprint; independent of blocker search.
    pub dense_shadow_filter: bool,
    /// Double the nearest cascade's density by reducing its coverage, retaining the other cascades.
    pub near_shadow_focus: bool,
    /// Broader near coverage with independently sized first and second cascade maps.
    pub wide_shadow_detail: bool,
    pub shadow_detail_resolution: u32,
    pub original_shadows: bool,
    /// Directional light angular radius in degrees.
    pub sun_angle: f32,
    pub shadow_resolution: u32,
    pub shadow_distance: f32,
    pub shadow_filter: f32,
    pub contact_length: f32,
}

impl Default for RenderExperiments {
    fn default() -> Self {
        Self {
            original_world_materials: false,
            sun_shadows: false,
            shadow_projection: None,
            far_shadows: false,
            contact_shadows: false,
            clearcoat: false,
            car_skylight: false,
            world_ambient: false,
            world_ambient_share: 0.6,
            world_ambient_tint: 0.35,
            freeze_sun: false,
            lighting_debug: LightingDebug::Off,
            sky_strength: 0.8,
            ground_strength: 0.5,
            soft_shadows: true,
            dense_shadow_filter: false,
            near_shadow_focus: false,
            wide_shadow_detail: false,
            shadow_detail_resolution: 4096,
            original_shadows: true,
            sun_angle: 0.27,
            shadow_resolution: 2048,
            shadow_distance: 160.0,
            shadow_filter: 1.0,
            contact_length: 0.35,
        }
    }
}

impl RenderExperiments {
    pub fn sanitized(self) -> Self {
        Self {
            shadow_resolution: self.shadow_resolution.clamp(512, 8192).next_power_of_two(),
            shadow_detail_resolution: self
                .shadow_detail_resolution
                .clamp(512, 8192)
                .next_power_of_two()
                .max(self.shadow_resolution.clamp(512, 8192).next_power_of_two()),
            sky_strength: finite(self.sky_strength, 0.8).clamp(0.0, 2.0),
            ground_strength: finite(self.ground_strength, 0.5).clamp(0.0, 2.0),
            world_ambient_share: finite(self.world_ambient_share, 0.6).clamp(0.35, 0.9),
            world_ambient_tint: finite(self.world_ambient_tint, 0.35).clamp(0.0, 1.0),
            sun_angle: finite(self.sun_angle, 0.27).clamp(0.0, 2.0),
            shadow_distance: finite(self.shadow_distance, 160.0).clamp(40.0, 320.0),
            shadow_filter: finite(self.shadow_filter, 1.0).clamp(0.0, 3.0),
            contact_length: finite(self.contact_length, 0.35).clamp(0.05, 0.8),
            ..self
        }
    }

    pub fn caster_distance(self) -> f32 {
        if !self.sun_shadows {
            return 0.0;
        }
        if let Some(projection) = self.shadow_projection {
            return projection.frustum[1];
        }
        self.shadow_distance
            * match self.far_shadows {
                true => 3.0,
                false => 1.0,
            }
    }
}

fn finite(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        return value;
    }
    fallback
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ExperimentStats {
    pub supported: bool,
    /// CPU command preparation only, not GPU elapsed time.
    pub shadow_cpu_ms: f32,
    pub shadow_draws: u32,
    pub shadow_instances: u32,
    pub cascades: u32,
    pub map_mib: u32,
    pub gpu_frame_ms: Option<f32>,
    pub gpu_shadow_ms: Option<f32>,
    pub gpu_contact_ms: Option<f32>,
    pub gpu_ao_ms: Option<f32>,
}
