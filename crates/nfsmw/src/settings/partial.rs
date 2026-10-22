//! One layer of settings: every field may be unset.

use blackbox_gfx::GraphicsApi;

use super::custom_fonts::CustomFontsLayer;
use super::hd_content::HdContentLayer;
use super::{
    CameraMode, CarShading, Deadzone, GraphicsPreset, HudLayout, MinimapMode, Monitor, PostAa, PostBloom, PostTonemap,
    RenderScale, RendererKind, Resolution, Sensitivity, SmokeQuality, SpeedUnits, Transmission, UiStyle, UiTheme,
    UpscaleMode, WindowMode,
};
use super::{RayTracingLevel, UpscaleQuality};
use crate::app::pacing::MaxFps;
use crate::devtools::{ShowMetrics, ShowReadout};

/// A volume setting in percent, 0 to 100.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Percent(pub u8);

impl Percent {
    /// As a linear amplitude, 0 to 1.
    pub fn amplitude(self) -> f32 {
        f32::from(self.0) / 100.0
    }
}

impl std::str::FromStr for Percent {
    type Err = String;

    /// `70`, `70%` or `0.7`.
    fn from_str(s: &str) -> Result<Self, String> {
        let text = s.trim().trim_end_matches('%');
        let value: f32 = text.parse().map_err(|_| format!("expected a volume from 0 to 100, got {s:?}"))?;
        let percent = if text.contains('.') && value <= 1.0 { value * 100.0 } else { value };
        if !(0.0..=100.0).contains(&percent) {
            return Err(format!("a volume is 0 to 100, got {s:?}"));
        }
        Ok(Percent(percent.round() as u8))
    }
}

impl std::fmt::Display for Percent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The settings one source (command line, environment, config file) sets. Unset fields fall
/// through to the next layer.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Partial {
    pub traffic_policy: Option<super::TrafficPolicy>,
    pub detail: Option<super::DetailSettings>,
    pub appearance: Option<super::Appearance>,
    pub deadzone_mode: Option<super::DeadzoneMode>,
    pub steering_deadzone: Option<Deadzone>,
    pub camera_deadzone: Option<Deadzone>,
    pub trigger_deadzone: Option<Deadzone>,
    pub steering_sensitivity: Option<Sensitivity>,
    pub camera_sensitivity: Option<Sensitivity>,
    pub mouse_sensitivity: Option<Sensitivity>,
    pub invert_camera_y: Option<bool>,
    pub backend: Option<GraphicsApi>,
    pub renderer: Option<RendererKind>,
    pub reflex: Option<super::ReflexMode>,
    pub frame_generation: Option<super::FrameGeneration>,
    pub vsync: Option<bool>,
    pub max_fps: Option<MaxFps>,
    pub show_metrics: Option<ShowMetrics>,
    pub window_mode: Option<WindowMode>,
    pub monitor: Option<Monitor>,
    pub resolution: Option<Resolution>,
    pub show_readout: Option<ShowReadout>,
    pub master_volume: Option<Percent>,
    pub music_volume: Option<Percent>,
    pub sfx_volume: Option<Percent>,
    pub engine_volume: Option<Percent>,
    pub speech_volume: Option<Percent>,
    pub hud: Option<bool>,
    pub tire_smoke: Option<bool>,
    pub radio: Option<bool>,
    pub skip_intro: Option<bool>,
    pub graphics_preset: Option<GraphicsPreset>,
    pub car_shading: Option<CarShading>,
    pub traffic: Option<u32>,
    pub cop_share: Option<Percent>,
    pub always_rain: Option<bool>,
    pub rain_amount: Option<Percent>,
    /// `GameSpeed`: the multiplier of the physics clock (1 is the original speed).
    pub game_speed: Option<f32>,
    /// `WorldAnimationSpeed`: the multiplier of the world animation clock (1 is the original rate).
    pub animation_speed: Option<f32>,
    pub car_scale: Option<f32>,
    /// `NoRevLimiter`: lift the engine's rev limiter.
    pub no_rev_limiter: Option<bool>,
    /// `ShowHUDOnLookBack`: keep the HUD up while looking back.
    pub show_hud_on_look_back: Option<bool>,
    pub traffic_lights: Option<bool>,
    pub traffic_avoidance: Option<bool>,
    pub cop_health_bars: Option<bool>,
    pub smoke_quality: Option<SmokeQuality>,
    pub skid_marks: Option<bool>,
    pub collision_sparks: Option<bool>,
    pub spark_style: Option<super::SparkStyle>,
    pub speed_trails: Option<bool>,
    pub speed_trail_style: Option<super::SpeedTrailStyle>,
    pub exhaust_flames: Option<bool>,
    /// Show turn signals on cars while steering (`[Gameplay] ShowTurnSignals`); off by default.
    pub turn_signals: Option<bool>,
    pub rear_view_mirror: Option<bool>,
    pub discord: Option<bool>,
    pub discord_time_played: Option<bool>,
    /// Which mods may load (`mods`).
    pub mods: Option<super::mods::ModsMode>,
    pub transmission: Option<Transmission>,
    pub camera_mode: Option<CameraMode>,
    pub minimap: Option<MinimapMode>,
    pub hud_layout: Option<HudLayout>,
    /// The UI theme (`ui_style`). The legacy `radio_hud` key is mapped onto it by the reader of each layer.
    pub ui_style: Option<UiStyle>,
    /// The HUD's speed unit (`speed_units`).
    pub speed_units: Option<SpeedUnits>,
    /// The palette of the token-driven HUD (`ui_theme`).
    pub ui_theme: Option<UiTheme>,
    pub post_tonemap: Option<PostTonemap>,
    pub post_bloom: Option<PostBloom>,
    pub post_aa: Option<PostAa>,
    pub texture_filtering: Option<super::TextureFiltering>,
    pub road_reflections: Option<super::RoadReflections>,
    pub road_reflection_resolution: Option<super::RoadReflectionResolution>,
    pub visual_treatment: Option<super::VisualTreatment>,
    pub reflections: Option<super::ReflectionQuality>,
    pub reflection_cube_size: Option<super::ReflectionSize>,
    pub reflection_mirror_size: Option<super::ReflectionSize>,
    pub reflection_pow2: Option<bool>,
    pub mirror_tint: Option<super::MirrorTint>,
    /// How far the reflection captures reach (`reflection_distance`).
    pub reflection_distance: Option<blackbox_gfx::reflection::distance::ReflectionDistance>,
    /// The car cube's scenery detail: its own face or the main view's (`reflection_lod`).
    pub reflection_lod: Option<blackbox_gfx::reflection::lod::ReflectionLod>,
    /// What the car cube adds to the scenery (`reflection_content`).
    pub reflection_content: Option<blackbox_gfx::reflection::content::ReflectionContent>,
    /// The vehicles' environment reflection brightness, a factor (`vehicle_reflection_brightness`).
    pub vehicle_reflection_brightness: Option<blackbox_gfx::reflection::brightness::VehicleBrightness>,
    pub shadows: Option<super::ShadowQuality>,
    pub ao: Option<super::AoQuality>,
    pub ao_intensity: Option<Percent>,
    pub ao_compare: Option<bool>,
    pub ao_depth_mips: Option<bool>,
    pub ao_method: Option<super::AoMethod>,
    pub render_scale: Option<RenderScale>,
    pub upscaler: Option<UpscaleMode>,
    pub upscale_sharpness: Option<Percent>,
    pub upscale_quality: Option<UpscaleQuality>,
    pub ray_tracing: Option<RayTracingLevel>,
    pub paddle_up: Option<u32>,
    /// `[Hotkeys] Headlights` of the Extra Options file: a Windows virtual-key code for the headlights.
    pub headlights_key: Option<u32>,
    pub paddle_down: Option<u32>,
    pub manual_clutch: Option<bool>,
    pub h_shifter: Option<bool>,
    /// The high-speed radial blur on or off (`motion_blur`).
    pub motion_blur: Option<bool>,
    /// The radial blur's strength multiplier, 0 to 2 (`motion_blur_strength`).
    pub motion_blur_strength: Option<f32>,
    /// The HD content options (`[hd_content]`).
    pub hd_content: HdContentLayer,
    /// The custom fonts (`[custom_fonts]`).
    pub custom_fonts: CustomFontsLayer,
    /// The front-end shadows (`fe_shadows*`).
    pub fe_shadows: super::fe_shadows::FeShadowsLayer,
    /// The LAN play settings (`net_*`).
    pub net: super::net::NetLayer,
}

impl Partial {
    /// This layer, with `lower` filling the fields it leaves unset.
    pub fn or(self, lower: Partial) -> Partial {
        Partial {
            traffic_policy: self.traffic_policy.or(lower.traffic_policy),
            detail: self.detail.or(lower.detail),
            appearance: self.appearance.or(lower.appearance),
            deadzone_mode: self.deadzone_mode.or(lower.deadzone_mode),
            steering_deadzone: self.steering_deadzone.or(lower.steering_deadzone),
            camera_deadzone: self.camera_deadzone.or(lower.camera_deadzone),
            trigger_deadzone: self.trigger_deadzone.or(lower.trigger_deadzone),
            steering_sensitivity: self.steering_sensitivity.or(lower.steering_sensitivity),
            camera_sensitivity: self.camera_sensitivity.or(lower.camera_sensitivity),
            mouse_sensitivity: self.mouse_sensitivity.or(lower.mouse_sensitivity),
            invert_camera_y: self.invert_camera_y.or(lower.invert_camera_y),
            backend: self.backend.or(lower.backend),
            renderer: self.renderer.or(lower.renderer),
            reflex: self.reflex.or(lower.reflex),
            frame_generation: self.frame_generation.or(lower.frame_generation),
            vsync: self.vsync.or(lower.vsync),
            max_fps: self.max_fps.or(lower.max_fps),
            show_metrics: self.show_metrics.or(lower.show_metrics),
            window_mode: self.window_mode.or(lower.window_mode),
            monitor: self.monitor.or(lower.monitor),
            resolution: self.resolution.or(lower.resolution),
            show_readout: self.show_readout.or(lower.show_readout),
            master_volume: self.master_volume.or(lower.master_volume),
            music_volume: self.music_volume.or(lower.music_volume),
            sfx_volume: self.sfx_volume.or(lower.sfx_volume),
            engine_volume: self.engine_volume.or(lower.engine_volume),
            speech_volume: self.speech_volume.or(lower.speech_volume),
            hud: self.hud.or(lower.hud),
            tire_smoke: self.tire_smoke.or(lower.tire_smoke),
            radio: self.radio.or(lower.radio),
            skip_intro: self.skip_intro.or(lower.skip_intro),
            graphics_preset: self.graphics_preset.or(lower.graphics_preset),
            car_shading: self.car_shading.or(lower.car_shading),
            traffic: self.traffic.or(lower.traffic),
            cop_share: self.cop_share.or(lower.cop_share),
            always_rain: self.always_rain.or(lower.always_rain),
            rain_amount: self.rain_amount.or(lower.rain_amount),
            game_speed: self.game_speed.or(lower.game_speed),
            animation_speed: self.animation_speed.or(lower.animation_speed),
            car_scale: self.car_scale.or(lower.car_scale),
            no_rev_limiter: self.no_rev_limiter.or(lower.no_rev_limiter),
            show_hud_on_look_back: self.show_hud_on_look_back.or(lower.show_hud_on_look_back),
            traffic_lights: self.traffic_lights.or(lower.traffic_lights),
            traffic_avoidance: self.traffic_avoidance.or(lower.traffic_avoidance),
            cop_health_bars: self.cop_health_bars.or(lower.cop_health_bars),
            smoke_quality: self.smoke_quality.or(lower.smoke_quality),
            skid_marks: self.skid_marks.or(lower.skid_marks),
            collision_sparks: self.collision_sparks.or(lower.collision_sparks),
            spark_style: self.spark_style.or(lower.spark_style),
            speed_trails: self.speed_trails.or(lower.speed_trails),
            speed_trail_style: self.speed_trail_style.or(lower.speed_trail_style),
            exhaust_flames: self.exhaust_flames.or(lower.exhaust_flames),
            turn_signals: self.turn_signals.or(lower.turn_signals),
            rear_view_mirror: self.rear_view_mirror.or(lower.rear_view_mirror),
            discord: self.discord.or(lower.discord),
            discord_time_played: self.discord_time_played.or(lower.discord_time_played),
            mods: self.mods.or(lower.mods),
            transmission: self.transmission.or(lower.transmission),
            camera_mode: self.camera_mode.or(lower.camera_mode),
            minimap: self.minimap.or(lower.minimap),
            hud_layout: self.hud_layout.or(lower.hud_layout),
            ui_style: self.ui_style.or(lower.ui_style),
            speed_units: self.speed_units.or(lower.speed_units),
            ui_theme: self.ui_theme.or(lower.ui_theme),
            post_tonemap: self.post_tonemap.or(lower.post_tonemap),
            post_bloom: self.post_bloom.or(lower.post_bloom),
            post_aa: self.post_aa.or(lower.post_aa),
            texture_filtering: self.texture_filtering.or(lower.texture_filtering),
            road_reflections: self.road_reflections.or(lower.road_reflections),
            road_reflection_resolution: self.road_reflection_resolution.or(lower.road_reflection_resolution),
            visual_treatment: self.visual_treatment.or(lower.visual_treatment),
            reflections: self.reflections.or(lower.reflections),
            reflection_cube_size: self.reflection_cube_size.or(lower.reflection_cube_size),
            reflection_mirror_size: self.reflection_mirror_size.or(lower.reflection_mirror_size),
            reflection_pow2: self.reflection_pow2.or(lower.reflection_pow2),
            mirror_tint: self.mirror_tint.or(lower.mirror_tint),
            reflection_distance: self.reflection_distance.or(lower.reflection_distance),
            reflection_lod: self.reflection_lod.or(lower.reflection_lod),
            reflection_content: self.reflection_content.or(lower.reflection_content),
            vehicle_reflection_brightness: self.vehicle_reflection_brightness.or(lower.vehicle_reflection_brightness),
            shadows: self.shadows.or(lower.shadows),
            ao: self.ao.or(lower.ao),
            ao_intensity: self.ao_intensity.or(lower.ao_intensity),
            ao_compare: self.ao_compare.or(lower.ao_compare),
            ao_depth_mips: self.ao_depth_mips.or(lower.ao_depth_mips),
            ao_method: self.ao_method.or(lower.ao_method),
            render_scale: self.render_scale.or(lower.render_scale),
            upscaler: self.upscaler.or(lower.upscaler),
            upscale_sharpness: self.upscale_sharpness.or(lower.upscale_sharpness),
            upscale_quality: self.upscale_quality.or(lower.upscale_quality),
            ray_tracing: self.ray_tracing.or(lower.ray_tracing),
            paddle_up: self.paddle_up.or(lower.paddle_up),
            headlights_key: self.headlights_key.or(lower.headlights_key),
            paddle_down: self.paddle_down.or(lower.paddle_down),
            manual_clutch: self.manual_clutch.or(lower.manual_clutch),
            h_shifter: self.h_shifter.or(lower.h_shifter),
            motion_blur: self.motion_blur.or(lower.motion_blur),
            motion_blur_strength: self.motion_blur_strength.or(lower.motion_blur_strength),
            hd_content: self.hd_content.or(lower.hd_content),
            custom_fonts: self.custom_fonts.or(lower.custom_fonts),
            fe_shadows: self.fe_shadows.or(lower.fe_shadows),
            net: self.net.or(lower.net),
        }
    }
}

/// Parse a boolean written as `1/0`, `true/false`, `on/off` or `yes/no`.
pub fn parse_bool(s: &str) -> Result<bool, String> {
    match s.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "on" | "yes" => Ok(true),
        "0" | "false" | "off" | "no" => Ok(false),
        _ => Err(format!("expected on or off, got {s:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upper_layer_wins_and_gaps_fall_through() {
        let cli = Partial { backend: Some(GraphicsApi::Gl), ..Partial::default() };
        let file = Partial { backend: Some(GraphicsApi::Vulkan), vsync: Some(false), ..Partial::default() };
        let merged = cli.or(file);
        assert_eq!(merged.backend, Some(GraphicsApi::Gl));
        assert_eq!(merged.vsync, Some(false));
        assert_eq!(merged.max_fps, None);
    }

    #[test]
    fn volumes_read_percent_or_fraction() {
        assert_eq!("70".parse(), Ok(Percent(70)));
        assert_eq!("70%".parse(), Ok(Percent(70)));
        assert_eq!("0.5".parse(), Ok(Percent(50)));
        assert_eq!("1".parse(), Ok(Percent(1)));
        assert!("101".parse::<Percent>().is_err());
        assert!("loud".parse::<Percent>().is_err());
    }

    #[test]
    fn booleans() {
        assert_eq!(parse_bool("On"), Ok(true));
        assert_eq!(parse_bool("0"), Ok(false));
        assert!(parse_bool("maybe").is_err());
    }
}
