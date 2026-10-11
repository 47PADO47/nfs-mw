//! Category layout for the option screens.
use super::{LABEL_TRANSMISSION, Row, Setting, Title, row};
use crate::frontend::graphics_options::GraphicsSetting;
use crate::frontend::input_options::InputSetting;
use crate::frontend::logic::Category;
use crate::frontend::post_options::PostSetting;
use crate::frontend::renderer_options::RendererSetting;
use blackbox_gfx::Capabilities;

/// The rows of a category on a renderer with these capabilities. A row the renderer cannot do (ray tracing, the
/// temporal upscalers' quality mode) is left out; dependencies of supported options can still be dimmed.
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
                row(Setting::Graphics(GraphicsSetting::Preset), GraphicsSetting::Preset.title()),
                row(Setting::RenderScale, Title::Text("Render Scale")),
                row(Setting::Upscaler, Title::Text("Upscaler")),
            ])
            .chain(offered(RendererSetting::UpscaleQuality))
            .chain([row(Setting::UpscaleSharpness, Title::Text("Upscale Sharpness"))])
            .chain(PostSetting::ALL.into_iter().map(|setting| row(Setting::Post(setting), setting.title())))
            .chain(offered(RendererSetting::RayTracing))
            .chain(
                GraphicsSetting::ALL
                    .into_iter()
                    .filter(|setting| *setting != GraphicsSetting::Preset)
                    .map(|setting| row(Setting::Graphics(setting), setting.title())),
            )
            .chain([
                row(Setting::TireSmoke, Title::Text("Tire Smoke")),
                row(Setting::SmokeQuality, Title::Text("Smoke Quality")),
                row(Setting::SkidMarks, Title::Text("Skid Marks")),
                row(Setting::CollisionSparks, Title::Text("Collision Sparks")),
                row(Setting::SpeedTrails, Title::Text("Speed Trails (Experimental)")),
                row(Setting::ExhaustFlames, Title::Text("Exhaust Flames")),
                row(Setting::Metrics, Title::Text("Performance Overlay")),
            ])
            .collect(),
        Category::Gameplay => vec![
            row(Setting::Hud, Title::Label(0xAC14_8579)),
            row(Setting::Transmission, Title::Label(LABEL_TRANSMISSION)),
            row(Setting::HudLayout, Title::Text("HUD Layout")),
            row(Setting::RadioHud, Title::Text("Radio HUD")),
            row(Setting::Minimap, Title::Text("Minimap")),
        ],
        Category::Controls => {
            InputSetting::ALL.into_iter().map(|setting| row(Setting::Input(setting), setting.title())).collect()
        }
    }
}
