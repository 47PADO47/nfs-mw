//! What each row's data string shows: a key of the language table where the original has the word, else a message of
//! the rewrite's catalogue, else a name that is not language (FSR 1, Xbox 360).

use super::*;
use crate::i18n::t;
use crate::settings::Transmission;

/// The original's camera values (`OPT_PLAYER_CAM_*`), the ones the rewrite's camera list shares with it.
const LABEL_CAM_BUMPER: u32 = 0xC3E9_AE58;
const LABEL_CAM_HOOD: u32 = 0x414F_19D7;
const LABEL_CAM_CLOSE: u32 = 0x5AE3_441F;
const LABEL_CAM_FAR: u32 = 0x1EA4_CEC2;
const LABEL_CAM_SUPER_FAR: u32 = 0x9160_39B4;

impl Setting {
    /// What the data string shows for a toggle.
    pub fn data(self, s: &Settings) -> Data {
        match self {
            Setting::AdvancedGraphics => message("options-open-f8"),
            Setting::Custom(row) => super::super::custom_race::options::data(row),
            Setting::TrafficPreset => Data::Text(s.traffic_preset().menu_label()),
            Setting::Input(setting) => setting.data(s),
            Setting::Post(setting) => setting.data(s),
            Setting::Graphics(setting) => setting.data(s),
            Setting::Renderer(setting) => setting.data(s),
            Setting::UpscaleSharpness => steps::sharpening(s.upscale_sharpness),
            Setting::Hd(setting) => setting.data(s),
            Setting::CustomHud(setting) => setting.data(s),
            Setting::Language => language::data(),
            Setting::HudLayout => match s.hud_layout {
                // The platform names are not language.
                HudLayout::Pc => Data::Text("PC".into()),
                HudLayout::Classic => message("options-hud-layout-centered"),
                HudLayout::Xbox360 => Data::Text("Xbox 360".into()),
            },
            Setting::UiStyle => Data::Text(s.ui_style.label()),
            Setting::SpeedUnits => Data::Text(s.speed_units.label()),
            Setting::UiTheme => Data::Text(s.ui_theme.label()),
            Setting::Minimap => match s.minimap {
                MinimapMode::Fixed => message("options-minimap-fixed"),
                MinimapMode::Rotating => message("options-minimap-rotating"),
                MinimapMode::Off => Data::Label(LABEL_OFF),
            },
            Setting::RenderScale => Data::Text(crate::i18n::fmt::percent(i64::from(s.render_scale.percent()))),
            Setting::Upscaler => match s.upscaler {
                UpscaleMode::Off => Data::Label(LABEL_OFF),
                UpscaleMode::Bilinear => message("options-upscaler-bilinear"),
                // The upscalers' names are not language.
                UpscaleMode::Fsr1 => Data::Text("FSR 1".into()),
                UpscaleMode::Fsr3 => Data::Text("FSR 3".into()),
                UpscaleMode::Fsr4 => Data::Text("FSR 4".into()),
                UpscaleMode::Dlss => Data::Text("DLSS".into()),
            },
            Setting::Vsync => on_off(s.vsync),
            Setting::Hud => on_off(s.hud),
            Setting::TireSmoke => on_off(s.tire_smoke),
            Setting::SkidMarks => on_off(s.skid_marks),
            Setting::CollisionSparks if !s.collision_sparks => on_off(false),
            Setting::CollisionSparks => Data::Text(s.spark_style.label()),
            Setting::SpeedTrails => on_off(s.speed_trails),
            Setting::SpeedTrailStyle => Data::Text(s.speed_trail_style.label()),
            Setting::ExhaustFlames => on_off(s.exhaust_flames),
            Setting::CustomFonts => on_off(s.custom_fonts.enabled),
            Setting::TurnSignals => on_off(s.turn_signals),
            Setting::CopHealthBars => on_off(s.cop_health_bars),
            Setting::RearViewMirror => on_off(s.rear_view_mirror),
            Setting::Discord => on_off(s.discord),
            Setting::DiscordTimePlayed => on_off(s.discord_time_played),
            Setting::SmokeQuality => match s.smoke_quality {
                SmokeQuality::Standard => message("options-standard"),
                SmokeQuality::High => message("options-high"),
            },
            Setting::WindowMode => match s.window_mode {
                WindowMode::Windowed => message("options-window-windowed"),
                WindowMode::Borderless => message("options-window-borderless"),
                WindowMode::Exclusive => message("options-window-exclusive"),
            },
            Setting::Transmission => Data::Label(match s.transmission {
                Transmission::Automatic => LABEL_AUTOMATIC,
                Transmission::Manual => LABEL_MANUAL,
            }),
            Setting::Camera => match s.camera_mode {
                CameraMode::Chase => message("options-camera-chase"),
                CameraMode::DynamicFov => message("options-camera-dynamic-fov"),
                CameraMode::Bumper => Data::Label(LABEL_CAM_BUMPER),
                CameraMode::Hood => Data::Label(LABEL_CAM_HOOD),
                CameraMode::Close => Data::Label(LABEL_CAM_CLOSE),
                CameraMode::Far => Data::Label(LABEL_CAM_FAR),
                CameraMode::SuperFar => Data::Label(LABEL_CAM_SUPER_FAR),
                CameraMode::Drift => message("options-camera-drift"),
                CameraMode::Pursuit => message("options-camera-pursuit"),
            },
            Setting::MaxFps => match s.max_fps.to_string().as_str() {
                "unlocked" => message("options-frame-limit-unlocked"),
                fps => Data::Text(t!("options-frame-limit-fps", fps = fps)),
            },
            Setting::Metrics => match s.show_metrics {
                ShowMetrics::Off => Data::Label(LABEL_OFF),
                ShowMetrics::Basic => message("options-metrics-basic"),
                ShowMetrics::Advanced => message("options-metrics-advanced"),
            },
            _ => Data::Text(String::new()),
        }
    }
}
