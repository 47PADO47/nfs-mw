use super::logic::Category;
use super::options::{Data, Setting, rows};
use crate::settings::{Partial, Settings};

fn settings(vsync: bool) -> Settings {
    Settings { vsync, ..Settings::from(Partial::default()) }
}

#[test]
fn the_frame_limit_is_disabled_while_vsync_is_on() {
    let on = settings(true);
    let off = settings(false);
    assert!(!Setting::MaxFps.enabled(&on));
    assert!(Setting::MaxFps.enabled(&off));
    assert!(
        rows(Category::Video, &crate::settings::test_caps::native()).iter().any(|row| row.setting == Setting::MaxFps),
        "still listed and selectable"
    );
}

#[test]
fn a_disabled_row_ignores_left_and_right_and_keeps_its_value() {
    let mut s = settings(true);
    let mut changes = Partial::default();
    let before = Setting::MaxFps.data(&s);
    assert!(!Setting::MaxFps.step(&mut s, &mut changes, true));
    assert!(!Setting::MaxFps.step(&mut s, &mut changes, false));
    assert_eq!(Setting::MaxFps.data(&s), before);
    assert_eq!(changes, Partial::default(), "nothing is recorded for the config file");
}

#[test]
fn turning_vsync_off_enables_the_frame_limit_again() {
    let mut s = settings(true);
    let mut changes = Partial::default();
    assert!(Setting::Vsync.step(&mut s, &mut changes, true));
    assert!(!s.vsync);
    assert!(Setting::MaxFps.enabled(&s));
    assert!(Setting::MaxFps.step(&mut s, &mut changes, true));
    assert_eq!(Setting::MaxFps.data(&s), Data::Text("30 FPS".into()));
    assert_eq!(changes.max_fps.map(|m| m.to_string()), Some("30".to_owned()));
}

#[test]
fn the_other_rows_are_always_enabled() {
    for s in [settings(true), settings(false)] {
        for category in [Category::Audio, Category::Video, Category::Gameplay, Category::Player, Category::Discord] {
            for row in rows(category, &crate::settings::test_caps::native()) {
                if !matches!(row.setting, Setting::MaxFps | Setting::Renderer(_) | Setting::Graphics(_)) {
                    assert!(row.setting.enabled(&s), "{:?}", row.setting);
                }
            }
        }
    }
}

#[test]
fn effect_dependencies_ignore_edits_and_preserve_preferences() {
    use crate::settings::{RenderScale, UpscaleMode};
    let caps = crate::settings::test_caps::native();
    let mut s = Settings { upscaler: UpscaleMode::Off, tire_smoke: false, ..settings(false) };
    let before = s;
    let mut changed = Partial::default();
    for row in [Setting::RenderScale, Setting::UpscaleSharpness, Setting::SmokeQuality] {
        assert!(!row.enabled_in(&s, &caps));
        for forward in [false, true] {
            assert!(!row.advance(&mut s, &mut changed, forward, &caps));
        }
    }
    assert_eq!(s, before);
    assert_eq!(changed, Partial::default());
    s.upscaler = UpscaleMode::Fsr1;
    s.tire_smoke = true;
    assert!(Setting::RenderScale.advance(&mut s, &mut changed, true, &caps));
    assert_ne!(s.render_scale, RenderScale::default());
    assert!(Setting::UpscaleSharpness.enabled_in(&s, &caps));
    assert!(Setting::SmokeQuality.enabled_in(&s, &caps));
}

#[test]
fn temporal_upscaling_owns_resolution_and_anti_aliasing_on_a_supported_renderer() {
    use super::post_options::PostSetting;
    use super::renderer_options::RendererSetting;
    use crate::settings::UpscaleMode;
    let full = crate::settings::test_caps::full();
    let native = crate::settings::test_caps::native();
    let mut s = Settings { upscaler: UpscaleMode::Dlss, ..settings(false) };
    let mut changes = Partial::default();
    assert!(!Setting::RenderScale.enabled_in(&s, &native));
    for row in [Setting::Post(PostSetting::Aa), Setting::UpscaleSharpness] {
        assert!(!row.enabled_in(&s, &full));
        assert!(!row.advance(&mut s, &mut changes, true, &full));
        assert!(row.enabled_in(&s, &native), "native falls back to spatial FSR 1");
    }
    assert!(Setting::Renderer(RendererSetting::UpscaleQuality).enabled_in(&s, &full));
    assert_eq!(changes, Partial::default());
    assert!(Setting::Post(PostSetting::Aa).advance(&mut s, &mut changes, true, &native));
    assert!(Setting::UpscaleSharpness.advance(&mut s, &mut changes, true, &native));
    assert_eq!(changes.upscale_sharpness, Some(crate::settings::Percent(0)), "High is followed by Off");
    assert_eq!(s.upscaler, UpscaleMode::Dlss, "a separate preference stays stored");
}

#[test]
fn the_discord_category_holds_the_two_toggles_and_the_time_played_dims_while_discord_is_off() {
    let caps = crate::settings::test_caps::native();
    let toggles: Vec<Setting> = rows(Category::Discord, &caps).iter().map(|row| row.setting).collect();
    assert_eq!(toggles, [Setting::Discord, Setting::DiscordTimePlayed]);
    let gameplay = rows(Category::Gameplay, &caps);
    assert!(!gameplay.iter().any(|row| matches!(row.setting, Setting::Discord | Setting::DiscordTimePlayed)));
    assert!(!Setting::DiscordTimePlayed.enabled(&Settings { discord: false, ..settings(true) }));
    assert!(Setting::DiscordTimePlayed.enabled(&Settings { discord: true, ..settings(true) }));
}
