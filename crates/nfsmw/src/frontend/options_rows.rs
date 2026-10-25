//! Category layout for the option screens.
use super::hd_options::HdSetting;
use super::hud_options::HudSetting;
use super::{LABEL_TRANSMISSION, Row, Setting, Title, row};
use crate::frontend::graphics_options::GraphicsSetting;
use crate::frontend::input_options::InputSetting;
use crate::frontend::logic::Category;
use crate::frontend::post_options::PostSetting;
use crate::frontend::renderer_options::RendererSetting;
use blackbox_gfx::Capabilities;

/// The original's Rearview Mirror row title (`OPT_GAMEPLAY_REARVIEW`).
const LABEL_REARVIEW: u32 = 0x85A6_CE05;

/// The rows of a category on a renderer with these capabilities. A row the renderer cannot do (the temporal
/// upscalers' quality mode) is left out; ray tracing is always listed, dimmed and reading "Unavailable" on a
/// renderer without it, so that the player can see it exists. Dependencies of supported options can be dimmed too.
pub fn rows(category: Category, caps: &Capabilities) -> Vec<Row> {
    let renderer_row = |setting: RendererSetting| row(Setting::Renderer(setting), setting.title());
    let offered = |setting: RendererSetting| setting.shown(caps).then(|| renderer_row(setting));
    match category {
        Category::Audio => vec![
            row(Setting::MasterVolume, Title::Label(0x2678_2C3E)),
            row(Setting::SfxVolume, Title::Label(0xFD48_7543)),
            row(Setting::EngineVolume, Title::Label(0xA2B1_F888)),
            row(Setting::MusicVolume, Title::Label(0x418E_681D)),
            // The speech volume is the police dispatch's radio, so the row says so (`speech_volume` keeps its key).
            row(Setting::SpeechVolume, Title::Key("options-police-dispatch-volume")),
        ],
        Category::Video => vec![row(Setting::WindowMode, Title::Key("options-window-mode"))]
            .into_iter()
            .chain(offered(RendererSetting::Renderer))
            .chain(offered(RendererSetting::GraphicsApi))
            .chain([
                row(Setting::Vsync, Title::Label(0x6CEB_9CB6)),
                row(Setting::MaxFps, Title::Key("options-frame-limit")),
                row(Setting::Graphics(GraphicsSetting::Style), GraphicsSetting::Style.title()),
                row(Setting::Graphics(GraphicsSetting::Quality), GraphicsSetting::Quality.title()),
                row(Setting::AdvancedGraphics, Title::Key("options-advanced-graphics")),
                row(Setting::RenderScale, Title::Key("options-render-scale")),
                row(Setting::Upscaler, Title::Key("options-upscaler")),
            ])
            .chain(offered(RendererSetting::UpscaleQuality))
            .chain([row(Setting::UpscaleSharpness, Title::Key("options-sharpening"))])
            .chain(offered(RendererSetting::FrameGeneration))
            .chain(
                PostSetting::ALL
                    .into_iter()
                    .filter(|setting| matches!(setting, PostSetting::Aa | PostSetting::MotionBlur))
                    .map(|setting| row(Setting::Post(setting), setting.title())),
            )
            .chain([renderer_row(RendererSetting::RayTracing)])
            .chain(
                GraphicsSetting::ALL
                    .into_iter()
                    .filter(|setting| {
                        matches!(
                            setting,
                            GraphicsSetting::Hdr
                                | GraphicsSetting::ShadowPreset
                                | GraphicsSetting::ReflectionPreset
                                | GraphicsSetting::WorldDetail
                                | GraphicsSetting::CarGeometry
                                | GraphicsSetting::TextureFiltering
                                | GraphicsSetting::AmbientOcclusion
                        ) && setting.shown(caps)
                    })
                    .flat_map(|setting| {
                        [
                            Some(row(Setting::Graphics(setting), setting.title())),
                            (setting == GraphicsSetting::CarGeometry)
                                .then(|| row(Setting::TrafficPreset, Title::Key("options-traffic-range-preset"))),
                        ]
                        .into_iter()
                        .flatten()
                    }),
            )
            .chain([
                row(Setting::TireSmoke, Title::Key("options-tire-smoke")),
                row(Setting::SmokeQuality, Title::Key("options-smoke-quality")),
                row(Setting::SkidMarks, Title::Key("options-skid-marks")),
                row(Setting::CollisionSparks, Title::Key("options-collision-sparks")),
                row(Setting::SpeedTrails, Title::Key("options-speed-trails")),
                row(Setting::SpeedTrailStyle, Title::Key("options-speed-trail-style")),
                row(Setting::ExhaustFlames, Title::Key("options-exhaust-flames")),
                row(Setting::CustomFonts, Title::Key("options-custom-fonts")),
                row(Setting::Hd(HdSetting::Fonts), HdSetting::Fonts.title()),
                row(Setting::Hd(HdSetting::Cursor), HdSetting::Cursor.title()),
                row(Setting::Hd(HdSetting::Movies), HdSetting::Movies.title()),
                row(Setting::Metrics, Title::Key("options-performance-overlay")),
            ])
            .collect(),
        Category::Gameplay => vec![
            row(Setting::HudLayout, Title::Key("options-hud-layout")),
            row(Setting::UiStyle, Title::Key("options-hud-style")),
            row(Setting::UiTheme, Title::Key("options-hud-theme")),
            row(Setting::SpeedUnits, Title::Key("options-speed-units")),
            row(Setting::Minimap, Title::Key("options-minimap")),
            row(Setting::RearViewMirror, Title::Label(LABEL_REARVIEW)),
            row(Setting::TurnSignals, Title::Key("options-turn-signals")),
            row(Setting::CopHealthBars, Title::Key("options-cop-health-bars")),
            // The general game options live here; the original has no label that names the language row.
            row(Setting::Language, Title::Key("options-language")),
        ],
        Category::Player => vec![
            row(Setting::Transmission, Title::Label(LABEL_TRANSMISSION)),
            row(Setting::Camera, Title::Label(0xF6CC_DC5F)),
            row(Setting::Hud, Title::Label(0xAC14_8579)),
        ],
        Category::Controls => {
            InputSetting::ALL.into_iter().map(|setting| row(Setting::Input(setting), setting.title())).collect()
        }
        // The rows of the Mods page are the mods' own (`bb.menus`), read from the bridge by `widget_menu.rs`.
        Category::Mods => Vec::new(),
        Category::CustomRace => crate::frontend::custom_race::options::rows()
            .into_iter()
            .map(|custom| row(Setting::Custom(custom), Title::Label(custom.title())))
            .collect(),
        // The HUD page: one row per `[custom_hud]` key, the map's orientation, then the speed units after it
        // (docs/specs/custom-hud/menus.md, section 4).
        Category::Hud => HudSetting::ALL
            .into_iter()
            .flat_map(|setting| {
                let units = (setting == HudSetting::MapOrientation)
                    .then(|| row(Setting::SpeedUnits, Title::Key("options-speed-units")));
                [Some(row(Setting::CustomHud(setting), setting.title())), units].into_iter().flatten()
            })
            .collect(),
        // The Discord Rich Presence toggles (`discord/`); the others are dimmed while Discord is off.
        Category::Discord => vec![
            row(Setting::Discord, Title::Key("options-discord-presence")),
            row(Setting::DiscordTimePlayed, Title::Key("options-discord-time-played")),
            row(Setting::DiscordJoin, Title::Key("options-discord-join")),
        ],
    }
}
