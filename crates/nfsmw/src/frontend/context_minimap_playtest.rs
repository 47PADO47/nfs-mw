use super::*;
use crate::settings::{MinimapContext, MinimapMode};

#[test]
fn batch18_main_and_pause_map_rows_apply_independently_and_request_a_save() {
    for (name, pause) in [(screen::OPTIONS, false), (screen::PAUSE_OPTIONS, true)] {
        for context in [MinimapContext::FreeRoam, MinimapContext::Race] {
            let args = Args { pause, category: Category::Gameplay, ..Args::default() };
            let Some(mut h) = Harness::open(name, args) else { return };
            h.wait(1.0);
            let rows = super::super::options::rows(Category::Gameplay, &crate::settings::test_caps::native());
            let row = rows
                .iter()
                .position(|row| row.setting == super::super::options::Setting::ContextMinimap(context))
                .unwrap();
            for _ in 0..row {
                h.press(pad::DOWN);
            }
            let before = h.settings;
            h.press(pad::RIGHT);
            let (expected, unchanged) = match context {
                MinimapContext::FreeRoam => (MinimapMode::Rotating, MinimapContext::Race),
                MinimapContext::Race => (MinimapMode::Off, MinimapContext::FreeRoam),
            };
            assert_eq!(h.settings.minimap_mode(context), expected);
            assert_eq!(h.settings.minimap_mode(unchanged), before.minimap_mode(unchanged));
            assert_eq!(h.changed.minimap, None, "context row does not edit the legacy/global value");
            h.press(pad::BACK);
            h.wait(1.5);
            assert!(h.said(&Command::SaveSettings));
        }
    }
}
