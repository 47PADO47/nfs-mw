//! `get` and `set`: reading and changing [`Settings`] from the console. Changes last for this run;
//! the config file is not written.

use std::str::FromStr;

use super::settings_keys::KEYS;
use crate::app::pacing::MaxFps;
use crate::devtools::{ShowMetrics, ShowReadout};
use crate::settings::custom_fonts;
use crate::settings::fe_shadows;
use crate::settings::hd_content;
use crate::settings::hud;
use crate::settings::net;
use crate::settings::{
    CameraMode, HudLayout, MinimapMode, Percent, PostAa, PostBloom, PostTonemap, RadioHudStyle, RenderScale,
    RendererKind, Settings, SpeedUnits, Transmission, UiStyle, parse_bool,
};

/// The text for `get <key>`, or an error naming the valid keys.
pub fn get(settings: &Settings, key: &str) -> Result<String, String> {
    let value = match key {
        "traffic_preset" => settings.traffic_preset().to_string(),
        "traffic_population" => settings.population_preset().to_string(),
        "traffic_density" => settings.traffic_policy.density_percent.to_string(),
        "world_detail" => settings.detail.world.to_string(),
        "car_geometry" => settings.detail.car.to_string(),
        "geometry_range" => settings.detail.geometry_range.to_string(),
        "visibility_range" => settings.detail.visibility_range.to_string(),
        "highest_world" => on_off(settings.detail.highest_world).to_owned(),
        "highest_player" => on_off(settings.detail.highest_player).to_owned(),
        "deadzone_mode" => settings.controls.deadzone_mode.to_string(),
        "steering_deadzone" => settings.controls.steering_deadzone.to_string(),
        "camera_deadzone" => settings.controls.camera_deadzone.to_string(),
        "trigger_deadzone" => settings.controls.trigger_deadzone.to_string(),
        "steering_sensitivity" => settings.controls.steering_sensitivity.to_string(),
        "camera_sensitivity" => settings.controls.camera_sensitivity.to_string(),
        "mouse_sensitivity" => settings.controls.mouse_sensitivity.to_string(),
        "invert_camera_y" => on_off(settings.controls.invert_camera_y).to_owned(),
        "backend" => settings.backend.to_string(),
        "renderer" => settings.renderer.to_string(),
        "vsync" => on_off(settings.vsync).to_owned(),
        "fps" | "max_fps" => settings.max_fps.to_string(),
        "metrics" | "show_metrics" => settings.show_metrics.to_string(),
        "window_mode" => settings.window_mode.to_string(),
        "monitor" => settings.monitor.to_string(),
        "resolution" => settings.resolution.to_string(),
        "readout" | "show_readout" => settings.show_readout.to_string(),
        "volume" | "master_volume" => settings.master_volume.to_string(),
        "music_volume" => settings.music_volume.to_string(),
        "sfx_volume" => settings.sfx_volume.to_string(),
        "engine_volume" => settings.engine_volume.to_string(),
        "speech_volume" => settings.speech_volume.to_string(),
        "hud" => on_off(settings.hud).to_owned(),
        "tire_smoke" => on_off(settings.tire_smoke).to_owned(),
        "radio" => on_off(settings.radio).to_owned(),
        "graphics_preset" => settings.graphics_preset.to_string(),
        "car_shading" => settings.car_shading.to_string(),
        "traffic" => settings.traffic.to_string(),
        "cop_share" => settings.cop_share.to_string(),
        "traffic_lights" => on_off(settings.traffic_lights).to_owned(),
        "traffic_avoidance" => on_off(settings.traffic_avoidance).to_owned(),
        "cop_health_bars" => on_off(settings.cop_health_bars).to_owned(),
        "smoke_quality" => settings.smoke_quality.to_string(),
        "skid_marks" => on_off(settings.skid_marks).to_owned(),
        "collision_sparks" => on_off(settings.collision_sparks).to_owned(),
        "spark_style" => settings.spark_style.to_string(),
        "speed_trails" => on_off(settings.speed_trails).to_owned(),
        "speed_trail_style" => settings.speed_trail_style.to_string(),
        "exhaust_flames" => on_off(settings.exhaust_flames).to_owned(),
        "enhanced_exhaust" => on_off(settings.enhanced_exhaust).to_owned(),
        "turn_signals" => on_off(settings.turn_signals).to_owned(),
        "transmission" => settings.transmission.to_string(),
        "camera_mode" => settings.camera_mode.to_string(),
        "minimap" => settings.minimap.to_string(),
        "hud_layout" | "hud-layout" => settings.hud_layout.to_string(),
        "ui_style" | "ui-style" | "radio_hud" | "radio-hud" => settings.ui_style.to_string(),
        "speed_units" | "speed-units" => settings.speed_units.to_string(),
        "manual_clutch" => on_off(settings.manual_clutch).to_owned(),
        "h_shifter" => on_off(settings.h_shifter).to_owned(),
        key if hd_content::is_key(key) => hd_content::get(&settings.hd_content, key),
        key if custom_fonts::is_key(key) => custom_fonts::get(&settings.custom_fonts, key),
        key if fe_shadows::is_key(key) => fe_shadows::get(&settings.fe_shadows, key),
        key if net::is_key(key) => net::get(&settings.net, key),
        key if hud::is_key(key) => hud::get(&settings.hud_options, key),
        "motion_blur" => on_off(settings.motion_blur).to_owned(),
        "motion_blur_strength" => settings.motion_blur_strength.to_string(),
        "post_tonemap" => settings.post_tonemap.to_string(),
        "post_bloom" => settings.post_bloom.to_string(),
        "post_aa" => settings.post_aa.to_string(),
        "visual_treatment" => settings.visual_treatment.to_string(),
        "reflections" => settings.reflections.to_string(),
        "reflection_cube_size" => settings.reflection_cube_size.to_string(),
        "reflection_mirror_size" => settings.reflection_mirror_size.to_string(),
        "reflection_pow2" => on_off(settings.reflection_pow2).to_owned(),
        "mirror_tint" => settings.mirror_tint.to_string(),
        "reflection_distance" => settings.reflection_extra.distance.to_string(),
        "reflection_lod" => settings.reflection_extra.lod.to_string(),
        "reflection_content" => settings.reflection_extra.content.to_string(),
        "vehicle_reflection_brightness" => settings.reflection_extra.brightness.to_string(),
        "shadows" => settings.shadows.to_string(),
        "ao" => settings.ao.to_string(),
        "ao_intensity" => settings.ao_intensity.to_string(),
        "ao_compare" => on_off(settings.ao_compare).to_owned(),
        "ao_depth_mips" => on_off(settings.ao_depth_mips).to_owned(),
        "ao_method" => settings.ao_method.to_string(),
        "render_scale" => settings.render_scale.to_string(),
        "upscaler" => settings.upscaler.to_string(),
        "upscale_sharpness" => settings.upscale_sharpness.to_string(),
        "upscale_quality" => settings.upscale_quality.to_string(),
        "ray_tracing" => settings.ray_tracing.to_string(),
        other => return Err(unknown(other)),
    };
    Ok(format!("{key} = {value}"))
}

/// Every setting, one per line.
pub fn get_all(settings: &Settings) -> String {
    KEYS.iter().filter_map(|k| get(settings, k).ok()).collect::<Vec<_>>().join("\n")
}

/// Change a setting. The message says what happened.
/// An empty `value` is a `set` without one: a switch flips, anything else answers with its usage.
pub fn set(settings: &mut Settings, key: &str, value: &str) -> Result<String, String> {
    if value.is_empty() {
        return set_without_value(settings, key);
    }
    match key {
        "traffic_preset" => settings.select_traffic_preset(value.parse()?),
        "traffic_population" => settings.select_population_preset(value.parse()?),
        "traffic_density" => settings.traffic_policy.density_percent = value.parse::<Percent>()?.0,
        "world_detail" => settings.detail.world = value.parse()?,
        "car_geometry" => settings.detail.car = value.parse()?,
        "geometry_range" => settings.detail.geometry_range = value.parse()?,
        "visibility_range" => settings.detail.visibility_range = value.parse()?,
        "highest_world" => settings.detail.highest_world = parse_bool(value)?,
        "highest_player" => settings.detail.highest_player = parse_bool(value)?,
        "deadzone_mode" => settings.controls.deadzone_mode = value.parse()?,
        "steering_deadzone" => settings.controls.steering_deadzone = value.parse()?,
        "camera_deadzone" => settings.controls.camera_deadzone = value.parse()?,
        "trigger_deadzone" => settings.controls.trigger_deadzone = value.parse()?,
        "steering_sensitivity" => settings.controls.steering_sensitivity = value.parse()?,
        "camera_sensitivity" => settings.controls.camera_sensitivity = value.parse()?,
        "mouse_sensitivity" => settings.controls.mouse_sensitivity = value.parse()?,
        "invert_camera_y" => settings.controls.invert_camera_y = parse_bool(value)?,
        "renderer" => settings.renderer = RendererKind::from_str(value)?,
        "vsync" => settings.vsync = parse_bool(value)?,
        "fps" | "max_fps" => settings.max_fps = MaxFps::from_str(value)?,
        "metrics" | "show_metrics" => settings.show_metrics = ShowMetrics::from_str(value)?,
        "window_mode" => settings.window_mode = value.parse()?,
        "monitor" => settings.monitor = value.parse()?,
        "resolution" => settings.resolution = value.parse()?,
        "readout" | "show_readout" => settings.show_readout = ShowReadout::from_str(value)?,
        "volume" | "master_volume" => settings.master_volume = Percent::from_str(value)?,
        "music_volume" => settings.music_volume = Percent::from_str(value)?,
        "sfx_volume" => settings.sfx_volume = Percent::from_str(value)?,
        "engine_volume" => settings.engine_volume = Percent::from_str(value)?,
        "speech_volume" => settings.speech_volume = Percent::from_str(value)?,
        "hud" => settings.hud = parse_bool(value)?,
        "tire_smoke" => settings.tire_smoke = parse_bool(value)?,
        "radio" => settings.radio = parse_bool(value)?,
        "graphics_preset" => settings.apply_preset(value.parse()?),
        "car_shading" => settings.car_shading = value.parse()?,
        "traffic" => {
            settings.traffic = value.trim().parse().map_err(|_| format!("expected a number of cars, got {value:?}"))?
        }
        "cop_share" => settings.cop_share = Percent::from_str(value)?,
        "traffic_lights" => settings.traffic_lights = parse_bool(value)?,
        "traffic_avoidance" => settings.traffic_avoidance = parse_bool(value)?,
        "cop_health_bars" => settings.cop_health_bars = parse_bool(value)?,
        "smoke_quality" => settings.smoke_quality = value.parse()?,
        "skid_marks" => settings.skid_marks = parse_bool(value)?,
        "collision_sparks" => settings.collision_sparks = parse_bool(value)?,
        "spark_style" => settings.spark_style = value.parse()?,
        "speed_trails" => settings.speed_trails = parse_bool(value)?,
        "speed_trail_style" => settings.speed_trail_style = value.parse()?,
        "exhaust_flames" => settings.exhaust_flames = parse_bool(value)?,
        "enhanced_exhaust" => settings.enhanced_exhaust = parse_bool(value)?,
        "turn_signals" => settings.turn_signals = parse_bool(value)?,
        "transmission" => settings.transmission = Transmission::from_str(value)?,
        "camera_mode" => settings.camera_mode = CameraMode::from_str(value)?,
        "minimap" => settings.minimap = MinimapMode::from_str(value)?,
        "hud_layout" | "hud-layout" => settings.hud_layout = HudLayout::from_str(value)?,
        "ui_style" | "ui-style" => settings.ui_style = UiStyle::from_str(value)?,
        "speed_units" | "speed-units" => settings.speed_units = SpeedUnits::from_str(value)?,
        // The legacy alias: ea_trax and custom, mapped onto the same setting.
        "radio_hud" | "radio-hud" => settings.ui_style = UiStyle::from(RadioHudStyle::from_str(value)?),
        "manual_clutch" => settings.manual_clutch = parse_bool(value)?,
        "h_shifter" => settings.h_shifter = parse_bool(value)?,
        "motion_blur" => settings.motion_blur = parse_bool(value)?,
        "motion_blur_strength" => settings.motion_blur_strength = crate::settings::motion_blur::parse_strength(value)?,
        "post_tonemap" => settings.post_tonemap = PostTonemap::from_str(value)?,
        "post_bloom" => settings.post_bloom = PostBloom::from_str(value)?,
        "post_aa" => settings.post_aa = PostAa::from_str(value)?,
        "visual_treatment" => settings.visual_treatment = value.parse()?,
        "reflections" => settings.reflections = value.parse()?,
        "reflection_cube_size" => settings.reflection_cube_size = value.parse()?,
        "reflection_mirror_size" => settings.reflection_mirror_size = value.parse()?,
        "reflection_pow2" => settings.reflection_pow2 = parse_bool(value)?,
        "mirror_tint" => settings.mirror_tint = value.parse()?,
        "reflection_distance" => settings.reflection_extra.distance = value.parse()?,
        "reflection_lod" => settings.reflection_extra.lod = value.parse()?,
        "reflection_content" => settings.reflection_extra.content = value.parse()?,
        "vehicle_reflection_brightness" => settings.reflection_extra.brightness = value.parse()?,
        "shadows" => settings.shadows = value.parse()?,
        "ao" => settings.ao = value.parse()?,
        "ao_intensity" => settings.ao_intensity = value.parse()?,
        "ao_compare" => settings.ao_compare = parse_bool(value)?,
        "ao_depth_mips" => settings.ao_depth_mips = parse_bool(value)?,
        "ao_method" => settings.ao_method = crate::settings::AoMethod::from_str(value)?,
        "render_scale" => settings.render_scale = RenderScale::from_str(value)?,
        "upscaler" => settings.upscaler = value.parse()?,
        "upscale_sharpness" => settings.upscale_sharpness = Percent::from_str(value)?,
        "upscale_quality" => settings.upscale_quality = value.parse()?,
        "ray_tracing" => settings.ray_tracing = value.parse()?,
        key if hd_content::is_key(key) => hd_content::set(&mut settings.hd_content, key, value)?,
        key if custom_fonts::is_key(key) => custom_fonts::set(&mut settings.custom_fonts, key, value)?,
        key if fe_shadows::is_key(key) => fe_shadows::set(&mut settings.fe_shadows, key, value)?,
        key if net::is_key(key) => net::set(&mut settings.net, key, value)?,
        key if hud::is_key(key) => hud::set(&mut settings.hud_options, key, value)?,
        "backend" => return Err("the graphics backend cannot change while running; restart with --backend".into()),
        other => return Err(unknown(other)),
    }
    settings.settle_preset();
    let text = get(settings, key)?;
    if key == "renderer" {
        return Ok(format!("{text} (applies after restart)"));
    }
    Ok(text)
}

/// The on/off settings, which `set <key>` flips.
fn switch<'a>(settings: &'a mut Settings, key: &str) -> Option<&'a mut bool> {
    match key {
        "highest_world" => Some(&mut settings.detail.highest_world),
        "highest_player" => Some(&mut settings.detail.highest_player),
        "vsync" => Some(&mut settings.vsync),
        "invert_camera_y" => Some(&mut settings.controls.invert_camera_y),
        "hud" => Some(&mut settings.hud),
        "tire_smoke" => Some(&mut settings.tire_smoke),
        "reflection_pow2" => Some(&mut settings.reflection_pow2),
        "skid_marks" => Some(&mut settings.skid_marks),
        "collision_sparks" => Some(&mut settings.collision_sparks),
        "speed_trails" => Some(&mut settings.speed_trails),
        "exhaust_flames" => Some(&mut settings.exhaust_flames),
        "enhanced_exhaust" => Some(&mut settings.enhanced_exhaust),
        "turn_signals" => Some(&mut settings.turn_signals),
        "radio" => Some(&mut settings.radio),
        "manual_clutch" => Some(&mut settings.manual_clutch),
        "h_shifter" => Some(&mut settings.h_shifter),
        "motion_blur" => Some(&mut settings.motion_blur),
        "ao_compare" => Some(&mut settings.ao_compare),
        "ao_depth_mips" => Some(&mut settings.ao_depth_mips),
        "traffic_lights" => Some(&mut settings.traffic_lights),
        "traffic_avoidance" => Some(&mut settings.traffic_avoidance),
        "cop_health_bars" => Some(&mut settings.cop_health_bars),
        key if custom_fonts::is_key(key) => custom_fonts::flag(&mut settings.custom_fonts, key),
        key if fe_shadows::is_key(key) => fe_shadows::flag(&mut settings.fe_shadows, key),
        key if net::is_key(key) => net::flag(&mut settings.net, key),
        key if hud::is_key(key) => hud::flag(&mut settings.hud_options, key),
        _ => hd_content::flag(&mut settings.hd_content, key),
    }
}

/// What a setting accepts, for the usage line.
fn syntax(key: &str) -> Option<&'static str> {
    Some(match key {
        "traffic_preset" => "<reference|extended|custom>",
        "traffic_population" => "<normal|high|very-high|custom>",
        "traffic_density" => "<0-100>",
        "world_detail" => "<low|medium|high|maximum>",
        "car_geometry" => "<low|high|highest>",
        "geometry_range" | "visibility_range" => "<1x|1.5x|2x>",
        "deadzone_mode" => "<rescaled|cutoff>",
        "steering_deadzone" | "camera_deadzone" | "trigger_deadzone" => "<0-95>",
        "steering_sensitivity" | "camera_sensitivity" | "mouse_sensitivity" => "<1-400>",
        "fps" | "max_fps" => "<unlocked|number>",
        "metrics" | "show_metrics" => "<off|basic|advanced>",
        "readout" | "show_readout" => "<off|minimal|full>",
        "window_mode" => "<windowed|borderless|exclusive>",
        "renderer" => "<blackbox|bevy>",
        "fe_shadows_bias" => "<0.25-4>",
        "fe_shadows_divider" => "<1|2|4>",
        "monitor" => "<current|primary|index>",
        "resolution" => "<WIDTHxHEIGHT|native>",
        "volume" | "master_volume" | "music_volume" | "sfx_volume" | "engine_volume" | "speech_volume" => "<0-100>",
        "graphics_preset" => "<custom|low|medium|high|ultra>",
        "car_shading" => "<simple|glossy|original|original-unshadowed>",
        "smoke_quality" => "<standard|high>",
        "spark_style" => "<original-pc|restored-experimental>",
        "speed_trail_style" => "<streaks|car>",
        "transmission" => "<automatic|manual>",
        "camera_mode" => "<chase|dynamic-fov|bumper|hood|close|far|superfar|drift|pursuit>",
        "minimap" => "<fixed|rotating|off>",
        "hud_layout" | "hud-layout" => "<pc|classic|xbox360>",
        "ui_style" | "ui-style" => "<original|custom|remastered>",
        "speed_units" | "speed-units" => "<metric|imperial>",
        "radio_hud" | "radio-hud" => "<ea_trax|custom> (alias of ui_style)",
        "post_tonemap" => "<off|aces>",
        "post_bloom" => "<off|low|medium|high>",
        "post_aa" => "<off|fxaa|smaa|taa>",
        "visual_treatment" => "<off|low|high>",
        "reflections" => "<off|low|medium|high>",
        "reflection_cube_size" | "reflection_mirror_size" => "<auto|25%-400%|pixels>",
        "mirror_tint" => "<off|on|#rrggbb|r g b>",
        "reflection_distance" => "<unlimited|original>",
        "reflection_lod" => "<capture|view>",
        "reflection_content" => "<original|restored>",
        "vehicle_reflection_brightness" => "<0%-300%>",
        "shadows" => "<off|low|medium|high>",
        "ao" => "<off|low|medium|high>",
        "ao_method" => "<gtao|ssao|xegtao>",
        "ao_intensity" => "<0-100>",
        "render_scale" => "<50-200>",
        "upscaler" => "<off|bilinear|fsr1|fsr3|fsr4|dlss>",
        "upscale_sharpness" => "<0-100>",
        "upscale_quality" => "<auto|native|quality|balanced|performance|ultra_performance>",
        "ray_tracing" => "<off|low|medium|high>",
        "traffic" => "<cars, 0 = off>",
        "cop_share" => "<0-100>",
        "motion_blur_strength" => "<0-2>",
        key if net::is_key(key) => return net::syntax(key),
        key if hud::is_key(key) => return hud::syntax(key),
        _ => return None,
    })
}

fn set_without_value(settings: &mut Settings, key: &str) -> Result<String, String> {
    if let Some(flag) = switch(settings, key) {
        *flag = !*flag;
        return get(settings, key);
    }
    if key == "backend" {
        return Err("the graphics backend cannot change while running; restart with --backend".into());
    }
    let Some(syntax) = syntax(key) else { return Err(unknown(key)) };
    let now = get(settings, key)?;
    let now = now.split_once(" = ").map_or(now.as_str(), |(_, value)| value);
    Err(format!("usage: set {key} {syntax} (now {now})"))
}

fn on_off(on: bool) -> &'static str {
    if on { "on" } else { "off" }
}

fn unknown(key: &str) -> String {
    format!("unknown setting {key:?} (settings: {})", KEYS.join(", "))
}

#[cfg(test)]
#[path = "settings_cmd_tests.rs"]
mod tests;
