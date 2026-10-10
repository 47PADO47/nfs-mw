//! The option rows against renderers with different capabilities: rows it cannot do are left out and the toggles
//! cycle only through values it can run.

use super::graphics_options::GraphicsSetting;
use super::logic::Category;
use super::options::{Data, Setting, Title, rows};
use super::post_options::PostSetting;
use super::renderer_options::RendererSetting;
use crate::settings::test_caps;
use crate::settings::{
    GraphicsPreset, Partial, PostAa, RayTracingLevel, RendererKind, Settings, UpscaleMode, UpscaleQuality,
};

fn defaults() -> Settings {
    Settings::from(Partial::default())
}

fn titles(category: Category, caps: &blackbox_gfx::Capabilities) -> Vec<Title> {
    rows(category, caps).iter().map(|r| r.title).collect()
}

fn text(s: &'static str) -> Title {
    Title::Text(s)
}

#[test]
fn the_native_renderer_hides_ray_tracing_and_the_quality_mode_but_has_the_renderer_row() {
    let video = titles(Category::Video, &test_caps::native());
    assert_eq!(video.len(), 19);
    for hidden in ["Ray Tracing", "Upscale Quality"] {
        assert!(!video.contains(&text(hidden)), "{hidden} needs a renderer that offers it");
    }
    assert_eq!(
        video.last(),
        Some(&text("Renderer (After Restart)")),
        "after the rows that were there before, which keep their places"
    );
    assert_eq!(video[video.len() - 3..video.len() - 1], [text("Graphics Preset"), text("Car Shading")]);
    assert_eq!(titles(Category::Audio, &test_caps::native()).len(), 5);
    assert_eq!(titles(Category::Gameplay, &test_caps::native()).len(), 5);
}

#[test]
fn a_full_renderer_has_every_row_next_to_the_setting_it_belongs_with() {
    let video = titles(Category::Video, &test_caps::full());
    assert_eq!(video.len(), 21);
    let at =
        |name: &'static str| video.iter().position(|t| *t == text(name)).unwrap_or_else(|| panic!("no {name} row"));
    assert_eq!(at("Upscale Quality"), at("Upscale Sharpness") + 1);
    assert_eq!(at("Ray Tracing"), at("Anti-Aliasing") + 1);
    assert_eq!(at("Renderer (After Restart)"), video.len() - 1);
    assert_eq!(titles(Category::Audio, &test_caps::full()).len(), 5, "only the Video screen has them");
}

#[test]
fn the_renderer_row_says_it_applies_after_a_restart() {
    assert_eq!(RendererSetting::Renderer.title(), Title::Text("Renderer (After Restart)"));
    assert!(RendererSetting::Renderer.shown(&test_caps::native()));
}

#[test]
fn the_upscaler_row_cycles_only_what_the_native_renderer_runs() {
    let (mut s, mut c, caps) = (defaults(), Partial::default(), test_caps::native());
    let row = Setting::Upscaler;
    assert_eq!(row.data_in(&s, &caps), Data::Text("FSR 1".into()));
    for name in ["Off", "Bilinear", "FSR 1", "Off"] {
        assert!(row.advance(&mut s, &mut c, true, &caps));
        assert_eq!(row.data_in(&s, &caps), Data::Text(name.into()));
    }
    row.advance(&mut s, &mut c, false, &caps);
    assert_eq!(s.upscaler, UpscaleMode::Fsr1, "left from off skips fsr3, fsr4 and dlss");
    assert_eq!(c.upscaler, Some(UpscaleMode::Fsr1));
}

#[test]
fn the_upscaler_row_cycles_all_six_on_a_full_renderer() {
    let (mut s, mut c, caps) = (defaults(), Partial::default(), test_caps::full());
    for name in ["FSR 3", "FSR 4", "DLSS", "Off", "Bilinear", "FSR 1"] {
        Setting::Upscaler.advance(&mut s, &mut c, true, &caps);
        assert_eq!(Setting::Upscaler.data_in(&s, &caps), Data::Text(name.into()));
    }
}

#[test]
fn anti_aliasing_cycles_off_and_fxaa_on_the_native_renderer_and_all_four_on_a_full_one() {
    let aa = Setting::Post(PostSetting::Aa);
    let (mut s, mut c, native) = (defaults(), Partial::default(), test_caps::native());
    aa.advance(&mut s, &mut c, true, &native);
    assert_eq!(s.post_aa, PostAa::Fxaa);
    aa.advance(&mut s, &mut c, true, &native);
    assert_eq!(s.post_aa, PostAa::Off, "smaa and taa are skipped");
    aa.advance(&mut s, &mut c, false, &native);
    assert_eq!(s.post_aa, PostAa::Fxaa, "backwards too");
    let (mut s, full) = (defaults(), test_caps::full());
    let seen: Vec<_> = (0..4)
        .map(|_| {
            aa.advance(&mut s, &mut c, true, &full);
            s.post_aa
        })
        .collect();
    assert_eq!(seen, [PostAa::Fxaa, PostAa::Smaa, PostAa::Taa, PostAa::Off]);
}

#[test]
fn a_value_stored_for_another_renderer_shows_as_what_runs_and_moves_to_a_value_that_works() {
    let native = test_caps::native();
    let stored = Settings::from(Partial {
        post_aa: Some(PostAa::Taa),
        upscaler: Some(UpscaleMode::Dlss),
        ray_tracing: Some(RayTracingLevel::High),
        ..Partial::default()
    });
    let aa = Setting::Post(PostSetting::Aa);
    assert_eq!(aa.data_in(&stored, &native), Data::Text("FXAA".into()));
    assert_eq!(Setting::Upscaler.data_in(&stored, &native), Data::Text("FSR 1".into()));
    assert_eq!(Setting::Renderer(RendererSetting::RayTracing).data_in(&stored, &native), Data::Text("Off".into()));
    assert!(!aa.offered(&stored, &native) && !Setting::Upscaler.offered(&stored, &native));
    let (mut s, mut c) = (stored, Partial::default());
    aa.advance(&mut s, &mut c, true, &native);
    assert_eq!(s.post_aa, PostAa::Off, "forward from taa wraps to off");
    let mut s = stored;
    aa.advance(&mut s, &mut c, false, &native);
    assert_eq!(s.post_aa, PostAa::Fxaa, "backward from taa skips smaa");
    // Only what the player changed is touched: the ray tracing value for another renderer stays.
    assert_eq!((s.ray_tracing, s.upscaler), (RayTracingLevel::High, UpscaleMode::Dlss));
    assert_eq!(c.ray_tracing, None);
}

#[test]
fn the_renderer_rows_cycle_and_record_only_their_own_change() {
    let full = test_caps::full();
    let (mut s, mut c) = (defaults(), Partial::default());
    let rt = Setting::Renderer(RendererSetting::RayTracing);
    for name in ["Low", "Medium", "High", "Off"] {
        assert!(rt.advance(&mut s, &mut c, true, &full));
        assert_eq!(rt.data_in(&s, &full), Data::Text(name.into()));
    }
    assert_eq!(c, Partial { ray_tracing: Some(RayTracingLevel::Off), ..Partial::default() });

    let (mut s, mut c) = (defaults(), Partial::default());
    let quality = Setting::Renderer(RendererSetting::UpscaleQuality);
    assert_eq!(quality.data_in(&s, &full), Data::Text("Quality".into()));
    for name in ["Balanced", "Performance", "Ultra Performance", "Auto", "Native", "Quality"] {
        quality.advance(&mut s, &mut c, true, &full);
        assert_eq!(quality.data_in(&s, &full), Data::Text(name.into()));
    }
    assert_eq!(c, Partial { upscale_quality: Some(UpscaleQuality::Quality), ..Partial::default() });

    let (mut s, mut c, renderer) = (defaults(), Partial::default(), Setting::Renderer(RendererSetting::Renderer));
    assert_eq!(renderer.data_in(&s, &test_caps::native()), Data::Text("Black Box".into()));
    renderer.advance(&mut s, &mut c, true, &test_caps::native());
    assert_eq!((s.renderer, c.renderer), (RendererKind::Bevy, Some(RendererKind::Bevy)));
    assert_eq!(renderer.data_in(&s, &test_caps::native()), Data::Text("Bevy".into()));
    renderer.advance(&mut s, &mut c, false, &test_caps::native());
    assert_eq!(s.renderer, RendererKind::Blackbox);
}

#[test]
fn the_preset_row_reaches_ultra_and_the_native_renderer_shows_it_as_high() {
    let native = test_caps::native();
    let (mut s, mut c) = (defaults(), Partial::default());
    let preset = Setting::Graphics(GraphicsSetting::Preset);
    for name in ["Low", "Medium", "High", "Ultra"] {
        preset.advance(&mut s, &mut c, true, &native);
        assert_eq!(preset.data_in(&s, &native), Data::Text(name.into()));
    }
    assert_eq!(s.graphics_preset, GraphicsPreset::Ultra);
    assert_eq!(Setting::Post(PostSetting::Aa).data_in(&s, &native), Data::Text("FXAA".into()));
    assert_eq!(c.ray_tracing, Some(RayTracingLevel::Medium), "the preset writes every key it sets");
    preset.advance(&mut s, &mut c, true, &native);
    assert_eq!(preset.data_in(&s, &native), Data::Text("Custom".into()), "forward from ultra wraps to custom");
}
