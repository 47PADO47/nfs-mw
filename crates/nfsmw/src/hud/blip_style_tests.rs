//! The abstract map blip: a custom style's condition and animation, and the game's own styles.

use super::blip_style::{BlipLook, BlipStyle, MapBlip, look_of};
use super::blip_styles::{self, CHECKPOINT, COP, Checkpoint, Cop};

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
    MapBlip { style, position: [0.0, 0.0], heading: [1.0, 0.0], active }
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
fn the_default_look_is_a_plain_white_icon() {
    let look = look_of(&Beacon, &blip("beacon", true), 0).unwrap();
    assert_eq!(look.colour, BlipLook::PLAIN.colour);
}

#[test]
fn a_custom_blip_has_a_pool_of_one_icon() {
    assert_eq!(Beacon.icons(), ["MY_BEACON"]);
}

#[test]
fn the_cop_flashes_only_while_active() {
    let cop = blip(COP, false);
    let idle = look_of(&Cop, &cop, 0).unwrap().colour;
    assert_eq!(idle, BlipLook::PLAIN.colour);
    let flashing: Vec<[u8; 4]> = (0..8).map(|t| look_of(&Cop, &blip(COP, true), t).unwrap().colour).collect();
    assert_eq!(flashing[0], [0, 0, 255, 255]);
    assert_eq!(flashing[4], [160, 0, 0, 255]);
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
