//! The abstract map blip: a custom style's condition and animation, and the game's own styles.

use super::blip_style::{BlipLook, BlipStyle, MapBlip, Visibility, colour_to_apply, drawn_in, look_of};
use super::blip_styles::{self, CHECKPOINT, COP, Checkpoint, Cop};
use super::blips::nearest_blip;

/// A custom blip: shows only while `active`, and spins a quarter turn each tick.
struct Beacon;

impl BlipStyle for Beacon {
    fn name(&self) -> &'static str {
        "beacon"
    }

    fn icons(&self) -> Vec<String> {
        vec!["MY_BEACON".to_string()]
    }

    fn shows(&self, blip: &MapBlip) -> bool {
        blip.active
    }

    fn look(&self, _blip: &MapBlip, tick: u32) -> BlipLook {
        BlipLook { spin: 90.0 * tick as f32, ..BlipLook::PLAIN }
    }
}

fn blip(style: &'static str, active: bool) -> MapBlip {
    MapBlip { style, position: [0.0, 0.0], active, heading: None }
}

#[test]
fn a_condition_hides_the_blip_and_takes_no_look() {
    assert_eq!(look_of(&Beacon, &blip("beacon", false), 0), None);
    assert!(look_of(&Beacon, &blip("beacon", true), 0).is_some());
}

#[test]
fn an_animation_changes_the_look_each_tick() {
    let on = blip("beacon", true);
    let spins: Vec<f32> = (0..4).map(|t| look_of(&Beacon, &on, t).unwrap().spin).collect();
    assert_eq!(spins, [0.0, 90.0, 180.0, 270.0]);
}

#[test]
fn the_default_look_keeps_the_package_colour() {
    let look = look_of(&Beacon, &blip("beacon", true), 0).unwrap();
    assert_eq!(look.colour, None);
}

#[test]
fn a_custom_blip_has_a_pool_of_one_icon() {
    assert_eq!(Beacon.icons(), ["MY_BEACON"]);
}

#[test]
fn a_cop_is_gated_to_the_view_whether_or_not_it_is_pursuing() {
    assert_eq!(Cop.visibility(&blip(COP, false)), Visibility::InZone);
    assert_eq!(Cop.visibility(&blip(COP, true)), Visibility::InZone);
}

#[test]
fn the_cop_flashes_only_while_active() {
    let cop = blip(COP, false);
    let idle = look_of(&Cop, &cop, 0).unwrap().colour;
    assert_eq!(idle, Some([255, 255, 255, 255]));
    let flashing: Vec<Option<[u8; 4]>> = (0..8).map(|t| look_of(&Cop, &blip(COP, true), t).unwrap().colour).collect();
    assert_eq!(flashing[0], Some([0, 0, 255, 255]));
    assert_eq!(flashing[2], Some([160, 0, 0, 255]));
}

#[test]
fn the_checkpoint_shows_only_when_active() {
    assert_eq!(look_of(&Checkpoint, &blip(CHECKPOINT, false), 0), None);
    assert!(look_of(&Checkpoint, &blip(CHECKPOINT, true), 0).is_some());
}

#[test]
fn the_builtin_styles_have_distinct_names_and_pools() {
    let styles = blip_styles::builtin();
    let mut names: Vec<&str> = styles.iter().map(|s| s.name()).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), styles.len());
    let racer = styles.iter().find(|s| s.name() == blip_styles::RACER).unwrap();
    assert_eq!(racer.icons().len(), 8);
}

#[test]
fn the_map_icon_pools_are_the_package_objects() {
    let styles = blip_styles::map_icon_styles();
    let mut names: Vec<&str> = styles.iter().map(|s| s.name()).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), styles.len());
    let pool = |name: &str| styles.iter().find(|s| s.name() == name).unwrap().icons();
    assert_eq!(pool(blip_styles::RACE_SPRINT)[0], "MMICON_SPRINT_0");
    assert_eq!(pool(blip_styles::RACE_SPEEDTRAP).last().unwrap(), "MMICON_SPEEDTRAP_7");
    assert_eq!(
        pool(blip_styles::AUTO_SHOP),
        ["SHOP_0", "SHOP_1", "SHOP_2", "SHOP_3", "SHOP_4", "SHOP_5", "SHOP_6", "SHOP_7"]
    );
    assert_eq!(
        pool(blip_styles::SAFE_HOUSE),
        ["SAFEHOUSEINDICATOR_0", "SAFEHOUSEINDICATOR_1", "SAFEHOUSEINDICATOR_2", "SAFEHOUSEINDICATOR_3"]
    );
}

#[test]
fn a_look_colour_wins_and_a_package_colour_is_kept_unless_plain_white() {
    const YELLOW: [u8; 4] = [0xff, 0xed, 0x00, 0xff];
    const RED: [u8; 4] = [0xa0, 0, 0, 0xff];
    let tinted = BlipLook { colour: Some(RED), ..BlipLook::PLAIN };
    assert_eq!(colour_to_apply(&tinted, Some(YELLOW), [255, 255, 255, 255]), Some(RED));
    assert_eq!(colour_to_apply(&BlipLook::PLAIN, Some(YELLOW), [255, 255, 255, 255]), Some(YELLOW));
    assert_eq!(colour_to_apply(&BlipLook::PLAIN, Some(YELLOW), [0, 255, 0, 255]), None);
    assert_eq!(colour_to_apply(&BlipLook::PLAIN, None, [255, 255, 255, 255]), None);
}

#[test]
fn a_zone_gated_blip_is_drawn_only_inside_the_circle() {
    assert!(drawn_in(Visibility::InZone, true));
    assert!(!drawn_in(Visibility::InZone, false));
    assert!(drawn_in(Visibility::Always, false));
}

#[test]
fn nearest_only_picks_the_closest_blip_of_the_style() {
    let at = |style: &'static str, x: f32| MapBlip { style, position: [x, 0.0], active: false, heading: None };
    let blips = [at(blip_styles::CAR_LOT, 50.0), at(blip_styles::AUTO_SHOP, 1.0), at(blip_styles::CAR_LOT, 8.0)];
    assert_eq!(nearest_blip(&blips, blip_styles::CAR_LOT, [0.0, 0.0]), Some(2));
    assert_eq!(nearest_blip(&blips, blip_styles::AUTO_SHOP, [0.0, 0.0]), Some(1));
    assert_eq!(nearest_blip(&blips, blip_styles::SAFE_HOUSE, [0.0, 0.0]), None);
}

#[test]
fn the_races_are_zone_gated_and_the_interiors_always_eligible() {
    let styles = blip_styles::map_icon_styles();
    let find = |name: &str| styles.iter().find(|s| s.name() == name).unwrap();
    let idle = |style: &'static str| blip(style, false);
    assert_eq!(find(blip_styles::RACE_SPRINT).visibility(&idle(blip_styles::RACE_SPRINT)), Visibility::InZone);
    assert_eq!(find(blip_styles::RACE_SPRINT).fallback_colour(), None);
    assert_eq!(find(blip_styles::AUTO_SHOP).visibility(&idle(blip_styles::AUTO_SHOP)), Visibility::Always);
    assert!(find(blip_styles::AUTO_SHOP).nearest_only());
    assert!(find(blip_styles::CAR_LOT).nearest_only());
    assert_eq!(find(blip_styles::CAR_LOT).fallback_colour(), Some(blip_styles::CAR_LOT_TINT));
    assert!(!find(blip_styles::SAFE_HOUSE).nearest_only());
    assert_eq!(find(blip_styles::SAFE_HOUSE).visibility(&idle(blip_styles::SAFE_HOUSE)), Visibility::Always);
    assert_eq!(find(blip_styles::SAFE_HOUSE).fallback_colour(), Some(blip_styles::SAFE_HOUSE_TINT));
    assert_eq!(find(blip_styles::CAR_LOT).icons(), ["CAR_LOT_0", "CAR_LOT_1", "CAR_LOT_2", "CAR_LOT_3"]);
}
