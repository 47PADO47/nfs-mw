//! The map's inputs and the open and close flow: what the keys, the mouse and the wheel ask for this frame, and the
//! frontend stage the map runs in (the world is paused behind it, as behind the pause menu).

use bevy_input::ButtonInput;
use bevy_input::keyboard::KeyCode;
use bevy_input::mouse::MouseButton;

use super::{MapInput, Outcome, activate_gps};
use crate::app::Host;
use crate::frontend::flow::{Frontend, Stage};
use crate::frontend::world_map::overlay::WorldMapOverlay;
use crate::hud::{
    AUTO_SHOP, AUTO_SHOP_TINT, CAR_LOT, CAR_LOT_TINT, RACE_CIRCUIT, RACE_DRAG, RACE_KNOCKOUT, RACE_SPEEDTRAP,
    RACE_SPRINT, RACE_TOLLBOOTH, SAFE_HOUSE, SAFE_HOUSE_TINT,
};
use crate::input::{Action, ActionState};
use crate::ui::UiAssets;

/// This frame's map input. `cursor` is the cursor in view units (see [`super::layout::view_of`]), `side` the picture's
/// size in points (a drag of that many points pans a full view).
pub fn input(
    actions: &ActionState,
    keys: &ButtonInput<KeyCode>,
    mouse: &ButtonInput<MouseButton>,
    cursor: Option<[f32; 2]>,
    side: f32,
) -> MapInput {
    // A drag moves the picture with the pointer, so the view moves the other way.
    let drag = [-actions.value(Action::OrbitX) / side, -actions.value(Action::OrbitY) / side];
    MapInput {
        cursor,
        drag,
        zoom_lines: actions.value(Action::Zoom),
        zoom_all: keys.just_pressed(KeyCode::KeyZ),
        next_filter: actions.just_pressed(Action::MenuRight),
        previous_filter: actions.just_pressed(Action::MenuLeft),
        // Enter, or a left click on a blip, asks for the GPS; a second press confirms it.
        accept: actions.just_pressed(Action::MenuAccept) || actions.just_pressed(Action::Click),
        secondary: mouse.just_pressed(MouseButton::Right),
        back: actions.just_pressed(Action::MenuBack) || actions.just_pressed(Action::Map),
    }
}

/// The icon file of a blip style, from the map's own legend (the first screenshot's Shop, Safe House and Car Lot
/// icons are the package's `Minimap_Icon_*` textures).
pub fn icon_name(style: &str) -> &'static str {
    match style {
        AUTO_SHOP => "Minimap_Icon_Chop_Shop.tga",
        CAR_LOT => "Minimap_Icon_Car_Lot.tga",
        SAFE_HOUSE => "Minimap_Icon_Safe_House.tga",
        RACE_SPEEDTRAP => "Minimap_Icon_Speedtrap.tga",
        RACE_CIRCUIT => "Minimap_Icon_Circuit.tga",
        _ => "Minimap_Icon_Event.tga",
    }
}

/// Every icon file [`icon_name`] can return.
pub const ICONS: [&str; 6] = [
    "Minimap_Icon_Chop_Shop.tga",
    "Minimap_Icon_Car_Lot.tga",
    "Minimap_Icon_Safe_House.tga",
    "Minimap_Icon_Speedtrap.tga",
    "Minimap_Icon_Circuit.tga",
    "Minimap_Icon_Event.tga",
];

/// The tint of a blip style: the HUD's own colours for the interiors (`hud/blip_styles.rs`), none for the races.
pub fn tint_of(style: &str) -> Option<[u8; 3]> {
    let rgba = match style {
        AUTO_SHOP => AUTO_SHOP_TINT,
        CAR_LOT => CAR_LOT_TINT,
        SAFE_HOUSE => SAFE_HOUSE_TINT,
        _ => return None,
    };
    Some([rgba[0], rgba[1], rgba[2]])
}

/// The name of a blip style as the list shows it.
pub fn style_name(style: &str) -> &'static str {
    match style {
        AUTO_SHOP => "Auto shop",
        CAR_LOT => "Car lot",
        SAFE_HOUSE => "Safe house",
        RACE_SPRINT => "Sprint",
        RACE_DRAG => "Drag",
        RACE_CIRCUIT => "Circuit",
        RACE_KNOCKOUT => "Knockout",
        RACE_TOLLBOOTH => "Tollbooth",
        RACE_SPEEDTRAP => "Speed trap",
        _ => "Blip",
    }
}

/// A distance in metres as the card and the list show it.
pub fn format_metres(metres: f32) -> String {
    match metres {
        m if m < 1000.0 => format!("{m:.0} m"),
        m => format!("{:.1} km", m / 1000.0),
    }
}

/// Opens the map over the frozen world. The blips and the player are read before the world freezes (a paused scene
/// reports no blips), and stay as they were while it is paused.
pub fn open(fe: &mut Frontend, overlay: &mut WorldMapOverlay, assets: &UiAssets, host: &mut Host) {
    let (blips, player) = match host.scene.hud_state() {
        Some(hud) => (hud.blips.clone(), hud.minimap.as_ref().map(|m| m.position)),
        None => (Vec::new(), None),
    };
    let calibration = match nfsmw_data::minimap::open_city_calibration(&fe.dir) {
        Ok(calibration) => Some(calibration),
        Err(e) => {
            log::warn!("world map: no city calibration, so no blips: {e:#}");
            None
        }
    };
    overlay.open(calibration, assets);
    overlay.refresh(&blips, player);
    log::info!("world map open: {} blips", overlay.blips.len());
    fe.paused.set(true);
    fe.stage = Stage::Map;
}

/// One frame of the open map: the input goes to the session, and its outcome closes the map or sets the GPS.
pub fn step(fe: &mut Frontend, overlay: &mut WorldMapOverlay, input: MapInput) {
    let outcome = overlay.session.frame(&input, &overlay.blips);
    match outcome {
        Outcome::Stay => {}
        Outcome::Close => close(fe, overlay),
        Outcome::ActivateGps(index) => {
            if let Some(blip) = overlay.blips.get(index) {
                activate_gps(blip);
            }
            close(fe, overlay);
        }
    }
}

fn close(fe: &mut Frontend, overlay: &mut WorldMapOverlay) {
    log::info!("world map closed");
    overlay.close();
    fe.resume();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_a_style_can_use_is_in_the_list() {
        for style in [AUTO_SHOP, CAR_LOT, SAFE_HOUSE, RACE_SPRINT, RACE_SPEEDTRAP, RACE_CIRCUIT, RACE_DRAG] {
            assert!(ICONS.contains(&icon_name(style)), "{style}");
        }
    }

    #[test]
    fn the_interiors_keep_the_hud_tints_and_the_races_have_none() {
        assert_eq!(tint_of(AUTO_SHOP), Some([0xff, 0xed, 0x00]));
        assert_eq!(tint_of(CAR_LOT), Some([0x00, 0x00, 0xff]));
        assert_eq!(tint_of(SAFE_HOUSE), Some([0x00, 0xff, 0x00]));
        assert_eq!(tint_of(RACE_SPRINT), None);
    }

    #[test]
    fn distances_read_in_metres_then_kilometres() {
        assert_eq!(format_metres(42.4), "42 m");
        assert_eq!(format_metres(1500.0), "1.5 km");
    }
}
