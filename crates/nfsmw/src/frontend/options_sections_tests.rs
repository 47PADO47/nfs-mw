//! The sections of the Gameplay and Video pages: which rows are under which title, the default section, empty sections, and how
//! the rows mods declare merge in. The rows are checked as data; the selection over them is in `widget_menu_tests.rs`.

use blackbox_ui::menu::nav;

use super::*;
use crate::frontend::mod_options::SectionedRow;
use crate::settings::test_caps;

fn caps() -> Capabilities {
    test_caps::native()
}

fn heading_titles(rows: &[Row]) -> Vec<String> {
    rows.iter().filter(|row| row.setting == Setting::Heading).map(|row| row.title.text()).collect()
}

/// The rows of the section that starts at heading `title` (up to the next heading).
fn under(rows: &[Row], title: &str) -> Vec<Setting> {
    let at =
        rows.iter().position(|row| row.setting == Setting::Heading && row.title.text() == title).expect("the heading");
    rows[at + 1..].iter().take_while(|row| !row.setting.is_heading()).map(|row| row.setting).collect()
}

fn mod_row(index: usize, section: &str, at: usize) -> SectionedRow {
    SectionedRow {
        row: row(Setting::GameplayOption(index), Title::Key("")),
        section: section.into(),
        heading: row(Setting::ModHeading(at), Title::Key("")),
    }
}

#[test]
fn the_default_section_has_no_title_and_is_first_on_every_page() {
    for category in [Category::Gameplay, Category::Video, Category::Audio, Category::Player, Category::Controls] {
        let page = page(category, &caps(), Vec::new());
        assert!(!page[0].setting.is_heading(), "{category:?} opens on rows, not a title");
    }
}

#[test]
fn a_page_without_sections_is_its_rows_as_they_were() {
    for category in [Category::Audio, Category::Player, Category::Controls, Category::Discord, Category::Hud] {
        assert_eq!(page(category, &caps(), Vec::new()), own_rows(category, &caps()));
    }
}

#[test]
fn gameplay_groups_the_experimental_rows_and_the_rider_rows_and_keeps_the_rest_untitled() {
    let page = page(Category::Gameplay, &caps(), Vec::new());
    assert_eq!(heading_titles(&page), ["Experimental", "RIDE 6"]);
    let experimental = under(&page, "Experimental");
    assert_eq!(experimental, [Setting::TurnSignals, Setting::CopHealthBars, Setting::AirControl]);
    let ride6 = under(&page, "RIDE 6");
    assert_eq!(ride6[0], Setting::RiderGear);
    assert!(ride6.iter().all(|setting| matches!(setting, Setting::RiderGear | Setting::Rider(_))));
    assert!(ride6.iter().filter(|setting| matches!(setting, Setting::Rider(_))).count() == 5);
    let first_heading = page.iter().position(|row| row.setting.is_heading()).unwrap();
    let default: Vec<Setting> = page[..first_heading].iter().map(|row| row.setting).collect();
    for ordinary in [Setting::HudLayout, Setting::UiStyle, Setting::Camera, Setting::Language, Setting::Autosave] {
        assert!(default.contains(&ordinary), "{ordinary:?} stays in the default section");
    }
    assert!(!default.contains(&Setting::TurnSignals) && !default.contains(&Setting::RiderGear));
}

#[test]
fn every_row_of_a_page_is_in_exactly_one_section() {
    for category in [Category::Gameplay, Category::Video] {
        let flat = own_rows(category, &caps());
        let mut sectioned: Vec<Row> = page(category, &caps(), Vec::new());
        sectioned.retain(|row| !row.setting.is_heading());
        assert_eq!(sectioned.len(), flat.len(), "{category:?}: nothing lost or doubled");
        for row in &flat {
            assert!(sectioned.contains(row), "{category:?}: {:?} is kept", row.setting);
        }
    }
}

#[test]
fn the_video_page_groups_its_experimental_rows() {
    let page = page(Category::Video, &caps(), Vec::new());
    assert_eq!(heading_titles(&page), ["Experimental"]);
    let experimental = under(&page, "Experimental");
    assert!(experimental.contains(&Setting::Renderer(RendererSetting::RayTracing)));
    assert!(experimental.contains(&Setting::Graphics(GraphicsSetting::AmbientOcclusion)));
    assert!(experimental.contains(&Setting::SpeedTrails) && experimental.contains(&Setting::SpeedTrailStyle));
    let heading_at = page.iter().position(|row| row.setting.is_heading()).unwrap();
    assert!(page[..heading_at].iter().any(|row| row.setting == Setting::WindowMode), "the ordinary rows come first");
    assert!(page[..heading_at].iter().any(|row| row.setting == Setting::Vsync));
}

#[test]
fn headings_are_not_stops_for_the_selection_and_every_row_is() {
    let page = page(Category::Gameplay, &caps(), Vec::new());
    let flags: Vec<_> = page.iter().map(Row::flags).collect();
    let stops = flags.iter().filter(|flags| flags.stops_scroll()).count();
    assert_eq!(stops, page.iter().filter(|row| !row.setting.is_heading()).count());
    assert_eq!(nav::first_stop(&flags), Some(0));
    assert_eq!(flags.len() - stops, 2, "one title each for Experimental and RIDE 6");
}

#[test]
fn mod_rows_without_a_section_join_the_default_section_after_the_games_rows() {
    let page = page(Category::Gameplay, &caps(), vec![mod_row(4, "", 0)]);
    let first_heading = page.iter().position(|row| row.setting.is_heading()).unwrap();
    assert_eq!(page[first_heading - 1].setting, Setting::GameplayOption(4), "the last row of the untitled section");
    assert_eq!(heading_titles(&page), ["Experimental", "RIDE 6"]);
}

#[test]
fn a_mods_section_comes_after_the_games_own_and_groups_that_mods_rows() {
    let mods = vec![
        mod_row(1, "NFS: Most Wanted (2012)", 0),
        mod_row(2, "RIDE 6 tracks", 1),
        mod_row(3, "NFS: Most Wanted (2012)", 0),
    ];
    let page = page(Category::Gameplay, &caps(), mods);
    assert_eq!(heading_titles(&page), ["Experimental", "RIDE 6"], "the mods' titles are read from the mod when drawn");
    let headings: Vec<Setting> = page.iter().map(|row| row.setting).filter(|setting| setting.is_heading()).collect();
    assert_eq!(headings, [Setting::Heading, Setting::Heading, Setting::ModHeading(0), Setting::ModHeading(1)]);
    let at = page.iter().position(|row| row.setting == Setting::ModHeading(0)).unwrap();
    assert_eq!(page[at + 1].setting, Setting::GameplayOption(1));
    assert_eq!(page[at + 2].setting, Setting::GameplayOption(3), "the same title is one section");
    assert_eq!(page[at + 3].setting, Setting::ModHeading(1));
    assert_eq!(page[at + 4].setting, Setting::GameplayOption(2));
    assert_eq!(page.len(), at + 5, "the mods' sections are last");
}

#[test]
fn a_mod_can_put_a_row_under_the_games_experimental_section() {
    let page = page(Category::Gameplay, &caps(), vec![mod_row(7, "Experimental", 0)]);
    assert_eq!(heading_titles(&page), ["Experimental", "RIDE 6"], "no second title");
    let experimental = under(&page, "Experimental");
    assert_eq!(experimental.last(), Some(&Setting::GameplayOption(7)));
}
