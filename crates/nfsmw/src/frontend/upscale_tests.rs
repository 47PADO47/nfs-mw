use super::logic::Category;
use super::options::{Control, Data, Setting, Title, rows};
use crate::settings::{Partial, Percent, RenderScale, Settings, UpscaleMode};

fn defaults() -> Settings {
    Settings::from(Partial::default())
}

#[test]
fn the_video_screen_lists_the_upscaling_rows_after_the_existing_ones() {
    let rows = rows(Category::Video);
    let titles: Vec<_> = rows.iter().map(|r| r.title).collect();
    let first = titles.iter().position(|t| *t == Title::Text("Render Scale")).expect("render scale row");
    assert_eq!(
        titles[first - 1..first + 3],
        [
            Title::Text("Exhaust Flames"),
            Title::Text("Render Scale"),
            Title::Text("Upscaler"),
            Title::Text("Upscale Sharpness")
        ],
        "appended after the effects rows, so the earlier rows keep their places"
    );
}

#[test]
fn render_scale_steps_through_the_presets_and_wraps() {
    let (mut s, mut c) = (defaults(), Partial::default());
    assert_eq!(Setting::RenderScale.data(&s), Data::Text("100%".into()));
    assert_eq!(Setting::RenderScale.control(&s), Control::Toggle);
    for expected in [125, 150, 200, 50, 59] {
        assert!(Setting::RenderScale.step(&mut s, &mut c, true));
        assert_eq!(s.render_scale.percent(), expected);
        assert_eq!(c.render_scale, Some(s.render_scale));
    }
    Setting::RenderScale.step(&mut s, &mut c, false);
    assert_eq!(s.render_scale.percent(), 50);
    Setting::RenderScale.step(&mut s, &mut c, false);
    assert_eq!(s.render_scale.percent(), 200, "left from the smallest wraps to the largest");
    assert_eq!(Setting::RenderScale.data(&s), Data::Text("200%".into()));
}

#[test]
fn a_scale_between_presets_moves_to_the_next_preset_in_that_direction() {
    let mut s = Settings { render_scale: RenderScale::new(90).unwrap(), ..defaults() };
    let mut c = Partial::default();
    Setting::RenderScale.step(&mut s, &mut c, true);
    assert_eq!(s.render_scale.percent(), 100);
    s.render_scale = RenderScale::new(90).unwrap();
    Setting::RenderScale.step(&mut s, &mut c, false);
    assert_eq!(s.render_scale.percent(), 85);
}

#[test]
fn the_upscaler_row_cycles_both_ways_and_records_only_itself() {
    let (mut s, mut c) = (defaults(), Partial::default());
    assert_eq!(Setting::Upscaler.data(&s), Data::Text("FSR 1".into()));
    for name in ["FSR 3", "FSR 4", "DLSS", "Off", "Bilinear"] {
        Setting::Upscaler.step(&mut s, &mut c, true);
        assert_eq!(Setting::Upscaler.data(&s), Data::Text(name.into()));
    }
    assert_eq!(s.upscaler, UpscaleMode::Bilinear, "right from the last wraps to the first");
    assert_eq!(c, Partial { upscaler: Some(UpscaleMode::Bilinear), ..Partial::default() });
    Setting::Upscaler.step(&mut s, &mut c, false);
    Setting::Upscaler.step(&mut s, &mut c, false);
    Setting::Upscaler.step(&mut s, &mut c, false);
    Setting::Upscaler.step(&mut s, &mut c, false);
    Setting::Upscaler.step(&mut s, &mut c, false);
    Setting::Upscaler.step(&mut s, &mut c, false);
    assert_eq!(s.upscaler, UpscaleMode::Bilinear, "six values make a full turn");
    Setting::Upscaler.step(&mut s, &mut c, false);
    assert_eq!(s.upscaler, UpscaleMode::Off);
}

#[test]
fn sharpness_is_a_slider_in_steps_of_ten() {
    let (mut s, mut c) = (defaults(), Partial::default());
    assert_eq!(Setting::UpscaleSharpness.control(&s), Control::Slider(80));
    assert!(Setting::UpscaleSharpness.step(&mut s, &mut c, true));
    assert!(Setting::UpscaleSharpness.step(&mut s, &mut c, true));
    assert!(!Setting::UpscaleSharpness.step(&mut s, &mut c, true), "100 is the end");
    assert_eq!(c.upscale_sharpness, Some(Percent(100)));
    for _ in 0..12 {
        Setting::UpscaleSharpness.step(&mut s, &mut c, false);
    }
    assert_eq!(s.upscale_sharpness, Percent(0));
}
