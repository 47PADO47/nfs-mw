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
    /// Procedural atmosphere and sun-lit cloud volume in the native sky pass.
    pub enhanced_sky: bool,
    /// Cloud sunlight attenuation, independent of geometry shadows.
    pub cloud_shadows: bool,
    /// Spatial wet-film and pooled-water response on eligible reflective surfaces.
    pub enhanced_wet_surfaces: bool,
    pub puddle_coverage: f32,
    /// World-space patch size in metres.
    pub puddle_scale: f32,
    pub water_ripples: f32,
    /// Use hardware geometry queries when the renderer was started with RT support.
    pub ray_road_reflections: bool,
    /// Hardware reflections on custom glossy materials; rough lobes keep their filtered probe.
    pub ray_car_reflections: bool,
    /// Reflection-only texture cache budget: Low, Balanced, High, Very High.
    pub ray_texture_quality: u32,
    pub ray_car_range: f32,
    pub ray_car_blend: f32,
    /// RT hit contribution relative to the environment fallback.
    pub ray_road_blend: f32,
    /// Reuse fully shaded screen colour at an exact visible RT hit.
    pub ray_road_screen_reuse: f32,
    /// Maximum reflection ray length in metres.
    pub road_reflection_range: f32,
    /// Fraction of the ray range used to blend into the environment fallback.
    pub road_reflection_fade: f32,
    pub sky_wind_speed: f32,
    pub sky_sun_size: f32,
    pub sky_sun_brightness: f32,
    pub sky_brightness: f32,
    pub sky_sunset_warmth: f32,
    /// Reconstructed authored world material families instead of legacy prelit fallback.
    pub original_world_materials: bool,
    pub sun_shadows: bool,
    /// Deterministic near-field Vulkan sun visibility, with cascades at distance.
    pub ray_sun_shadows: bool,
    /// Finite sun-disc visibility; hard rays remain an independent comparison.
    pub ray_soft_shadows: bool,
    pub ray_shadow_samples: u32,
    /// Linear mask resolution percentage. 50 traces one quarter as many pixels.
    pub ray_shadow_resolution: u32,
    /// Blend two adjacent spatial estimates without recursive accumulation.
    pub ray_shadow_temporal: bool,
    pub ray_shadow_filter: f32,
    pub ray_shadow_range: f32,
    pub ray_shadow_length: f32,
    pub ray_shadow_bias: f32,
    /// Reuse exact static depth while redrawing moving casters every frame.
    pub cached_shadows: bool,
    /// Reuse local probes within a bounded time and movement budget.
    pub cached_reflections: bool,
    /// Trace current scene colour only on glossy materials, with probe fallback.
    pub screen_reflections: bool,
    /// None uses custom cascades; Some fits one perspective shadow projection.
    pub shadow_projection: Option<crate::ShadowProjection>,
    /// Short motion-reprojected stabilization of cascaded visibility.
    pub cascade_temporal: bool,
    pub far_shadows: bool,
    /// Outer cascade coverage relative to the detailed range.
    pub far_shadow_scale: f32,
    pub contact_shadows: bool,
    pub clearcoat: bool,
    /// Opt-in surfaces use calibrated highlights and filtered environment reflections.
    pub paint_response: bool,
    /// Reproject current local capture depth for nearby custom surfaces.
    pub local_reflection_correction: bool,
    /// The game may match custom car lights to authored world lighting.
    pub weather_car_lighting: bool,
    pub car_skylight: bool,
    pub world_ambient: bool,
    /// Minimum ambient share of authored prelit illumination; dark baked art receives a larger share.
    pub world_ambient_share: f32,
    /// Strength of the scene ambient colour in additional shaded regions.
    pub world_ambient_tint: f32,
    pub freeze_sun: bool,
    pub lighting_debug: LightingDebug,
    /// Overall indirect lighting multiplier, independent of hemisphere calibration.
    pub indirect_strength: f32,
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
    /// Skip static scenery smaller than this radius in separate shadow feeds.
    /// Zero preserves every caster. Vehicles and dynamic props are unaffected.
    pub shadow_caster_min_radius: f32,
    pub shadow_filter: f32,
    pub contact_length: f32,
}

impl Default for RenderExperiments {
    fn default() -> Self {
        Self {
            enhanced_sky: true,
            cloud_shadows: false,
            enhanced_wet_surfaces: false,
            puddle_coverage: 0.7,
            puddle_scale: 7.0,
            water_ripples: 0.5,
            road_reflection_range: 320.0,
            road_reflection_fade: 0.65,
            ray_road_reflections: true,
            ray_sun_shadows: false,
            ray_soft_shadows: false,
            ray_shadow_samples: 4,
            ray_shadow_resolution: 50,
            ray_shadow_temporal: false,
            ray_shadow_filter: 0.75,
            ray_shadow_range: 100.0,
            ray_shadow_length: 300.0,
            ray_shadow_bias: 0.006,
            ray_car_reflections: false,
            ray_texture_quality: 1,
            ray_car_range: 320.0,
            ray_car_blend: 1.0,
            ray_road_blend: 1.0,
            ray_road_screen_reuse: 1.0,
            sky_wind_speed: 2.0,
            sky_sun_size: 2.0,
            sky_sun_brightness: 1.0,
            sky_brightness: 1.0,
            sky_sunset_warmth: 1.0,
            original_world_materials: false,
            sun_shadows: false,
            cached_shadows: false,
            cached_reflections: false,
            screen_reflections: false,
            shadow_projection: None,
            cascade_temporal: false,
            far_shadows: false,
            far_shadow_scale: 3.0,
            contact_shadows: false,
            clearcoat: false,
            paint_response: true,
            local_reflection_correction: true,
            weather_car_lighting: true,
            car_skylight: false,
            world_ambient: false,
            world_ambient_share: 0.6,
            world_ambient_tint: 0.35,
            freeze_sun: false,
            lighting_debug: LightingDebug::Off,
            indirect_strength: 1.0,
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
            shadow_caster_min_radius: 0.0,
            shadow_filter: 1.0,
            contact_length: 0.35,
        }
    }
}

impl RenderExperiments {
    pub fn sanitized(self) -> Self {
        Self {
            puddle_coverage: finite(self.puddle_coverage, 0.7).clamp(0.0, 1.0),
            puddle_scale: finite(self.puddle_scale, 7.0).clamp(2.0, 30.0),
            road_reflection_range: finite(self.road_reflection_range, 320.0).clamp(80.0, 800.0),
            road_reflection_fade: finite(self.road_reflection_fade, 0.65).clamp(0.1, 0.9),
            ray_shadow_samples: match self.ray_shadow_samples {
                0..=4 => 4,
                5..=8 => 8,
                _ => 16,
            },
            ray_shadow_resolution: match self.ray_shadow_resolution {
                0..=50 => 50,
                _ => 100,
            },
            far_shadow_scale: finite(self.far_shadow_scale, 3.0).clamp(1.0, 5.0),
            ray_shadow_filter: finite(self.ray_shadow_filter, 0.75).clamp(0.0, 2.0),
            ray_shadow_range: finite(self.ray_shadow_range, 100.0).clamp(10.0, 240.0),
            ray_shadow_length: finite(self.ray_shadow_length, 300.0).clamp(40.0, 800.0),
            ray_shadow_bias: finite(self.ray_shadow_bias, 0.006).clamp(0.001, 0.05),
            ray_texture_quality: self.ray_texture_quality.min(3),
            ray_car_range: finite(self.ray_car_range, 320.0).clamp(40.0, 800.0),
            ray_car_blend: finite(self.ray_car_blend, 1.0).clamp(0.0, 1.0),
            ray_road_blend: finite(self.ray_road_blend, 1.0).clamp(0.0, 1.0),
            ray_road_screen_reuse: finite(self.ray_road_screen_reuse, 1.0).clamp(0.0, 1.0),
            water_ripples: finite(self.water_ripples, 0.5).clamp(0.0, 2.0),
            sky_wind_speed: finite(self.sky_wind_speed, 2.0).clamp(0.0, 12.0),
            sky_sun_size: finite(self.sky_sun_size, 2.0).clamp(0.5, 3.0),
            sky_sun_brightness: finite(self.sky_sun_brightness, 1.0).clamp(0.0, 3.0),
            sky_brightness: finite(self.sky_brightness, 1.0).clamp(0.25, 2.0),
            sky_sunset_warmth: finite(self.sky_sunset_warmth, 1.0).clamp(0.0, 2.0),
            shadow_resolution: self.shadow_resolution.clamp(512, 8192).next_power_of_two(),
            shadow_detail_resolution: self
                .shadow_detail_resolution
                .clamp(512, 8192)
                .next_power_of_two()
                .max(self.shadow_resolution.clamp(512, 8192).next_power_of_two()),
            indirect_strength: finite(self.indirect_strength, 1.0).clamp(0.0, 2.0),
            sky_strength: finite(self.sky_strength, 0.8).clamp(0.0, 2.0),
            ground_strength: finite(self.ground_strength, 0.5).clamp(0.0, 2.0),
            world_ambient_share: finite(self.world_ambient_share, 0.6).clamp(0.35, 0.9),
            world_ambient_tint: finite(self.world_ambient_tint, 0.35).clamp(0.0, 1.0),
            sun_angle: finite(self.sun_angle, 0.27).clamp(0.0, 2.0),
            shadow_distance: finite(self.shadow_distance, 160.0).clamp(40.0, 320.0),
            shadow_caster_min_radius: finite(self.shadow_caster_min_radius, 0.0).clamp(0.0, 5.0),
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
                true => finite(self.far_shadow_scale, 3.0).clamp(1.0, 5.0),
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
    pub shadow_cached_cascades: u32,
    pub shadow_instances: u32,
    pub cascades: u32,
    pub map_mib: u32,
    pub gpu_frame_ms: Option<f32>,
    pub gpu_shadow_ms: Option<f32>,
    pub gpu_contact_ms: Option<f32>,
    pub gpu_ao_ms: Option<f32>,
}

#[cfg(test)]
mod ray_shadow_tests {
    use super::RenderExperiments;
    #[test]
    fn invalid_sun_ray_controls_are_bounded_and_old_profiles_stay_opt_in() {
        let defaults = RenderExperiments::default();
        assert!(!defaults.ray_sun_shadows);
        let invalid = RenderExperiments {
            ray_shadow_samples: 100,
            ray_shadow_resolution: 999,
            far_shadow_scale: f32::INFINITY,
            ray_shadow_filter: f32::NAN,
            ray_shadow_range: f32::NAN,
            ray_shadow_length: f32::INFINITY,
            ray_shadow_bias: -1.0,
            ..defaults
        }
        .sanitized();
        assert_eq!(invalid.ray_shadow_samples, 16);
        assert_eq!(invalid.ray_shadow_resolution, 100);
        assert_eq!(invalid.far_shadow_scale, 3.0);
        let range = RenderExperiments {
            sun_shadows: true,
            far_shadows: true,
            shadow_distance: 160.0,
            far_shadow_scale: 2.5,
            ..defaults
        };
        assert_eq!(range.caster_distance(), 400.0);
        assert_eq!(invalid.ray_shadow_filter, 0.75);
        assert_eq!(invalid.ray_shadow_range, 100.0);
        assert_eq!(invalid.ray_shadow_length, 300.0);
        assert_eq!(invalid.ray_shadow_bias, 0.001);
        let large =
            RenderExperiments { ray_shadow_range: 900.0, ray_shadow_length: 900.0, ray_shadow_bias: 1.0, ..defaults }
                .sanitized();
        assert_eq!((large.ray_shadow_range, large.ray_shadow_length, large.ray_shadow_bias), (240.0, 800.0, 0.05));
    }
}
