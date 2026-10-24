use super::*;
use crate::settings::Partial;

fn defaults() -> Settings {
    Settings::from(Partial::default())
}

#[test]
fn detail_commands_validate_ranges_and_share_menu_profile_values() {
    let mut s = defaults();
    set(&mut s, "world_detail", "medium").unwrap();
    set(&mut s, "car_geometry", "high").unwrap();
    set(&mut s, "geometry_range", "2x").unwrap();
    set(&mut s, "visibility_range", "1.5x").unwrap();
    set(&mut s, "highest_player", "on").unwrap();
    assert_eq!(s.detail.world, crate::settings::WorldDetail::Medium);
    assert_eq!(s.detail.geometry_range, crate::settings::detail::DetailRange::Double);
    assert_eq!(get(&s, "car_geometry").unwrap(), "car_geometry = high");
    let before = s;
    assert!(set(&mut s, "geometry_range", "9x").is_err());
    assert_eq!(s, before);
}

#[test]
fn sets_and_reads_back() {
    let mut s = defaults();
    assert_eq!(set(&mut s, "fps", "60").unwrap(), "fps = 60");
    assert_eq!(set(&mut s, "vsync", "off").unwrap(), "vsync = off");
    assert_eq!(set(&mut s, "metrics", "advanced").unwrap(), "metrics = advanced");
    assert_eq!(set(&mut s, "readout", "full").unwrap(), "readout = full");
    assert!(!s.vsync);
    assert_eq!(s.show_metrics, ShowMetrics::Advanced);
    assert_eq!(s.show_readout, ShowReadout::Full);
    assert_eq!(set(&mut s, "fps", "unlocked").unwrap(), "fps = unlocked");
}

#[test]
fn window_preferences_can_change_and_invalid_values_do_not_mutate_them() {
    let mut s = defaults();
    assert_eq!(set(&mut s, "window_mode", "exclusive").unwrap(), "window_mode = exclusive");
    assert_eq!(set(&mut s, "monitor", "0").unwrap(), "monitor = 0");
    assert_eq!(set(&mut s, "resolution", "1920x1080").unwrap(), "resolution = 1920x1080");
    let before = s;
    assert!(set(&mut s, "window_mode", "bad").is_err());
    assert!(set(&mut s, "monitor", "-1").is_err());
    assert!(set(&mut s, "resolution", "0x0").is_err());
    assert_eq!(s, before);
}

#[test]
fn the_speech_volume_is_set_like_the_other_volumes() {
    let mut s = defaults();
    assert_eq!(set(&mut s, "speech_volume", "35").unwrap(), "speech_volume = 35");
    assert_eq!(s.speech_volume.0, 35);
    assert_eq!(get(&s, "speech_volume").unwrap(), "speech_volume = 35");
    assert!(set(&mut s, "speech_volume", "").unwrap_err().starts_with("usage: set speech_volume <0-100>"));
}

#[test]
fn the_transmission_is_set_by_name() {
    let mut s = defaults();
    assert_eq!(get(&s, "transmission").unwrap(), "transmission = automatic");
    assert_eq!(set(&mut s, "transmission", "manual").unwrap(), "transmission = manual");
    assert_eq!(s.transmission, Transmission::Manual);
    assert!(set(&mut s, "transmission", "sport").is_err());
    assert_eq!(s.transmission, Transmission::Manual);
}

#[test]
fn the_wheel_switches_start_off_and_can_be_set_or_flipped() {
    let mut s = defaults();
    assert_eq!(get(&s, "manual_clutch").unwrap(), "manual_clutch = off");
    assert_eq!(get(&s, "h_shifter").unwrap(), "h_shifter = off");
    assert_eq!(set(&mut s, "manual_clutch", "on").unwrap(), "manual_clutch = on");
    assert_eq!(set(&mut s, "h_shifter", "").unwrap(), "h_shifter = on");
    assert!(s.wheel_options().manual_clutch && s.wheel_options().h_shifter);
    assert!(set(&mut s, "manual_clutch", "pedal").is_err());
    assert!(s.manual_clutch, "a bad value leaves the setting alone");
    assert!(get_all(&s).contains("h_shifter = on"));
}

#[test]
fn set_without_a_value_flips_a_switch_and_explains_anything_else() {
    let mut s = defaults();
    assert_eq!(set(&mut s, "vsync", "").unwrap(), "vsync = off");
    assert_eq!(set(&mut s, "vsync", "").unwrap(), "vsync = on");
    assert_eq!(set(&mut s, "radio", "").unwrap(), "radio = off");
    assert!(!s.radio);
    let before = s;
    assert_eq!(set(&mut s, "fps", "").unwrap_err(), "usage: set fps <unlocked|number> (now unlocked)");
    assert_eq!(set(&mut s, "metrics", "").unwrap_err(), "usage: set metrics <off|basic|advanced> (now off)");
    assert!(set(&mut s, "volume", "").unwrap_err().starts_with("usage: set volume <0-100> (now 80)"));
    assert!(set(&mut s, "backend", "").unwrap_err().contains("restart"));
    assert!(set(&mut s, "bass", "").unwrap_err().contains("unknown setting"));
    assert_eq!(s, before);
}

#[test]
fn every_setting_has_a_switch_or_a_syntax() {
    let mut s = defaults();
    for key in KEYS.into_iter().filter(|k| *k != "backend") {
        let answer = set(&mut s, key, "");
        assert!(answer.is_ok() || answer.unwrap_err().starts_with("usage: set "), "{key}");
    }
}

#[test]
fn the_renderer_is_set_by_name_and_applies_after_a_restart() {
    let mut s = defaults();
    assert_eq!(get(&s, "renderer").unwrap(), "renderer = blackbox");
    assert_eq!(set(&mut s, "renderer", "bevy").unwrap(), "renderer = bevy (applies after restart)");
    assert_eq!(s.renderer, RendererKind::Bevy);
    assert_eq!(get(&s, "renderer").unwrap(), "renderer = bevy");
    let before = s;
    assert!(set(&mut s, "renderer", "wgpu").unwrap_err().contains("expected blackbox or bevy"));
    assert_eq!(s, before);
    assert_eq!(set(&mut s, "renderer", "").unwrap_err(), "usage: set renderer <blackbox|bevy> (now bevy)");
}

#[test]
fn upscale_quality_and_ray_tracing_are_set_by_name_and_validated() {
    let mut s = defaults();
    assert_eq!(get(&s, "upscale_quality").unwrap(), "upscale_quality = quality");
    assert_eq!(get(&s, "ray_tracing").unwrap(), "ray_tracing = off");
    assert_eq!(set(&mut s, "upscale_quality", "ultra_performance").unwrap(), "upscale_quality = ultra_performance");
    assert_eq!(set(&mut s, "ray_tracing", "high").unwrap(), "ray_tracing = high");
    let before = s;
    assert!(set(&mut s, "upscale_quality", "extreme").unwrap_err().contains("expected auto, native"));
    assert!(set(&mut s, "ray_tracing", "path").unwrap_err().contains("expected off, low, medium or high"));
    assert_eq!(s, before);
    assert_eq!(set(&mut s, "ray_tracing", "").unwrap_err(), "usage: set ray_tracing <off|low|medium|high> (now high)");
}

#[test]
fn the_minimap_mode_is_set_by_name() {
    let mut s = defaults();
    assert_eq!(get(&s, "minimap").unwrap(), "minimap = fixed");
    assert_eq!(set(&mut s, "minimap", "rotating").unwrap(), "minimap = rotating");
    assert_eq!(s.minimap, MinimapMode::Rotating);
    assert!(set(&mut s, "minimap", "zoomed").is_err());
    assert_eq!(s.minimap, MinimapMode::Rotating);
}

#[test]
fn the_upscaling_settings_are_set_by_name_and_validated() {
    let mut s = defaults();
    assert_eq!(get(&s, "render_scale").unwrap(), "render_scale = 100");
    assert_eq!(get(&s, "upscaler").unwrap(), "upscaler = fsr1");
    assert_eq!(get(&s, "upscale_sharpness").unwrap(), "upscale_sharpness = 80");
    assert_eq!(set(&mut s, "render_scale", "67%").unwrap(), "render_scale = 67");
    assert_eq!(set(&mut s, "upscaler", "bilinear").unwrap(), "upscaler = bilinear");
    assert_eq!(set(&mut s, "upscale_sharpness", "25").unwrap(), "upscale_sharpness = 25");
    let before = s;
    assert!(set(&mut s, "render_scale", "10").is_err());
    assert!(set(&mut s, "upscaler", "xess").is_err());
    assert!(set(&mut s, "upscale_sharpness", "101").is_err());
    assert_eq!(s, before);
    assert_eq!(set(&mut s, "render_scale", "").unwrap_err(), "usage: set render_scale <50-200> (now 67)");
}

#[test]
fn the_reflection_sizes_and_mirror_tint_are_set_by_name_and_validated() {
    let mut s = defaults();
    assert_eq!(get(&s, "reflection_cube_size").unwrap(), "reflection_cube_size = 100%");
    assert_eq!(set(&mut s, "reflection_cube_size", "256").unwrap(), "reflection_cube_size = 256");
    assert_eq!(set(&mut s, "reflection_mirror_size", "150%").unwrap(), "reflection_mirror_size = 150%");
    assert_eq!(set(&mut s, "reflection_pow2", "on").unwrap(), "reflection_pow2 = on");
    assert_eq!(set(&mut s, "mirror_tint", "on").unwrap(), "mirror_tint = #969696");
    let before = s;
    assert!(set(&mut s, "reflection_cube_size", "10%").is_err());
    assert!(set(&mut s, "mirror_tint", "300 0 0").is_err());
    assert_eq!(s, before);
}

#[test]
fn the_reflection_reach_detail_content_and_brightness_are_set_by_name_and_validated() {
    let mut s = defaults();
    assert_eq!(get(&s, "reflection_distance").unwrap(), "reflection_distance = unlimited");
    assert_eq!(set(&mut s, "reflection_distance", "original").unwrap(), "reflection_distance = original");
    assert_eq!(set(&mut s, "reflection_lod", "view").unwrap(), "reflection_lod = view");
    assert_eq!(set(&mut s, "reflection_content", "restored").unwrap(), "reflection_content = restored");
    assert_eq!(set(&mut s, "vehicle_reflection_brightness", "150").unwrap(), "vehicle_reflection_brightness = 150%");
    let before = s;
    assert!(set(&mut s, "reflection_distance", "far").is_err());
    assert!(set(&mut s, "reflection_lod", "480").is_err());
    assert!(set(&mut s, "vehicle_reflection_brightness", "301").is_err());
    assert_eq!(s, before);
}

#[test]
fn a_preset_sets_its_keys_and_changing_one_makes_it_custom() {
    let mut s = defaults();
    assert_eq!(get(&s, "graphics_preset").unwrap(), "graphics_preset = custom");
    assert_eq!(set(&mut s, "graphics_preset", "low").unwrap(), "graphics_preset = low");
    assert_eq!(get(&s, "car_shading").unwrap(), "car_shading = simple");
    assert_eq!(get(&s, "render_scale").unwrap(), "render_scale = 75");
    assert_eq!(get(&s, "upscaler").unwrap(), "upscaler = bilinear");
    assert_eq!(set(&mut s, "post_bloom", "low").unwrap(), "post_bloom = low");
    assert_eq!(get(&s, "graphics_preset").unwrap(), "graphics_preset = custom");
    assert_eq!(get(&s, "car_shading").unwrap(), "car_shading = simple", "the rest stays");
    assert!(set(&mut s, "graphics_preset", "extreme").is_err());
    assert_eq!(
        set(&mut s, "graphics_preset", "").unwrap_err(),
        "usage: set graphics_preset <custom|low|medium|high|ultra> (now custom)"
    );
    assert_eq!(set(&mut s, "car_shading", "glossy").unwrap(), "car_shading = glossy");
}

#[test]
fn bad_values_and_keys_leave_settings_alone() {
    let mut s = defaults();
    assert!(set(&mut s, "fps", "fast").is_err());
    assert!(set(&mut s, "vsync", "maybe").is_err());
    assert!(set(&mut s, "backend", "gl").unwrap_err().contains("restart"));
    assert!(set(&mut s, "volume", "loud").is_err());
    assert!(set(&mut s, "bass", "11").unwrap_err().contains("unknown setting"));
    assert_eq!(s, defaults());
}

#[test]
fn the_traffic_lights_can_be_read_set_and_flipped() {
    let mut s = defaults();
    assert_eq!(get(&s, "traffic_lights").unwrap(), "traffic_lights = on");
    assert_eq!(set(&mut s, "traffic_lights", "off").unwrap(), "traffic_lights = off");
    assert!(!s.traffic_lights);
    assert_eq!(set(&mut s, "traffic_lights", "").unwrap(), "traffic_lights = on");
    assert!(set(&mut s, "traffic_lights", "red").is_err());
    assert!(s.traffic_lights);
}

#[test]
fn the_cop_health_bars_are_off_by_default_and_can_be_read_set_and_flipped() {
    let mut s = defaults();
    assert!(!s.cop_health_bars);
    assert_eq!(get(&s, "cop_health_bars").unwrap(), "cop_health_bars = off");
    assert_eq!(set(&mut s, "cop_health_bars", "on").unwrap(), "cop_health_bars = on");
    assert!(s.cop_health_bars);
    assert_eq!(set(&mut s, "cop_health_bars", "").unwrap(), "cop_health_bars = off");
    assert!(set(&mut s, "cop_health_bars", "red").is_err());
    assert!(!s.cop_health_bars);
}

#[test]
fn get_all_lists_every_key() {
    let text = get_all(&defaults());
    assert_eq!(text.lines().count(), KEYS.len());
    assert!(text.contains("vsync = on"));
}

#[test]
fn control_changes_are_validated_and_invalid_edit_is_atomic() {
    let mut s = defaults();
    set(&mut s, "steering_deadzone", "10").unwrap();
    set(&mut s, "camera_sensitivity", "200").unwrap();
    set(&mut s, "invert_camera_y", "").unwrap();
    assert_eq!(get(&s, "steering_deadzone").unwrap(), "steering_deadzone = 10");
    assert_eq!(get(&s, "camera_sensitivity").unwrap(), "camera_sensitivity = 200");
    assert!(s.controls.invert_camera_y);
    let before = s;
    assert!(set(&mut s, "trigger_deadzone", "100").is_err());
    assert!(set(&mut s, "mouse_sensitivity", "0").is_err());
    assert_eq!(s, before);
}

#[test]
fn the_front_end_shadow_keys_set_and_read_back() {
    let mut s = defaults();
    assert_eq!(set(&mut s, "fe_shadows", "on").unwrap(), "fe_shadows = on");
    assert!(s.fe_shadows.enabled);
    assert_eq!(get(&s, "fe_shadows").unwrap(), "fe_shadows = on");
    assert_eq!(set(&mut s, "fe_shadows_cars", "off").unwrap(), "fe_shadows_cars = off");
    assert!(!s.fe_shadows.cars);
    assert_eq!(set(&mut s, "fe_shadows_bias", "9").unwrap(), "fe_shadows_bias = 4");
    assert_eq!(set(&mut s, "fe_shadows_divider", "4").unwrap(), "fe_shadows_divider = 4");
    assert!(set(&mut s, "fe_shadows_divider", "8").is_err());
    assert_eq!(s.fe_shadows.divider, 4, "a rejected value changes nothing");
    assert!(set(&mut s, "fe_shadows", "sometimes").is_err());
}

#[test]
fn traffic_population_commands_preserve_independent_range_and_context() {
    let mut s = defaults();
    set(&mut s, "traffic_preset", "extended").unwrap();
    set(&mut s, "traffic_density", "30").unwrap();
    assert_eq!(set(&mut s, "traffic_population", "very-high").unwrap(), "traffic_population = very-high");
    assert_eq!(s.traffic, 64);
    assert_eq!(s.traffic_policy.density_percent, 30);
    assert_eq!(s.traffic_preset(), crate::settings::TrafficPreset::Extended);
    let before = s;
    assert!(set(&mut s, "traffic_population", "bad").is_err());
    assert_eq!(s, before);
}

#[test]
fn the_hud_keys_set_and_read_back() {
    let mut s = defaults();
    assert_eq!(get(&s, "custom_hud.map_mode").unwrap(), "custom_hud.map_mode = centered");
    assert_eq!(set(&mut s, "custom_hud.scale", "1.25").unwrap(), "custom_hud.scale = 1.25");
    assert_eq!(set(&mut s, "custom_hud.camera_shake", "3").unwrap(), "custom_hud.camera_shake = 1", "clamped");
    assert_eq!(set(&mut s, "custom_hud.palette", "tritanopia").unwrap(), "custom_hud.palette = tritanopia");
    assert_eq!(set(&mut s, "custom_hud.haptics", "").unwrap(), "custom_hud.haptics = on", "a switch flips");
    assert_eq!(set(&mut s, "custom_hud.preset", "minimal").unwrap(), "custom_hud.preset = minimal");
    assert_eq!(get(&s, "custom_hud.map_mode").unwrap(), "custom_hud.map_mode = compact", "the preset applies at once");
    assert!(s.hud, "the old hud switch is a separate key");
    let before = s;
    assert!(set(&mut s, "custom_hud.map_mode", "huge").is_err());
    assert!(set(&mut s, "custom_hud.opacity", "lots").is_err());
    assert_eq!(s, before, "a rejected value changes nothing");
    assert!(set(&mut s, "custom_hud.map_zoom", "").unwrap_err().starts_with("usage: set custom_hud.map_zoom <0.5-2>"));
}
