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
    /// Scale dynamic object environment lighting by local shelter.
    pub per_car_environment: bool,
    /// Spatial wet-film and pooled-water response on eligible reflective surfaces.
    pub enhanced_wet_surfaces: bool,
    pub puddle_ripples: bool,
    pub puddle_coverage: f32,
    /// World-space patch size in metres.
    pub puddle_scale: f32,
    pub water_ripples: f32,
    /// How much a continuous water film smooths and polishes wet surfaces outside the pools: 0 a damp film, 1 close
    /// to standing water.
    pub wet_sheen: f32,
    /// Brightness of lights in wet surfaces: lamp glints and bright reflected sources (1 = unboosted).
    pub wet_light_strength: f32,
    /// At night, the brightness of a clear coat's sharp reflection relative to the night world it mirrors (1 = as
    /// bright); chrome, bare metal and rough lobes take its square root.
    pub car_night_reflections: f32,
    /// Brightness of lights in car paint at night: lamp glints and bright reflected sources (1 = unboosted).
    pub car_light_strength: f32,
    /// RT car reflections light what they hit as the world is lit: the night tint and the local lamps.
    pub car_rt_lamp_lighting: bool,
    /// Radius in metres of a street lamp's emitter mirrored in car paint (headlights half of it); 0 a point.
    pub car_lamp_size: f32,
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
    /// Reflect the directional sky and horizon on world glazing.
    pub enhanced_windows: bool,
    pub sun_shadows: bool,
    pub local_lamps: bool,
    /// Recognized streetlight fixtures, illuminated at night.
    pub local_street_lights: bool,
    pub local_headlights: bool,
    pub local_traffic_headlights: bool,
    /// Maximum traffic cars contributing physical headlights.
    pub local_traffic_limit: u32,
    pub experimental_night: bool,
    /// Optional continuous, clock-driven day and night.
    pub full_day_night: bool,
    /// Automatically switch the player lights at dusk and dawn.
    pub auto_player_lights: bool,
    /// Player tail and brake lights illuminate nearby surfaces.
    pub local_tail_lights: bool,
    /// Allow moving rear and emergency spill lights to use local shadow maps.
    pub moving_spill_shadows: bool,
    /// Flashing emergency light bars of nearby vehicles illuminate the surfaces around them.
    pub local_emergency_lights: bool,
    /// Brightness of the emergency light-bar spill.
    pub local_emergency_strength: f32,
    /// Enhanced sky brightness during the experimental night mode.
    pub night_sky_brightness: f32,
    /// At night the sun's direct light becomes moonlight: the share of it kept on a clear night (shadows and
    /// highlights); the rest turns into soft ambient light.
    pub moonlight: f32,
    /// How much a full overcast takes away of that moonlight (1 = all of it).
    pub moon_cloud_block: f32,
    pub local_light_distance: f32,
    pub local_light_strength: f32,
    /// Streetlight brightness and physical reach, independent of tunnel fixtures.
    pub local_street_strength: f32,
    pub local_street_range: f32,
    /// Apparent bulb glow, independent of the light cast on the world.
    pub local_street_glow: f32,
    /// Shared weather-dependent light scattering from local fixtures.
    pub local_light_fog: f32,
    /// Scatter directional light in the shared fog volume.
    pub sun_shafts: bool,
    /// Directional scattering multiplier.
    pub sun_shaft_strength: f32,
    /// View distance of the shared local-light fog in metres.
    pub local_light_fog_distance: f32,
    /// Shared fog-grid resolution, from 1 (low) to 3 (high).
    pub local_light_fog_quality: u32,
    /// Street haze volumes budget. Kept for saved profiles; the fog budget is `local_light_fog_count`.
    pub local_street_haze_count: u32,
    /// Maximum number of local lights contributing to the shared light fog (0 to 64).
    pub local_light_fog_count: u32,
    pub local_headlight_strength: f32,
    pub local_light_limit: u32,
    pub local_shadow_count: u32,
    pub local_shadow_resolution: u32,
    pub local_shadow_filter: f32,
    pub local_shadow_distance: f32,
    pub local_specular_strength: f32,
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
    /// With contact shadows: long screen-space sun shadows (60 depth samples towards the sun) in place of the
    /// short contact march.
    pub screen_space_shadows: bool,
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
    /// Replace a portion of authored ambient with directional sky/ground illumination.
    pub world_skylight: bool,
    pub world_skylight_strength: f32,
    /// Break up the repetition of tiled terrain, cliff and concrete textures the game flags
    /// (`RenderBackend::set_texture_variation`). Off keeps every texture as authored.
    pub texture_variation: bool,
    /// Parallax occlusion mapping on normal-mapped scenery whose normal map carries a height in alpha (a texture mod's
    /// DXT5 normal map). Maps without alpha read as flat and keep their look.
    pub parallax: bool,
    /// Relief depth of a full alpha swing, in centimetres.
    pub parallax_depth: f32,
    /// Most height samples per pixel; grazing views take the most.
    pub parallax_steps: u32,
    /// Distance over which parallax fades out, in metres.
    pub parallax_distance: f32,
    /// Use roughness-based direct highlights for normal-mapped scenery.
    pub world_specular: bool,
    /// Widen unresolved world highlights before raster output.
    pub world_specular_aa: bool,
    /// Steer the directional world ambient light with XeGTAO.
    pub bent_normal_ao: bool,
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
            per_car_environment: false,
            enhanced_wet_surfaces: false,
            puddle_ripples: false,
            puddle_coverage: 0.7,
            puddle_scale: 7.0,
            water_ripples: 0.5,
            wet_sheen: 0.6,
            wet_light_strength: 2.0,
            car_night_reflections: 2.0,
            car_light_strength: 2.0,
            car_rt_lamp_lighting: true,
            car_lamp_size: 0.25,
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
            enhanced_windows: false,
            sun_shadows: false,
            local_lamps: false,
            local_street_lights: false,
            local_headlights: false,
            local_traffic_headlights: false,
            local_traffic_limit: 4,
            experimental_night: false,
            full_day_night: false,
            auto_player_lights: true,
            local_tail_lights: false,
            moving_spill_shadows: false,
            local_emergency_lights: false,
            local_emergency_strength: 1.0,
            night_sky_brightness: 0.18,
            moonlight: 0.2,
            moon_cloud_block: 0.9,
            local_light_distance: 160.0,
            local_light_strength: 1.0,
            local_street_strength: 1.5,
            local_street_range: 35.0,
            local_street_glow: 1.0,
            local_light_fog: 0.5,
            sun_shafts: false,
            sun_shaft_strength: 1.0,
            local_light_fog_distance: 200.0,
            local_light_fog_quality: 2,
            local_street_haze_count: 8,
            local_light_fog_count: 8,
            local_headlight_strength: 1.15,
            local_light_limit: 128,
            local_shadow_count: 8,
            local_shadow_resolution: 1024,
            local_shadow_filter: 1.0,
            local_shadow_distance: 80.0,
            local_specular_strength: 0.15,
            cached_shadows: false,
            cached_reflections: false,
            screen_reflections: false,
            shadow_projection: None,
            cascade_temporal: false,
            far_shadows: false,
            far_shadow_scale: 3.0,
            contact_shadows: false,
            screen_space_shadows: false,
            clearcoat: false,
            paint_response: true,
            local_reflection_correction: true,
            weather_car_lighting: true,
            car_skylight: false,
            world_ambient: false,
            world_ambient_share: 0.6,
            world_ambient_tint: 0.35,
            world_skylight: false,
            world_skylight_strength: 0.65,
            texture_variation: false,
            parallax: false,
            parallax_depth: 4.0,
            parallax_steps: 24,
            parallax_distance: 40.0,
            world_specular: false,
            world_specular_aa: false,
            bent_normal_ao: false,
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
            night_sky_brightness: finite(self.night_sky_brightness, 0.18).clamp(0.05, 0.4),
            moonlight: finite(self.moonlight, 0.2).clamp(0.0, 1.0),
            moon_cloud_block: finite(self.moon_cloud_block, 0.9).clamp(0.0, 1.0),
            local_light_distance: finite(self.local_light_distance, 160.0).clamp(80.0, 600.0),
            local_headlight_strength: finite(self.local_headlight_strength, 1.15).clamp(0.0, 5.0),
            local_emergency_strength: finite(self.local_emergency_strength, 1.0).clamp(0.0, 5.0),
            local_light_limit: self.local_light_limit.clamp(8, 128),
            local_traffic_limit: self.local_traffic_limit.min(8),
            local_shadow_count: self.local_shadow_count.min(16),
            local_shadow_resolution: match self.local_shadow_resolution {
                0..=512 => 512,
                513..=1024 => 1024,
                _ => 2048,
            },
            local_shadow_distance: finite(self.local_shadow_distance, 80.0).clamp(20.0, 240.0),
            local_specular_strength: finite(self.local_specular_strength, 0.15).clamp(0.0, 1.0),
            local_shadow_filter: finite(self.local_shadow_filter, 1.0).clamp(0.0, 3.0),
            local_light_strength: finite(self.local_light_strength, 1.0).clamp(0.0, 5.0),
            local_street_strength: finite(self.local_street_strength, 1.5).clamp(0.0, 5.0),
            local_street_glow: finite(self.local_street_glow, 1.0).clamp(0.0, 2.0),
            sun_shaft_strength: finite(self.sun_shaft_strength, 1.0).clamp(0.0, 4.0),
            local_light_fog: finite(self.local_light_fog, 0.5).clamp(0.0, 4.0),
            local_light_fog_distance: finite(self.local_light_fog_distance, 200.0).clamp(80.0, 600.0),
            local_light_fog_quality: self.local_light_fog_quality.clamp(1, 3),
            local_street_haze_count: self.local_street_haze_count.min(64),
            local_light_fog_count: self.local_light_fog_count.min(64),
            local_street_range: finite(self.local_street_range, 35.0).clamp(10.0, 120.0),
            world_skylight_strength: finite(self.world_skylight_strength, 0.65).clamp(0.0, 1.0),
            parallax_depth: finite(self.parallax_depth, 4.0).clamp(0.0, 15.0),
            parallax_steps: self.parallax_steps.clamp(4, 64),
            parallax_distance: finite(self.parallax_distance, 40.0).clamp(5.0, 200.0),
            puddle_coverage: finite(self.puddle_coverage, 0.7).clamp(0.0, 1.0),
            puddle_scale: finite(self.puddle_scale, 7.0).clamp(2.0, 30.0),
            road_reflection_range: finite(self.road_reflection_range, 320.0).clamp(80.0, 800.0),
            road_reflection_fade: finite(self.road_reflection_fade, 0.65).clamp(0.1, 0.9),
            ray_shadow_samples: match self.ray_shadow_samples {
                2 => 2,
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
            wet_sheen: finite(self.wet_sheen, 0.6).clamp(0.0, 1.0),
            wet_light_strength: finite(self.wet_light_strength, 2.0).clamp(0.0, 5.0),
            car_night_reflections: finite(self.car_night_reflections, 2.0).clamp(0.25, 3.0),
            car_light_strength: finite(self.car_light_strength, 2.0).clamp(0.0, 5.0),
            car_lamp_size: finite(self.car_lamp_size, 0.25).clamp(0.0, 1.0),
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
mod tests;
