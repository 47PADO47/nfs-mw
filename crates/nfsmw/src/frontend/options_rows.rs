//! Category layout for the option screens.
use super::hd_options::HdSetting;
use super::{LABEL_TRANSMISSION, Row, Setting, Title, row};
use crate::frontend::alias_options::AliasSetting;
use crate::frontend::graphics_options::GraphicsSetting;
use crate::frontend::input_options::InputSetting;
use crate::frontend::logic::Category;
use crate::frontend::post_options::PostSetting;
use crate::frontend::renderer_options::RendererSetting;
use blackbox_gfx::Capabilities;

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
            row(Setting::SpeechVolume, Title::Label(0x9E5F_B82A)),
        ],
        Category::Video => vec![row(Setting::WindowMode, Title::Text("Window Mode"))]
            .into_iter()
            .chain(offered(RendererSetting::Renderer))
            .chain(offered(RendererSetting::GraphicsApi))
            .chain([
                row(Setting::Vsync, Title::Label(0x6CEB_9CB6)),
                row(Setting::MaxFps, Title::Text("Frame Limit")),
                row(Setting::Graphics(GraphicsSetting::Style), GraphicsSetting::Style.title()),
                row(Setting::Graphics(GraphicsSetting::Quality), GraphicsSetting::Quality.title()),
                row(Setting::AdvancedGraphics, Title::Text("Advanced Graphics")),
                row(Setting::RenderScale, Title::Text("Render Scale")),
                row(Setting::Upscaler, Title::Text("Upscaler")),
            ])
            .chain(offered(RendererSetting::UpscaleQuality))
            .chain([row(Setting::UpscaleSharpness, Title::Text("Sharpening"))])
            .chain(offered(RendererSetting::FrameGeneration))
            .chain(PostSetting::ALL.into_iter().map(|setting| row(Setting::Post(setting), setting.title())))
            .chain([renderer_row(RendererSetting::RayTracing)])
            .chain(
                GraphicsSetting::ALL
                    .into_iter()
                    .filter(|setting| {
                        !matches!(
                            setting,
                            GraphicsSetting::Preset
                                | GraphicsSetting::Style
                                | GraphicsSetting::Quality
                                | GraphicsSetting::AoIntensity
                                | GraphicsSetting::AoComparison
                        ) && setting.shown(caps)
                    })
                    .flat_map(|setting| {
                        [
                            Some(row(Setting::Graphics(setting), setting.title())),
                            (setting == GraphicsSetting::CarGeometry)
                                .then(|| row(Setting::TrafficPreset, Title::Text("Traffic Range Preset"))),
                        ]
                        .into_iter()
                        .flatten()
                    }),
            )
            .chain([
                row(Setting::TireSmoke, Title::Text("Tire Smoke")),
                row(Setting::SmokeQuality, Title::Text("Smoke Quality")),
                row(Setting::SkidMarks, Title::Text("Skid Marks")),
                row(Setting::CollisionSparks, Title::Text("Collision Sparks")),
                row(Setting::SpeedTrails, Title::Text("Speed Trails (Experimental)")),
                row(Setting::SpeedTrailStyle, Title::Text("Speed Trail Style")),
                row(Setting::ExhaustFlames, Title::Text("Exhaust Flames")),
                row(Setting::CustomFonts, Title::Text("Custom Fonts")),
                row(Setting::Hd(HdSetting::Fonts), HdSetting::Fonts.title()),
                row(Setting::Hd(HdSetting::Cursor), HdSetting::Cursor.title()),
                row(Setting::Hd(HdSetting::Movies), HdSetting::Movies.title()),
                row(Setting::Metrics, Title::Text("Performance Overlay")),
            ])
            .collect(),
        Category::Gameplay => vec![
            row(Setting::HudLayout, Title::Text("HUD Layout")),
            row(Setting::RadioHud, Title::Text("Radio HUD")),
            row(Setting::Minimap, Title::Text("Minimap")),
            row(Setting::RearViewMirror, Title::Text("Rear-View Mirror")),
            row(Setting::TurnSignals, Title::Text("Turn Signals")),
            row(Setting::CopHealthBars, Title::Text("Cop Health Bars")),
        ],
        Category::Player => vec![
            row(Setting::Transmission, Title::Label(LABEL_TRANSMISSION)),
            row(Setting::Camera, Title::Label(0xF6CC_DC5F)),
            row(Setting::Hud, Title::Label(0xAC14_8579)),
        ],
        Category::Controls => {
            InputSetting::ALL.into_iter().map(|setting| row(Setting::Input(setting), setting.title())).collect()
        }
        Category::AliasManager => AliasSetting::ALL
            .into_iter()
            .map(|setting| row(Setting::Alias(setting), Title::Text(setting.title())))
            .collect(),
        // The rows of the Mods page are the mods' own (`bb.menus`), read from the bridge by `widget_menu.rs`.
        Category::Mods => Vec::new(),
        // The Discord Rich Presence toggles (`discord/`); the time played is dimmed while Discord is off.
        Category::Discord => vec![
            row(Setting::Discord, Title::Text("Discord Rich Presence")),
            row(Setting::DiscordTimePlayed, Title::Text("Show Time Played")),
        ],
    }
}
