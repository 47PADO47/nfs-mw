//! The map's inputs and the open and close flow: what the keys, the mouse and the wheel ask for this frame, and the
//! frontend stage the map runs in (the world is paused behind it, as behind the pause menu). The map's screen is
//! `WorldMapMain.fng` (`screen/`), opened as a screen over the frozen world.

use bevy_input::ButtonInput;
use bevy_input::keyboard::KeyCode;
use bevy_input::mouse::MouseButton;
use nfsmw_career::regions::MapVariant;

use super::screen::WorldMapLogic;
use super::{Chrome, MapInput, Outcome};
use crate::app::Host;
use crate::frontend::confirm::{Confirm, ConfirmLogic};
use crate::frontend::flow::{Frontend, Stage};
use crate::frontend::ids::screen::DIALOG;
use crate::frontend::logic::{Command, Env};
use crate::frontend::world_map::overlay::WorldMapOverlay;
use crate::hud::{
    AUTO_SHOP, AUTO_SHOP_TINT, CAR_LOT, CAR_LOT_TINT, RACE_CIRCUIT, RACE_DRAG, RACE_KNOCKOUT, RACE_SPEEDTRAP,
    RACE_SPRINT, RACE_TOLLBOOTH, ROADBLOCK, SAFE_HOUSE, SAFE_HOUSE_TINT,
};
use crate::input::{Action, ActionState, Bindings, InputDevice};
use crate::ui::Text;
use crate::ui::prompts::{self, Prompt};
use crate::ui::theme::MenuHint;

/// This frame's map input. `cursor` is the cursor in view units (see [`super::layout::Picture::view_of`]), `side` the
/// picture's size in points (a drag of that many points pans a full view). `script_key` is a key a scripted run pressed
/// this frame (`--ui-script "key L"`), which counts as that key being pressed.
pub fn input(
    actions: &ActionState,
    keys: &ButtonInput<KeyCode>,
    script_key: Option<KeyCode>,
    mouse: &ButtonInput<MouseButton>,
    cursor: Option<[f32; 2]>,
    side: f32,
    chrome: Chrome,
) -> MapInput {
    let pressed = |key: KeyCode| keys.just_pressed(key) || script_key == Some(key);
    // A drag moves the picture with the pointer, so the view moves the other way.
    let drag = [-actions.value(Action::OrbitX) / side, -actions.value(Action::OrbitY) / side];
    MapInput {
        cursor,
        drag,
        zoom_lines: actions.value(Action::Zoom),
        zoom_all: pressed(KeyCode::KeyZ),
        toggle_legend: pressed(KeyCode::KeyL),
        full_screen: chrome == Chrome::Drawn,
        next_filter: actions.just_pressed(Action::MenuRight),
        previous_filter: actions.just_pressed(Action::MenuLeft),
        // A left click focuses the blip under the cursor (or unfocuses); Enter opens the GPS prompt on the focused blip.
        click: actions.just_pressed(Action::Click),
        accept: actions.just_pressed(Action::MenuAccept),
        secondary: mouse.just_pressed(MouseButton::Right),
        back: actions.just_pressed(Action::MenuBack) || actions.just_pressed(Action::Map),
    }
}

/// The footer hints of the themed full-screen map: back, the legend key, and the tab keys, with the player's bindings.
pub fn hints(bindings: &Bindings, device: InputDevice) -> Vec<MenuHint> {
    vec![
        MenuHint {
            keys: vec![prompts::for_action(bindings, Action::MenuBack, device)],
            label: "Back",
            action: Some(Action::MenuBack),
        },
        MenuHint { keys: vec![Prompt::Key("L".into())], label: "Legend", action: None },
        MenuHint {
            keys: prompts::pair(bindings, Action::MenuLeft, Action::MenuRight, device),
            label: "Filter",
            action: None,
        },
    ]
}

/// Every icon file [`icon_name`] can return, so the themed legend can register them as textures up front.
pub const MAP_ICON_FILES: [&str; 7] = [
    "Minimap_Icon_Car.tga",
    "Minimap_Icon_Chop_Shop.tga",
    "Minimap_Icon_Car_Lot.tga",
    "Minimap_Icon_Safe_House.tga",
    "Minimap_Icon_Speedtrap.tga",
    "Minimap_Icon_Circuit.tga",
    "Minimap_Icon_Event.tga",
];

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

/// The icon colour of a blip style as the themed legend draws it: the HUD interior tints, and amber for the races.
pub fn blip_rgba(style: &str) -> [u8; 4] {
    // The roadblock's icon is tinted `0xffffed00` by the original's icon builder (`docs/specs/hud-minimap.md` section 7).
    if style == ROADBLOCK {
        return [0xff, 0xed, 0x00, 0xff];
    }
    match tint_of(style) {
        Some([r, g, b]) => [r, g, b, 255],
        None => [255, 196, 64, 255],
    }
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
        ROADBLOCK => "Roadblock",
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
pub fn open(fe: &mut Frontend, overlay: &mut WorldMapOverlay, host: &mut Host, env: &mut Env) {
    let (blips, player) = match host.scene.hud_state() {
        Some(hud) => (hud.blips.clone(), hud.minimap),
        None => (Vec::new(), None),
    };
    let calibration = match nfsmw_data::minimap::open_city_calibration(&fe.dir) {
        Ok(calibration) => Some(calibration),
        Err(e) => {
            log::warn!("world map: no city calibration, so no blips: {e:#}");
            None
        }
    };
    overlay.open(calibration);
    overlay.refresh(&blips, player.as_ref());
    log::info!("world map open: {} blips", overlay.blips.len());
    // The career stage picks the map; outside career mode (no career handle) it is the full map.
    let variant = fe.career.with(|c| MapVariant::of_stage(c.stage())).unwrap_or(MapVariant::Full);
    overlay.texture = variant.world_map_texture();
    fe.open_map(Box::new(WorldMapLogic { variant, last_attached: None }), host, env);
}

/// One frame of the open map: the input goes to the session, and its outcome closes the map, sets the GPS, or draws
/// the map's screen with the session's new state.
pub fn step(
    fe: &mut Frontend,
    overlay: &mut WorldMapOverlay,
    input: MapInput,
    dt: f32,
    chrome: Chrome,
    host: &mut Host,
    env: &mut Env,
) {
    overlay.tick(dt);
    let outcome = overlay.session.frame(&input, &overlay.blips);
    match outcome {
        Outcome::Stay => draw(fe, overlay, dt, chrome, host, env),
        Outcome::Close => close(fe, overlay),
        Outcome::AskGps(index) => ask_gps(fe, overlay, index, dt, chrome, host, env),
    }
}

/// Opens the game's own `Dialog.fng` Yes/No confirm over the map. Yes sets the route to the focused blip, then closes
/// the map and goes on driving; No pops the dialog and the map continues with the blip still focused.
fn ask_gps(
    fe: &mut Frontend,
    overlay: &WorldMapOverlay,
    index: usize,
    dt: f32,
    chrome: Chrome,
    host: &mut Host,
    env: &mut Env,
) {
    let Some(blip) = overlay.blips.get(index) else {
        return draw(fe, overlay, dt, chrome, host, env);
    };
    let confirm = Confirm::new(Text::Custom("Activate GPS?".into()), Command::SetWaypoint(blip.world));
    let commands = fe.screens.open_with(DIALOG, Box::new(ConfirmLogic::new(confirm)), false, env);
    fe.apply(commands, host, env);
    draw(fe, overlay, dt, chrome, host, env);
}

/// Gives the map's screen this frame's state and runs the screens one step (the package's animations and the logic
/// that writes the state into its objects).
fn draw(fe: &mut Frontend, overlay: &WorldMapOverlay, dt: f32, chrome: Chrome, host: &mut Host, env: &mut Env) {
    if !matches!(fe.stage, Stage::Map) {
        return;
    }
    fe.screens.set_world_map(Some(overlay.frame(chrome)));
    let commands = fe.screens.update(dt, 0, env);
    fe.apply(commands, host, env);
}

fn close(fe: &mut Frontend, overlay: &mut WorldMapOverlay) {
    log::info!("world map closed");
    overlay.close();
    fe.screens.set_world_map(None);
    fe.resume();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_style_has_a_map_icon_file() {
        for style in [AUTO_SHOP, CAR_LOT, SAFE_HOUSE, RACE_SPRINT, RACE_SPEEDTRAP, RACE_CIRCUIT, RACE_DRAG] {
            assert!(icon_name(style).starts_with("Minimap_Icon_") && icon_name(style).ends_with(".tga"), "{style}");
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
