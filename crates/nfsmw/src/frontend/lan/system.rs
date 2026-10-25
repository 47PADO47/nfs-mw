//! The LAN menu in the app: the [`LanUi`] resource, the frame system that feeds it the pad, the typed keys, a `--ui-script`
//! and the mouse (before the front end's own frame, which then waits: `gate.rs`), and the system that hands the page or the
//! session HUD to the themes ([`publish`]), which draw them with the game's UI toolkit (`ui/theme`, `present.rs`).

use bevy_app::{App, Update};
use bevy_ecs::prelude::*;
use bevy_input::ButtonInput;
use bevy_input::keyboard::KeyCode;
use bevy_input::mouse::MouseButton;
use bevy_time::Time;
use bevy_window::{PrimaryWindow, Window};
use blackbox_feng::fe_hash_upper;
use blackbox_feng::runtime::pad;
use blackbox_ui::ThemeId;
use nfsmw_data::gameplay::RaceType;
use nfsmw_net::message::Mode;

use super::demo::{self, DemoNet};
use super::handle::LanBackend;
use super::hud::hud_view;
use super::menu::{Cx, LanKey, LanMenu, MenuOut, TrackChoice, Typed, city};
use super::offline::Offline;
use super::present::{self, ClickMap};
use super::{NetFrontend, link};
use crate::devtools::Console;
use crate::frontend::flow::Frontend;
use crate::frontend::name_entry::TextKey;
use crate::frontend::plugin_keys::{TypedKeys, pad_mask, text_mask};
use crate::input::{Action, ActionState};
use crate::settings::Settings;
use crate::ui::theme::remastered::menu::layout::Side;
use crate::ui::theme::{ElementId, UiThemes};

/// The LAN menu and what it keeps between frames.
#[derive(Resource, Default)]
pub struct LanUi {
    pub menu: LanMenu,
    /// The pad mask of the last frame: a key acts on its press.
    prev_mask: u32,
    /// The same for the spectator's follow keys while the world has the screen.
    follow_mask: u32,
    /// Where the page published last put its rows and hints, for a click on them.
    clicks: ClickMap,
    /// The row that shows first, kept so the window of rows moves only when the selection leaves it.
    first: usize,
    /// The network of a run with none wired in: refusals, or the demo (`NFSMW_LAN_DEMO`).
    fallback: Fallback,
    cars: Vec<String>,
    tracks_loaded: bool,
}

#[derive(Default)]
enum Fallback {
    #[default]
    Unset,
    Offline(Offline),
    Demo(Box<DemoNet>),
}

impl Fallback {
    fn net(&mut self) -> &mut dyn NetFrontend {
        if matches!(self, Self::Unset) {
            *self = match demo::wanted() {
                true => Self::Demo(Box::default()),
                false => Self::Offline(Offline::default()),
            };
        }
        match self {
            Self::Demo(net) => net.as_mut(),
            Self::Offline(net) => net,
            Self::Unset => unreachable!("set above"),
        }
    }
}

/// The game's network, or the fallback.
fn network<'a>(backend: Option<&'a mut LanBackend>, fallback: &'a mut Fallback) -> &'a mut dyn NetFrontend {
    match backend.and_then(LanBackend::get) {
        Some(net) => net,
        None => fallback.net(),
    }
}

pub(in crate::frontend) fn build(app: &mut App) {
    app.init_resource::<LanUi>()
        .add_systems(
            Update,
            update
                .after(crate::frontend::plugin_keys::collect_typed)
                .before(crate::app::FrameSet::Frontend)
                .run_if(resource_exists::<Frontend>),
        )
        .add_systems(
            Update,
            publish.after(update).before(crate::app::FrameSet::Hud).run_if(resource_exists::<UiThemes>),
        );
}

/// The keys of `edges` (pressed this frame), in pad order.
pub fn keys_of(edges: u32) -> Vec<LanKey> {
    [
        (pad::UP, LanKey::Up),
        (pad::DOWN, LanKey::Down),
        (pad::LEFT, LanKey::Left),
        (pad::RIGHT, LanKey::Right),
        (pad::ACCEPT, LanKey::Accept),
        (pad::BACK, LanKey::Back),
    ]
    .into_iter()
    .filter(|(bit, _)| edges & bit != 0)
    .map(|(_, key)| key)
    .collect()
}

/// The menu's frame: open on request, take the keys while on screen, follow the session.
#[allow(clippy::too_many_arguments)]
fn update(
    mut ui: ResMut<LanUi>,
    mut fe: ResMut<Frontend>,
    mut backend: Option<NonSendMut<LanBackend>>,
    mut settings: ResMut<Settings>,
    actions: Res<ActionState>,
    keys: Res<ButtonInput<KeyCode>>,
    mut typed: ResMut<TypedKeys>,
    time: Res<Time>,
    console: Res<Console>,
    mouse: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
) {
    let ui = &mut *ui;
    let net = network(backend.as_deref_mut(), &mut ui.fallback);
    if link::take_open_request() {
        load_lists(&mut ui.cars, &mut ui.tracks_loaded, &mut ui.menu);
        ui.menu.open(&net.status());
    }
    let mut cx = Cx::new(net, &settings, &ui.cars);
    let mut saved_name = None;
    if let Some(answer) = link::take_car_answer() {
        if let Some(folder) = answer {
            ui.menu.car_chosen(&folder, &mut cx);
        }
        ui.menu.open = true;
    }
    ui.menu.frame(&mut cx, time.delta_secs());
    let visible = ui.menu.visible(&cx.net.status());
    fe.lan_input = visible;
    if !visible || console.open {
        ui.prev_mask = u32::MAX;
        // A spectator steps the follow camera with left and right while the world has the screen (MP-41).
        let mask = pad_mask(&actions);
        let edges = mask & !ui.follow_mask;
        ui.follow_mask = mask;
        if !console.open && cx.net.status() == super::NetStatus::Spectating {
            follow_keys(&mut *cx.net, edges);
        }
        // The vote key votes for the running vote while the world has the screen.
        if !console.open && keys.just_pressed(KeyCode::F7) {
            super::hud::cast_vote(&mut *cx.net);
        }
        return;
    }
    let mut pressed = match mouse.just_pressed(MouseButton::Left) {
        true => clicked(&ui.clicks, crate::frontend::pointer::feng_point(&window)),
        false => Vec::new(),
    };
    let scripted = fe.script.as_mut().and_then(|script| script.next_mask());
    let script_text = fe.script.as_mut().and_then(|script| script.take_text()).unwrap_or_default();
    let mask = match (scripted, ui.menu.typing()) {
        (Some(mask), _) => mask,
        (None, true) => text_mask(&keys),
        (None, false) => pad_mask(&actions),
    };
    // A key held when the menu opened waits for its release (`u32::MAX` after a hidden frame).
    let edges = mask & !ui.prev_mask;
    ui.prev_mask = mask;
    let typed_keys = std::mem::take(&mut typed.0).into_iter().map(|key| match key {
        TextKey::Char(c) => Typed::Char(c),
        TextKey::Backspace => Typed::Backspace,
    });
    let script_keys = script_text.chars().map(|c| match c {
        '~' => Typed::Backspace,
        c => Typed::Char(c),
    });
    if ui.menu.typing() {
        pressed.extend(typed_keys.chain(script_keys).map(LanKey::Typed));
    }
    pressed.extend(keys_of(edges));
    for key in pressed {
        ui.menu.key(key, &mut cx);
    }
    for out in std::mem::take(&mut cx.out) {
        match out {
            // The main menu takes no press the closing key is still holding.
            MenuOut::Close => fe.latch = true,
            MenuOut::CarSelect => {
                ui.menu.open = false;
                link::request_car_select();
            }
            MenuOut::SaveName(name) => saved_name = Some(name),
        }
    }
    fe.lan_input = ui.menu.visible(&cx.net.status());
    if let Some(name) = saved_name {
        save_name(&mut settings, name, fe.save);
    }
}

/// The player's new name: in the settings now, and in the config file as `net_name` when the front end saves.
fn save_name(settings: &mut Settings, name: crate::settings::net::NetName, save: bool) {
    settings.net.name = name;
    if !save {
        return;
    }
    let layer = crate::settings::net::NetLayer { name: Some(name), ..Default::default() };
    match Settings::save(&crate::settings::Partial { net: layer, ..Default::default() }) {
        Ok(path) => log::info!("net_name saved to {}", path.display()),
        Err(e) => log::warn!("net_name could not be saved: {e:#}"),
    }
}

/// Left and right pressed this frame step the spectator camera to the previous or next racer.
pub fn follow_keys(net: &mut dyn NetFrontend, edges: u32) {
    if edges & pad::LEFT != 0 {
        net.follow(false);
    }
    if edges & pad::RIGHT != 0 {
        net.follow(true);
    }
}

/// The car folders and the tracks, once the background readers have them.
fn load_lists(cars: &mut Vec<String>, loaded: &mut bool, menu: &mut LanMenu) {
    if cars.is_empty()
        && let Some(pool) = crate::frontend::quick_play::pool::cars()
    {
        *cars = pool.cars.iter().map(|car| car.folder.clone()).collect();
    }
    if *loaded {
        return;
    }
    let Some(board) = crate::frontend::quick_race::board() else { return };
    let mut tracks = vec![city()];
    tracks.extend(board.tracks.iter().filter_map(|track| {
        let mode = track_mode(track.race_type)?;
        Some(TrackChoice { id: fe_hash_upper(&track.name), title: track.title.clone(), mode: Some(mode) })
    }));
    menu.set_tracks(tracks);
    *loaded = true;
}

/// The session mode a race of the track list runs as; `None` for the kinds a session does not run.
fn track_mode(kind: RaceType) -> Option<Mode> {
    match kind {
        RaceType::Circuit => Some(Mode::Circuit),
        RaceType::Sprint => Some(Mode::Sprint),
        RaceType::SpeedTrap => Some(Mode::Speedtrap),
        RaceType::Knockout => Some(Mode::Knockout),
        _ => None,
    }
}

/// The keys a left click at `point` (FEng units) presses on the page drawn last: a chevron of a choice steps it, any other
/// part of a row is a click on the row, and a footer hint presses what it is for. A click outside them presses nothing.
fn clicked(map: &ClickMap, point: Option<glam::Vec2>) -> Vec<LanKey> {
    let Some(point) = point else { return Vec::new() };
    match map.hint_at(point) {
        Some(Action::MenuAccept) => return vec![LanKey::Accept],
        Some(Action::MenuBack) => return vec![LanKey::Back],
        _ => {}
    }
    let Some((row, side)) = map.row_at(point) else { return Vec::new() };
    let choice = map.choices.get(row).copied().unwrap_or(false);
    match (choice, side) {
        (true, Side::Left) => vec![LanKey::Step(row, false)],
        (true, Side::Right) => vec![LanKey::Step(row, true)],
        _ => vec![LanKey::Click(row)],
    }
}

/// Hands the page (when the menu is on screen) or the session's HUD (while the world has the screen) to the themes' cell,
/// and keeps where the page's rows are for the next click. Runs after the menu's frame, before the screens are presented.
pub(in crate::frontend) fn publish(
    mut ui: ResMut<LanUi>,
    mut backend: Option<NonSendMut<LanBackend>>,
    settings: Res<Settings>,
    themes: Res<UiThemes>,
    window: Single<&Window, With<PrimaryWindow>>,
) {
    let ui = &mut *ui;
    let cell = themes.frame().lan();
    let aspect = window.width() / window.height().max(1.0);
    let net = network(backend.as_deref_mut(), &mut ui.fallback);
    let status = net.status();
    if ui.menu.visible(&status) {
        let view = ui.menu.view(&Cx::new(net, &settings, &ui.cars));
        ui.first = present::window_first(ui.first, view.selected, view.rows.len());
        let page = present::page_of(&view, ui.first, aspect);
        // The Custom look lays the rows out its own way, so a click is tested against its places.
        let custom =
            (themes.provided_by(ElementId::LanMenu) == Some(ThemeId::Custom)).then_some(settings.hud_options.text_size);
        ui.clicks = ClickMap::of(&view, &page).in_custom(custom);
        cell.set_page(page);
        return;
    }
    ui.clicks = ClickMap::default();
    match hud_view(&ui.menu, net) {
        Some(hud) => cell.set_hud(present::hud_of(&hud, aspect)),
        None => cell.clear(),
    }
}

/// The `net` line of the metrics overlay (MP-82): the session's line, when there is a session.
pub fn stats_line(ui: &mut LanUi, backend: Option<&mut LanBackend>) -> Option<String> {
    network(backend, &mut ui.fallback).stats_line().map(|line| line.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presses_become_keys_in_pad_order() {
        assert_eq!(keys_of(pad::ACCEPT | pad::UP), [LanKey::Up, LanKey::Accept]);
        assert!(keys_of(0).is_empty());
    }

    #[test]
    fn a_spectator_steps_the_follow_camera_with_left_and_right() {
        use crate::frontend::lan::mock::{Call, MockNetFrontend};
        let mut net = MockNetFrontend::default();
        follow_keys(&mut net, pad::LEFT | pad::RIGHT | pad::ACCEPT);
        assert_eq!(net.take_calls(), [Call::Follow(false), Call::Follow(true)]);
    }

    #[test]
    fn only_the_race_kinds_a_session_runs_get_tracks() {
        assert_eq!(track_mode(RaceType::Circuit), Some(Mode::Circuit));
        assert_eq!(track_mode(RaceType::Drag), None);
        assert_eq!(track_mode(RaceType::Tollbooth), None);
    }
}
