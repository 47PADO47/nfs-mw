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
fn the_native_renderer_hides_the_quality_mode_and_dims_ray_tracing() {
    let native = test_caps::native();
    let video = titles(Category::Video, &native);
    assert_eq!(video.len(), 41);
    for hidden in ["Upscale Quality", "Dynamic Soft Shadows"] {
        assert!(!video.contains(&text(hidden)), "{hidden} needs a renderer that offers it");
    }
    let rt = Setting::Renderer(RendererSetting::RayTracing);
    assert!(video.contains(&text("Ray Tracing")), "ray tracing is listed on every renderer");
    assert!(!rt.enabled_in(&defaults(), &native), "but it is dimmed where it cannot run");
    assert_eq!(rt.data_in(&defaults(), &native), Data::Text("Unavailable".into()));
    let (mut s, mut c) = (defaults(), Partial::default());
    assert!(!rt.advance(&mut s, &mut c, true, &native), "and it does not change");
    assert_eq!(c, Partial::default());
    assert!(rt.enabled_in(&defaults(), &test_caps::full()), "a renderer with ray tracing lets it change");
    assert_eq!(
        video[1..3],
        [text("Renderer (After Restart)"), text("Graphics API (After Restart)")],
        "renderer choices sit with the display controls"
    );
    assert_eq!(video[5], text("Graphics Preset"));
    assert_eq!(titles(Category::Audio, &test_caps::native()).len(), 5);
    assert_eq!(titles(Category::Gameplay, &test_caps::native()).len(), 6);
    assert_eq!(titles(Category::Player, &test_caps::native()).len(), 3);
    assert_eq!(titles(Category::Discord, &test_caps::native()).len(), 2, "the Discord toggles have their own category");
}

#[test]
fn a_full_renderer_has_every_row_next_to_the_setting_it_belongs_with() {
    let video = titles(Category::Video, &test_caps::full());
    assert_eq!(video.len(), 45);
    let at =
        |name: &'static str| video.iter().position(|t| *t == text(name)).unwrap_or_else(|| panic!("no {name} row"));
    assert_eq!(
        at("Front-End Shadows"),
        at("Dynamic Soft Shadows") + 1,
        "the front-end row sits beside the shadows row"
    );
    assert_eq!(at("Upscale Quality"), at("Upscaler") + 1);
    assert_eq!(at("HD Fonts"), at("Custom Fonts") + 1, "the HD content rows follow the custom fonts row");
    assert_eq!(at("HD Movies"), at("HD Fonts") + 2);
    assert_eq!(at("Ray Tracing"), at("Motion Blur Strength") + 1, "ray tracing follows the post-processing rows");
    assert_eq!(
        at("Car Reflection Size"),
        at("Car Reflections") + 1,
        "the HD reflection rows follow the reflection row"
    );
    assert_eq!(at("Dynamic Soft Shadows"), at("Mirror Tint") + 1);
    assert_eq!(at("Vehicle Reflection Brightness"), at("Power-of-Two Cube Size") + 1, "beside the cube rows");
    assert_eq!(
        at("Mirror Reflection Size"),
        at("Reflection Distance") + 1,
        "the reach row sits beside the mirror size"
    );
    assert_eq!(at("Renderer (After Restart)"), 1);
    assert_eq!(at("Graphics API (After Restart)"), 2);
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
fn a_native_renderer_with_fsr3_cycles_it_and_shows_the_quality_mode() {
    let caps = test_caps::native_fsr3();
    let video = titles(Category::Video, &caps);
    assert!(video.contains(&text("Upscale Quality")), "a temporal upscaler brings its quality row");
    let (mut s, mut c) = (defaults(), Partial::default());
    for name in ["FSR 3", "Off", "Bilinear", "FSR 1"] {
        assert!(Setting::Upscaler.advance(&mut s, &mut c, true, &caps));
        assert_eq!(Setting::Upscaler.data_in(&s, &caps), Data::Text(name.into()));
    }
    Setting::Upscaler.advance(&mut s, &mut c, false, &caps);
    assert_eq!(s.upscaler, UpscaleMode::Bilinear, "left from FSR 1 goes back one");
    s.upscaler = UpscaleMode::Fsr3;
    assert!(Setting::Renderer(RendererSetting::UpscaleQuality).enabled_in(&s, &caps));
    assert!(!Setting::Post(PostSetting::Aa).enabled_in(&s, &caps), "FSR 3 does its own anti-aliasing");
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
fn a_dlss_renderer_shows_the_temporal_rows_and_cycles_only_what_it_runs() {
    let dlss = blackbox_gfx::Capabilities::mock_dlss("bevy", blackbox_gfx::GraphicsApi::Vulkan);
    let video = titles(Category::Video, &dlss);
    assert!(video.contains(&text("Upscale Quality")) && video.contains(&text("Ray Tracing")));
    let (mut s, mut c) = (defaults(), Partial::default());
    let mut seen = Vec::new();
    for _ in 0..4 {
        Setting::Upscaler.advance(&mut s, &mut c, true, &dlss);
        seen.push(Setting::Upscaler.data_in(&s, &dlss));
    }
    assert_eq!(
        seen,
        [
            Data::Text("DLSS".into()),
            Data::Text("Off".into()),
            Data::Text("Bilinear".into()),
            Data::Text("FSR 1".into())
        ],
        "FSR 3 and FSR 4 are skipped: the DLSS mock's caps do not list them"
    );
}

#[test]
fn anti_aliasing_cycles_spatial_modes_on_native_and_all_four_on_a_full_one() {
    let aa = Setting::Post(PostSetting::Aa);
    let (mut s, mut c, native) = (defaults(), Partial::default(), test_caps::native());
    aa.advance(&mut s, &mut c, true, &native);
    assert_eq!(s.post_aa, PostAa::Fxaa);
    aa.advance(&mut s, &mut c, true, &native);
    assert_eq!(s.post_aa, PostAa::Smaa);
    aa.advance(&mut s, &mut c, true, &native);
    assert_eq!(s.post_aa, PostAa::Off, "taa is skipped");
    aa.advance(&mut s, &mut c, false, &native);
    assert_eq!(s.post_aa, PostAa::Smaa, "backwards too");
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
    assert_eq!(
        Setting::Renderer(RendererSetting::RayTracing).data_in(&stored, &native),
        Data::Text("Unavailable".into())
    );
    assert!(!aa.offered(&stored, &native) && !Setting::Upscaler.offered(&stored, &native));
    let (mut s, mut c) = (stored, Partial::default());
    aa.advance(&mut s, &mut c, true, &native);
    assert_eq!(s.post_aa, PostAa::Off, "forward from taa wraps to off");
    let mut s = stored;
    aa.advance(&mut s, &mut c, false, &native);
    assert_eq!(s.post_aa, PostAa::Smaa, "backward from taa offers spatial smaa");
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
    s.upscaler = UpscaleMode::Dlss;
    assert_eq!(quality.data_in(&s, &full), Data::Text("Quality".into()));
    for name in ["Balanced", "Performance", "Ultra Performance", "Auto", "DLAA", "Quality"] {
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

#[test]
fn reflections_follow_renderer_support_and_glossy_shading() {
    let native = test_caps::native();
    let unsupported = blackbox_gfx::Capabilities { local_reflections: false, ..native };
    let row = Setting::Graphics(GraphicsSetting::Reflections);
    let mut s = defaults();
    let mut changes = Partial::default();
    s.car_shading = crate::settings::CarShading::Simple;
    assert!(!row.enabled(&s));
    assert!(!row.advance(&mut s, &mut changes, true, &native));
    assert_eq!(changes, Partial::default());
    s.car_shading = crate::settings::CarShading::Glossy;
    assert!(row.enabled(&s));
    assert!(row.advance(&mut s, &mut changes, true, &native));
    assert_eq!(row.data_in(&s, &native), Data::Text("High".into()));
    assert!(!titles(Category::Video, &unsupported).contains(&text("Car Reflections")));
    assert_eq!(row.data_in(&s, &unsupported), Data::Text("Static Sky".into()));
}

#[test]
fn ao_rows_follow_support_and_keep_disabled_preferences() {
    use crate::settings::{AoQuality, Percent};
    let native = test_caps::native();
    let unsupported = blackbox_gfx::Capabilities { ambient_occlusion: false, ..native };
    for title in ["Ambient Occlusion (Experimental)", "AO Intensity", "AO Comparison"] {
        assert!(titles(Category::Video, &native).contains(&text(title)));
        assert!(!titles(Category::Video, &unsupported).contains(&text(title)));
    }
    let mut s = Settings { ao: AoQuality::Off, ao_intensity: Percent(65), ao_compare: true, ..defaults() };
    let before = s;
    let mut changes = Partial::default();
    for setting in [GraphicsSetting::AoIntensity, GraphicsSetting::AoComparison] {
        let row = Setting::Graphics(setting);
        assert!(!row.enabled(&s));
        for forward in [false, true] {
            assert!(!row.advance(&mut s, &mut changes, forward, &native));
        }
    }
    assert_eq!(s, before);
    assert_eq!(changes, Partial::default());
    s.ao = AoQuality::Medium;
    assert!(Setting::Graphics(GraphicsSetting::AoIntensity).advance(&mut s, &mut changes, true, &native));
    assert_eq!(changes.ao_intensity, Some(Percent(75)));
    assert_eq!(
        Setting::Graphics(GraphicsSetting::AmbientOcclusion).data_in(&s, &unsupported),
        Data::Text("Off".into())
    );
}

#[test]
fn the_temporal_upscalers_have_sharpening_frame_generation_and_dlaa_rows() {
    let sharpening = Setting::UpscaleSharpness;
    let frame_generation = Setting::Renderer(RendererSetting::FrameGeneration);
    let quality = Setting::Renderer(RendererSetting::UpscaleQuality);

    // Without a temporal upscaler there is no frame generation row; with one, the order is quality, sharpening,
    // frame generation.
    assert!(!titles(Category::Video, &test_caps::native()).contains(&text("Frame Generation")));
    let full = test_caps::full();
    let video = titles(Category::Video, &full);
    let at = |name: &'static str| video.iter().position(|t| *t == text(name)).unwrap_or_else(|| panic!("no {name}"));
    assert_eq!((at("Sharpening"), at("Frame Generation")), (at("Upscale Quality") + 1, at("Upscale Quality") + 2));

    // FSR 3 sharpens, so the row is live for it as for FSR 1; DLSS has no sharpening pass of its own.
    let mut s = Settings { upscaler: UpscaleMode::Fsr3, ..defaults() };
    assert!(sharpening.enabled_in(&s, &full));
    s.upscaler = UpscaleMode::Dlss;
    assert!(!sharpening.enabled_in(&s, &full));

    // No renderer has frame generation yet: the row is dimmed, reads Unavailable and changes nothing.
    let mut c = Partial::default();
    assert!(!frame_generation.enabled_in(&s, &full));
    assert_eq!(frame_generation.data_in(&s, &full), Data::Text("Unavailable".into()));
    assert!(!frame_generation.advance(&mut s, &mut c, true, &full));
    assert_eq!(c, Partial::default());

    // DLSS at the output size is DLAA.
    s.upscale_quality = UpscaleQuality::Native;
    assert_eq!(quality.data_in(&s, &full), Data::Text("DLAA".into()));
    s.upscaler = UpscaleMode::Fsr3;
    assert_eq!(quality.data_in(&s, &full), Data::Text("Native".into()));
    s.upscaler = UpscaleMode::Dlss;
    s.upscale_quality = UpscaleQuality::Quality;
    assert_eq!(quality.data_in(&s, &full), Data::Text("Quality".into()));
}
