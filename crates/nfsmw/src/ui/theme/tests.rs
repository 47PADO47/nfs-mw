//! Tests for the element catalogue and the registry built from it.

use std::collections::HashSet;

use blackbox_ui::{BuildCx, DrawList, Theme, ThemeId};

use super::custom::{CustomFeedCell, CustomTheme, DriveFeedCell, FullMapCell, RadioFeedCell};
use super::original::{FrameCell, OriginalTheme};
use super::{ElementId, MenuListCell, build_themes, default_skin};
use crate::frontend::world_map::Chrome;

/// The numbers are part of the mod ABI. Pinning them here means a reorder or a renumber fails a test.
const PINNED: &[(ElementId, u32)] = &[
    (ElementId::Tachometer, 1),
    (ElementId::Speedometer, 2),
    (ElementId::GearIndicator, 3),
    (ElementId::Minimap, 4),
    (ElementId::Blips, 5),
    (ElementId::PursuitMeter, 6),
    (ElementId::Nitrous, 7),
    (ElementId::Turbo, 8),
    (ElementId::Speedbreaker, 9),
    (ElementId::RadioCard, 10),
    (ElementId::BustedOverlay, 11),
    (ElementId::PoliceRadar, 12),
    (ElementId::RaceStatus, 13),
    (ElementId::Leaderboard, 14),
    (ElementId::BountyRow, 15),
    (ElementId::Banner, 16),
    (ElementId::Notifications, 17),
    (ElementId::WorldMarkers, 18),
    (ElementId::VehicleCondition, 19),
    (ElementId::MainMenu, 20),
    (ElementId::Categories, 21),
    (ElementId::PauseMenu, 22),
    (ElementId::OptionTable, 23),
    (ElementId::TrackSelect, 24),
    (ElementId::SafeHouseMenu, 25),
    (ElementId::WorldMapChrome, 26),
    (ElementId::Dialog, 27),
    (ElementId::ControlsList, 28),
    (ElementId::Jukebox, 29),
    (ElementId::Credits, 30),
    (ElementId::LanMenu, 31),
    (ElementId::LanHud, 32),
    (ElementId::Blacklist, 40),
    (ElementId::CarLot, 41),
    (ElementId::RapSheet, 42),
    (ElementId::RegionUnlock, 43),
    (ElementId::MemoryCard, 44),
    (ElementId::Sms, 45),
];

#[test]
fn every_slot_id_is_unique() {
    let mut seen = HashSet::new();
    for slot in ElementId::ALL {
        assert!(seen.insert(slot.raw()), "slot id {} is used twice ({})", slot.raw(), slot.name());
    }
    assert_eq!(seen.len(), ElementId::ALL.len());
}

#[test]
fn every_name_is_unique() {
    let names: HashSet<&str> = ElementId::ALL.iter().map(|slot| slot.name()).collect();
    assert_eq!(names.len(), ElementId::ALL.len());
}

#[test]
fn all_lists_exactly_the_pinned_catalogue() {
    assert_eq!(ElementId::ALL.len(), PINNED.len());
    for (slot, id) in PINNED {
        assert!(ElementId::ALL.contains(slot), "{} is missing from ALL", slot.name());
        assert_eq!(slot.raw(), *id, "{} changed number", slot.name());
    }
}

#[test]
fn slot_id_wraps_the_explicit_number() {
    for slot in ElementId::ALL {
        assert_eq!(slot.slot().raw(), slot.raw());
    }
}

#[test]
fn name_is_the_variant_name() {
    assert_eq!(ElementId::Tachometer.name(), "Tachometer");
    assert_eq!(ElementId::MemoryCard.name(), "MemoryCard");
}

#[test]
fn the_custom_hud_slots_are_hud_slots_not_menus() {
    let custom_hud = [
        ElementId::RaceStatus,
        ElementId::Leaderboard,
        ElementId::BountyRow,
        ElementId::Banner,
        ElementId::Notifications,
        ElementId::WorldMarkers,
        ElementId::VehicleCondition,
    ];
    for slot in custom_hud {
        assert!(!slot.is_menu(), "{} is classed as a menu", slot.name());
        assert!((13..20).contains(&slot.raw()), "{} is outside the HUD block", slot.name());
    }
    // The first menu number is still a menu.
    assert!(ElementId::MainMenu.is_menu());
}

#[test]
fn theme_ids_are_original_and_custom() {
    assert_eq!(OriginalTheme::new(FrameCell::new()).id(), ThemeId::Original);
    assert_eq!(
        CustomTheme::new(RadioFeedCell::default(), CustomFeedCell::default(), FullMapCell::default()).id(),
        ThemeId::Custom
    );
}

#[test]
fn registry_has_original_then_custom_and_remastered_with_original_active() {
    let themes = build_themes(
        FrameCell::new(),
        RadioFeedCell::new(),
        DriveFeedCell::new(),
        CustomFeedCell::new(),
        FullMapCell::new(),
        MenuListCell::new(),
        Default::default(),
    );
    let ids: Vec<ThemeId> = themes.ids().collect();
    assert_eq!(ids, vec![ThemeId::Original, ThemeId::Custom, ThemeId::Remastered]);
    assert_eq!(themes.active(), ThemeId::Original);
}

#[test]
fn custom_can_be_made_active_and_resolves_every_slot() {
    let mut themes = build_themes(
        FrameCell::new(),
        RadioFeedCell::new(),
        DriveFeedCell::new(),
        CustomFeedCell::new(),
        FullMapCell::new(),
        MenuListCell::new(),
        Default::default(),
    );
    themes.set_active(ThemeId::Custom).expect("Custom is registered");
    let skin = default_skin();
    for slot in ElementId::ALL {
        // Custom fills the radio card and the drive slots, and every other slot falls back to Original: the HUD and
        // menu slots get their adapters, which draw nothing from an empty frame cell. The radio card draws nothing from
        // an empty feed. The check is that resolution and build run for every slot without panicking.
        let component = themes.resolve(slot.slot(), &BuildCx::new(&skin));
        let mut out = DrawList::new();
        component.build(&mut out);
    }
}

#[test]
fn ui_style_maps_onto_its_theme_id() {
    use crate::settings::UiStyle;
    assert_eq!(super::theme_of(UiStyle::Original), ThemeId::Original);
    assert_eq!(super::theme_of(UiStyle::Custom), ThemeId::Custom);
    assert_eq!(super::theme_of(UiStyle::Remastered), ThemeId::Remastered);
}

#[test]
fn custom_draws_the_whole_world_map_and_original_does_not() {
    use crate::settings::UiStyle;
    let mut themes = super::UiThemes::build();

    // Original (the default) leaves the FEng map screen as it is.
    assert_eq!(themes.world_map_chrome(), Chrome::Original, "Original does not dress the map");

    // Custom provides the slot and draws the whole map, as Remastered does: that is the flag the plugin reads.
    themes.select(UiStyle::Custom);
    assert_eq!(themes.world_map_chrome(), Chrome::Drawn, "Custom draws the map");
    assert!(
        super::slot_layer(&themes, ElementId::WorldMapChrome, 0.0).nodes.is_empty(),
        "no map is drawn until one is open"
    );

    // Back to Original: the map is left alone again.
    themes.select(UiStyle::Original);
    assert_eq!(themes.world_map_chrome(), Chrome::Original);
}

#[test]
fn remastered_draws_the_whole_world_map() {
    use crate::settings::UiStyle;
    let mut themes = super::UiThemes::build();
    themes.select(UiStyle::Remastered);
    assert_eq!(themes.world_map_chrome(), Chrome::Drawn);
    assert!(
        super::slot_layer(&themes, ElementId::WorldMapChrome, 0.0).nodes.is_empty(),
        "no map is drawn until one is open"
    );
}

#[test]
fn select_follows_the_style_and_keeps_original_for_an_unregistered_theme() {
    use crate::settings::UiStyle;
    let mut themes = super::UiThemes::build();
    assert_eq!(themes.themes.active(), ThemeId::Original);
    themes.select(UiStyle::Custom);
    assert_eq!(themes.themes.active(), ThemeId::Custom);
    themes.select(UiStyle::Remastered);
    assert_eq!(themes.themes.active(), ThemeId::Remastered);
    themes.select(UiStyle::Custom);
    themes.select(UiStyle::Original);
    assert_eq!(themes.themes.active(), ThemeId::Original);
}

#[test]
fn sync_system_selects_the_theme_of_the_current_settings() {
    use crate::settings::{Partial, Settings, UiStyle};
    use bevy_ecs::system::RunSystemOnce;
    let mut world = bevy_ecs::world::World::new();
    world.insert_resource(Settings::from(Partial { ui_style: Some(UiStyle::Custom), ..Partial::default() }));
    world.insert_resource(super::UiThemes::build());
    world.run_system_once(super::sync_active_theme).expect("the sync system runs");
    assert_eq!(world.resource::<super::UiThemes>().themes.active(), ThemeId::Custom);
    world.resource_mut::<Settings>().ui_style = UiStyle::Original;
    world.run_system_once(super::sync_active_theme).expect("the sync system runs");
    assert_eq!(world.resource::<super::UiThemes>().themes.active(), ThemeId::Original);
}

#[test]
fn the_active_theme_tokens_reach_the_skin_the_components_read() {
    use crate::settings::{UiStyle, UiTheme};
    use blackbox_ui::token_keys;

    use super::custom::palette;

    let mut themes = super::UiThemes::build();
    themes.select(UiStyle::Custom);
    assert_eq!(themes.skin.colour(token_keys::BACKGROUND), Some(palette::bevy().background));
    themes.set_ui_theme(UiTheme::Remastered);
    assert_eq!(themes.skin.colour(token_keys::BACKGROUND), Some(palette::remastered().background));
    assert_eq!(themes.skin.metric(token_keys::RADIUS), Some(palette::remastered().radius));
    themes.select(UiStyle::Original);
    assert_eq!(
        themes.skin.colour(token_keys::BACKGROUND),
        Some(blackbox_ui::Tokens::default().background),
        "the floor merges the neutral tokens"
    );
}

#[test]
fn the_custom_tokens_follow_the_look_and_take_their_accent_from_the_theme() {
    use crate::settings::hud::HudPalette;
    use crate::settings::{UiStyle, UiTheme};

    use super::custom::palette;
    use super::custom::tokens::{Colours, CustomLook, keys};

    let mut themes = super::UiThemes::build();
    themes.select(UiStyle::Custom);
    assert_eq!(themes.skin.colour(keys::TEXT), Some(Colours::BRIEF.text));
    assert_eq!(
        themes.skin.colour(keys::ACCENT),
        Some(super::custom::tokens::SIGNAL),
        "the Bevy theme takes the signal yellow"
    );
    themes.set_ui_theme(UiTheme::Remastered);
    assert_eq!(themes.skin.colour(keys::ACCENT), Some(palette::remastered().secondary), "the Remastered gold");
    themes.set_custom_look(CustomLook { palette: HudPalette::Deuteranopia, high_contrast: true });
    assert_eq!(themes.skin.colour(keys::TEXT), Some([255, 255, 255, 255]));
    assert!(themes.skin.colour(keys::OUTLINE).is_some(), "high contrast outlines the plates");
    themes.set_custom_look(CustomLook::default());
    assert_eq!(themes.skin.colour(keys::OUTLINE), None, "the outline goes with high contrast");
}

#[test]
fn the_themes_colour_roles_reach_the_skin_and_a_mod_theme_gets_derived_ones() {
    use crate::settings::hud::HudPalette;
    use crate::settings::{UiStyle, UiTheme};
    use blackbox_ui::{Tokens, token_keys};

    use super::custom::palette;
    use super::custom::tokens::{Colours, CustomLook};

    let mut themes = super::UiThemes::build();
    themes.select(UiStyle::Custom);
    assert_eq!(themes.skin.colour(token_keys::GUIDE), Some(palette::bevy().resolved_roles().guide));
    assert_eq!(themes.skin.colour(token_keys::SCRIM), Some(palette::SCRIM));
    themes.set_ui_theme(UiTheme::Remastered);
    assert_eq!(themes.skin.colour(token_keys::GUIDE), Some(palette::remastered().resolved_roles().guide));
    assert_ne!(
        themes.skin.colour(token_keys::GUIDE),
        Some(palette::bevy().resolved_roles().guide),
        "switching the theme changes the route's colour"
    );
    // The player's colour-vision palette wins over the theme for the meaning hues.
    let look = CustomLook { palette: HudPalette::Deuteranopia, high_contrast: false };
    themes.set_custom_look(look);
    assert_eq!(themes.skin.colour(token_keys::DANGER), Some(Colours::for_look(look).danger));
    themes.set_custom_look(CustomLook { palette: HudPalette::Default, high_contrast: true });
    assert_eq!(themes.skin.colour(token_keys::LINE), Some([255, 255, 255, 255]), "high contrast whitens the linework");
    // The floor and a theme with no roles of its own derive them from the five core colours.
    themes.set_custom_look(CustomLook::default());
    themes.select(UiStyle::Original);
    let neutral = Tokens::default();
    assert_eq!(themes.skin.colour(token_keys::LINE), Some(neutral.primary));
    assert_eq!(themes.skin.colour(token_keys::GUIDE), Some(neutral.secondary));
}
