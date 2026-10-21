//! The Bevy side of the front end: drives the flow every frame and draws the screens.

use std::sync::Arc;

use bevy_app::{App, AppExit, Plugin, Update};
use bevy_asset::Assets;
use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use bevy_image::Image;
use bevy_input::ButtonInput;
use bevy_input::keyboard::KeyCode;
use bevy_input::mouse::MouseButton;
use bevy_time::Time;
use bevy_window::{PrimaryWindow, Window};
use blackbox_feng::runtime::pad;
use game_install::GameDir;
use glam::Vec2;

use super::browse::Browse;
use super::flow::{Boot, Frontend, Stage, Start};
use super::logic::{Command, Env};
use super::plugin_keys::{TypedKeys, collect_typed, pad_mask, remember_key, text_mask};
use super::pointer::{self, feng_point};
use super::screens::{Presented, Screens};
use super::script::{self, UiScript};
use super::world_map::{Chrome, WorldMapOverlay, control, layout};
use crate::app::{FrameSet, Host};
use crate::devtools::Console;
use crate::gui::UiOutput;
use crate::input::{Action, ActionState, Bindings, InputDevice, InputPresentation, MouseCapture};
use crate::settings::Settings;
use crate::ui::present::{FENG_HEIGHT, Screen};
use crate::ui::theme::remastered::MapSnapshot;
use crate::ui::theme::{ElementId, UiThemes, slot_layer};
use crate::ui::{Catalog, Presenter, SCREEN_FILES, SharedAssets};

/// Longest step the screens' clock takes, so a stall does not skip animations.
const MAX_STEP: f32 = 0.1;

pub struct FrontendPlugin {
    pub dir: GameDir,
    pub start: Start,
    /// A scripted pad that replaces the real input (`--ui-script`).
    pub script: Option<UiScript>,
    /// The sidebar that picks the screens, for `feng-viewer` (`Start::Browse`).
    pub browse: Option<Browse>,
    /// The career being played: the same handle the HUD and the world hold.
    pub career: crate::career::CareerHandle,
    /// A backdrop look that stays whatever the screens ask for (`view-screen --look`, for checking the looks offscreen).
    pub look: Option<super::look::Look>,
}

impl Plugin for FrontendPlugin {
    fn build(&self, app: &mut App) {
        super::jukebox::prime(&self.dir);
        super::credits::prime(&self.dir);
        let Some(assets) = crate::ui::ensure(app, &self.dir) else { return };
        let catalog = Catalog::load(&self.dir, &SCREEN_FILES);
        let screens = Screens::new(catalog, assets);
        let mut frontend = Frontend::new(screens, self.dir.clone(), self.start.clone(), self.script.clone());
        frontend.browse = self.browse.clone();
        frontend.career = self.career.clone();
        if let Some(look) = &self.look {
            frontend.look = look.clone();
            frontend.look_override = Some(look.clone());
        }
        app.insert_resource(frontend)
            .init_resource::<WorldMapOverlay>()
            .init_resource::<Assets<Image>>()
            .init_resource::<TypedKeys>()
            .add_systems(Update, collect_typed.before(update))
            .add_systems(Update, update.in_set(FrameSet::Frontend))
            .add_systems(Update, present.in_set(FrameSet::Hud))
            .add_systems(Update, pointer::apply);
    }
}

/// Whether the pointer is over the world map's picture, where the map draws its own cross cursor. Outside it (the
/// buttons, the list) the system pointer shows, so it can click. `app::cursor::update` owns the pointer's visibility
/// and asks this.
pub fn map_pointer_on_picture(fe: &Frontend, window: &Window) -> bool {
    let map_on_top = matches!(fe.stage, Stage::Map) && fe.screens.top() == Some(super::ids::screen::WORLD_MAP);
    if !map_on_top {
        return false;
    }
    // The themed full-screen map draws its own cursor over the whole window.
    if fe.map_chrome == Chrome::Drawn {
        return true;
    }
    feng_point(window)
        .map(layout::view_of)
        .is_some_and(|view| (0.0..=1.0).contains(&view[0]) && (0.0..=1.0).contains(&view[1]))
}

/// What a click on the menus presses.
enum Press {
    /// Accept, held while the button is held.
    Accept,
    /// Back, once.
    Back,
    /// Quit, once, as the quit action does on the main menu.
    Quit,
    /// A face button, once, as the pad mask it presses (Game Stats on the main menu, Randomize on the Quick Play brief, Defaults,
    /// View Tutorial, Showcase).
    Button(u32),
    /// A screen's own part (an option row or its arrow), with what it queued.
    Screen(Vec<Command>),
    Nothing,
}

/// What a click at a point of the menus presses: a footer hint first, then the screen's own parts, then a button.
fn press_at(fe: &mut Frontend, point: Vec2, bindings: &Bindings, device: InputDevice, env: &mut Env) -> Press {
    match fe.screens.hint_at(point, bindings, device) {
        Some(Action::MenuAccept) => return Press::Accept,
        Some(Action::MenuBack) => return Press::Back,
        Some(Action::MenuQuit) => return Press::Quit,
        Some(Action::MenuButton1) => return Press::Button((pad::BUTTON0 << 1) | (pad::BUTTON0 << 5)),
        Some(Action::MenuButton2) => return Press::Button(pad::BUTTON0 << 4),
        Some(Action::MenuButton3) => return Press::Button(pad::BUTTON0),
        _ => {}
    }
    if let Some(commands) = fe.screens.click(point, env) {
        return Press::Screen(commands);
    }
    if fe.screens.focus_button_at(point) {
        return Press::Accept;
    }
    if fe.screens.release_at(point, super::ids::MOUSE_LEFT_RELEASED) {
        return Press::Nothing;
    }
    Press::Nothing
}

/// What the world map needs, grouped so the frame system stays within Bevy's parameter limit.
#[derive(SystemParam)]
struct MapParams<'w> {
    overlay: ResMut<'w, WorldMapOverlay>,
    mouse: Res<'w, ButtonInput<MouseButton>>,
    /// The theme registry: whether the active theme draws the map chrome, and the feed that chrome reads.
    themes: Res<'w, UiThemes>,
    /// The mods, for the rows of the options' Mods page (`bb.menus`); absent without mods.
    mods: Option<NonSend<'w, crate::modding::Mods>>,
    /// The keys typed this frame, for a screen that takes text.
    typed: ResMut<'w, TypedKeys>,
}

/// How long a menu step may take before it is logged as a stall.
const SLOW_MENU_STEP: std::time::Duration = std::time::Duration::from_millis(250);

#[allow(clippy::too_many_arguments)]
fn update(
    mut fe: ResMut<Frontend>,
    mut host: NonSendMut<Host>,
    actions: Res<ActionState>,
    keys: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<Settings>,
    time: Res<Time>,
    console: Res<Console>,
    mut capture: ResMut<MouseCapture>,
    mut exit: MessageWriter<AppExit>,
    input: Res<InputPresentation>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut bindings: ResMut<Bindings>,
    mut clicking: Local<bool>,
    mut wheel: Local<f32>,
    overlays: (ResMut<crate::garage::Overlay>, ResMut<crate::devtools::GraphicsLab>, ResMut<crate::input::UiFocus>),
    mut map: MapParams,
) {
    let _trace = blackbox_trace::scope("Frontend: update");
    let (mut garage, mut lab, mut focus) = overlays;
    if fe.graphics_requested {
        fe.graphics_requested = false;
        fe.latch = true;
        lab.open_from_menu(&mut capture, &mut focus);
    }
    if lab.open {
        if !fe.graphics_visible {
            let mut pending = std::mem::take(&mut fe.changed);
            fe.save_settings(&mut pending);
            fe.changed = pending;
        }
        fe.graphics_visible = true;
        *clicking = false;
        *wheel = 0.0;
        return;
    }
    if fe.graphics_visible {
        fe.graphics_visible = false;
        fe.latch = true;
    }
    if garage.open || garage.had_input {
        *clicking = false;
        *wheel = 0.0;
        return;
    }
    let (fe, host) = (&mut *fe, &mut *host);
    fe.screens.set_aspect(window.width() / window.height());
    let Some(caps) = host.renderer.as_ref().map(|r| *r.capabilities()) else { return };
    let mut changed = std::mem::take(&mut fe.changed);
    // The screens work on a copy so the resource is only marked as changed when a setting really changed.
    let mut current = *settings;
    let mut env = Env { settings: &mut current, changed: &mut changed, caps, mods: map.mods.as_deref() };
    fe.begin(host, &mut env);
    // The sidebar's pick is shown on the frame after it was clicked (its scene runs after this system).
    let picked = fe.browse.as_ref().and_then(Browse::take);
    if let Some(name) = picked {
        fe.show_browsed(&name, host, &mut env);
    }

    // Escape belongs to the front end unless the console had it (it closes the console on the same press).
    let console_had_it = console.open || fe.console_was_open;
    fe.console_was_open = console.open;
    host.cancel_handled = !console.open;
    host.flow_driven = true;

    // A script replaces the input and the clock.
    let scripted = fe.script.as_mut().and_then(UiScript::next_mask);
    // A screenshot waits for the world while the loading screen is up (the world is not in the window until it is built).
    host.hold_capture = fe.script.as_ref().is_some_and(|s| !s.is_over()) || fe.stage == Stage::Loading;
    let dt = if scripted.is_some() { script::STEP } else { time.delta_secs().min(MAX_STEP) };
    // A slot waiting for its key takes the next key pressed; Escape cancels the wait.
    let script_key = fe.script.as_mut().and_then(UiScript::take_key);
    // The pointer on the screen (FEng units): a scripted run sets it (`pointer X Y`), else it is the window's.
    let pointer = fe.script.as_ref().and_then(UiScript::pointer).or_else(|| feng_point(&window));
    let device = fe.script.as_ref().and_then(|s| s.device).unwrap_or(input.device);
    if let Some((control, slot)) = fe.remap
        && let Some(key) = script_key.or_else(|| keys.get_just_pressed().next().copied())
    {
        fe.remap = None;
        if key != KeyCode::Escape {
            remember_key(&mut bindings, control, slot, key);
        }
    }
    fe.screens.set_key_texts(super::controls_screen::key_texts(&bindings, fe.remap));
    let mut mask = scripted.unwrap_or_else(|| pad_mask(&actions));
    // A click does what accept does while the game boots: it skips a movie and continues from the title screen.
    if matches!(fe.stage, Stage::Boot(_)) && actions.pressed(Action::Click) {
        mask |= pad::ACCEPT;
    }
    // On the menus a click presses what is under the pointer and holds accept while a button is held. A sidebar's
    // list takes its own clicks and wheel turns, so none of them reach the screen under it.
    let (mut press_back, mut press_quit, mut press_buttons) = (false, false, 0);
    let on_list = fe.browse.as_ref().is_some_and(Browse::over_panel);
    // The GPS Yes/No dialog is the one menu that opens over the map; it takes the menu click and pad handling too.
    let dialog_over_map = matches!(fe.stage, Stage::Map) && fe.screens.top() == Some(super::ids::screen::DIALOG);
    // The GPS dialog follows the pointer with its focus, so the answer under it shows as selected before a click.
    if dialog_over_map && let Some(point) = pointer {
        fe.screens.hover(point, &mut env);
    }
    if matches!(fe.stage, Stage::Menus | Stage::Paused) || dialog_over_map {
        // A vertical menu follows the pointer's movement: the row under it is selected, as the keys would.
        if !on_list {
            fe.screens.hover_row(pointer, &mut env);
        }
        if actions.just_pressed(Action::Click)
            && !on_list
            && let Some(point) = pointer
        {
            match press_at(fe, point, &bindings, device, &mut env) {
                Press::Accept => *clicking = true,
                Press::Back => press_back = true,
                Press::Quit => press_quit = true,
                Press::Button(bits) => press_buttons |= bits,
                Press::Screen(commands) => fe.apply(commands, host, &mut env),
                Press::Nothing => {}
            }
        }
        if !actions.pressed(Action::Click) {
            *clicking = false;
        }
        if *clicking {
            mask |= pad::ACCEPT;
        }
        if press_back {
            mask |= pad::BACK;
        }
        mask |= press_buttons;
    }
    // A screen that takes text (the alias name prompt) gets the typed characters, and only Enter and Escape press its pad.
    if matches!(fe.stage, Stage::Menus | Stage::Paused) && fe.screens.wants_text() {
        let typed = std::mem::take(&mut map.typed.0).into_iter();
        let scripted_text = fe.script.as_mut().and_then(UiScript::take_text).unwrap_or_default();
        // In a script `~` is Backspace.
        let from_script = scripted_text.chars().map(|c| match c {
            '~' => super::name_entry::TextKey::Backspace,
            c => super::name_entry::TextKey::Char(c),
        });
        for key in typed.chain(from_script) {
            let commands = fe.screens.type_key(key, &mut env);
            fe.apply(commands, host, &mut env);
        }
        if scripted.is_none() {
            mask = text_mask(&keys);
        }
    } else {
        map.typed.0.clear();
    }
    // The wheel scrolls the menus a notch at a time; the fraction of a notch waits for the next frame.
    let menus = matches!(fe.stage, Stage::Menus | Stage::Paused) && !on_list;
    *wheel = if menus { *wheel + actions.value(Action::Zoom) } else { 0.0 };
    let notches = wheel.trunc();
    *wheel -= notches;
    if menus && notches != 0.0 {
        let commands = fe.screens.scroll(notches as i32, &mut env);
        fe.apply(commands, host, &mut env);
    }
    if fe.latch {
        if mask != 0 {
            mask = 0;
        } else {
            fe.latch = false;
        }
    }

    match fe.stage {
        Stage::Boot(Boot::Movie { bypass, .. }) => {
            let skip = bypass
                && (actions.just_pressed(Action::MenuAccept)
                    || actions.just_pressed(Action::MenuStart)
                    || actions.just_pressed(Action::Click));
            if host.scene.finished() || skip {
                fe.next_boot(host, &mut env);
            }
        }
        Stage::Tutorial => {
            let skip = actions.just_pressed(Action::MenuAccept)
                || actions.just_pressed(Action::MenuStart)
                || actions.just_pressed(Action::MenuBack)
                || actions.just_pressed(Action::Click);
            if host.scene.finished() || skip {
                fe.end_tutorial(host, &mut env);
            }
        }
        // Nothing pauses or skips the loading screen: the pad only drives its animation (mask 0 below).
        Stage::Loading => fe.loading_frame(dt, host, &mut env),
        Stage::Playing => {
            // The GPS dialog's Yes resumes the game; the map overlay it left open is cleared here.
            if map.overlay.open {
                map.overlay.close();
                map.themes.world_map_legend().clear();
                map.themes.world_map_view().clear();
            }
            fe.poll_bust(host, &mut env);
            fe.poll_race(host, &mut env);
            fe.poll_career(host, &mut env);
            // An auto shop asks the garage to open on the next frame (`cursor.rs` opens it, as the start request does).
            if std::mem::take(&mut fe.auto_shop_requested) {
                garage.start_tab = crate::garage::OptionsTab::Performance;
                garage.start_requested = true;
            }
            let scripted_start = mask & pad::START != 0 && fe.last_mask & pad::START == 0;
            let pause = input.disconnected
                || (!console_had_it
                    && (actions.just_pressed(Action::Cancel)
                        || actions.just_pressed(Action::MenuStart)
                        || scripted_start));
            if pause {
                capture.0 = false;
                fe.pause(host, &mut env);
            }
            if !pause && !console_had_it && (actions.just_pressed(Action::Map) || script_key == Some(KeyCode::KeyM)) {
                control::open(fe, &mut map.overlay, host, &mut env);
            }
        }
        Stage::Map if dialog_over_map => {
            // The GPS Yes/No dialog is over the map: it takes the input (handled above), the map stays frozen behind.
            let commands = fe.screens.update(dt, mask, &mut env);
            fe.apply(commands, host, &mut env);
        }
        Stage::Map => {
            // How the active theme dresses the map: Custom recolours the FEng screen dark in place (and draws a legend
            // over it), Remastered draws the whole map itself and the FEng screen only runs underneath.
            let chrome = map.themes.world_map_chrome();
            fe.map_chrome = chrome;
            let picture = chrome.picture();
            let side = picture.side * window.height() / FENG_HEIGHT;
            let cursor = pointer.map(|point| picture.view_of(point));
            let map_input = control::input(&actions, &keys, script_key, &map.mouse, cursor, side, chrome);
            control::step(fe, &mut map.overlay, map_input, dt, chrome, host, &mut env);
            let (legend, view) = (map.themes.world_map_legend(), map.themes.world_map_view());
            match (chrome, map.overlay.open) {
                (Chrome::Recoloured, true) => legend.set(&map.overlay.frame(chrome)),
                (Chrome::Drawn, true) => {
                    let (hints, aspect) = (control::hints(&bindings, device), window.width() / window.height());
                    view.set(MapSnapshot { frame: map.overlay.frame(chrome), hints, aspect });
                }
                _ => {}
            }
            if chrome != Chrome::Recoloured || !map.overlay.open {
                legend.clear();
            }
            if chrome != Chrome::Drawn || !map.overlay.open {
                view.clear();
            }
        }
        Stage::Boot(Boot::Splash) | Stage::Menus | Stage::Paused => {
            if (actions.just_pressed(Action::MenuQuit) || press_quit)
                && fe.screens.top() == Some(super::ids::screen::MAIN_MENU)
            {
                fe.screens.post_to_top(super::ids::QUIT_BUTTON, super::ids::MOUSE_LEFT_RELEASED);
            }
            let started = std::time::Instant::now();
            let top = fe.screens.top().map(str::to_owned);
            let commands = fe.screens.update(dt, mask, &mut env);
            fe.apply(commands, host, &mut env);
            fe.sync_backdrop(host);
            // A menu step that stalls the frame is worth a line in the log: it says which screen it started on.
            let took = started.elapsed();
            if took >= SLOW_MENU_STEP {
                log::warn!("a menu step took {} ms (on {})", took.as_millis(), top.as_deref().unwrap_or("no screen"));
            }
        }
    }

    fe.last_mask = mask;
    if current != *settings {
        *settings = current;
    }
    if fe.quit {
        exit.write(AppExit::Success);
    }
    fe.changed = changed;
}

/// Draws the screens through the theme composer. Each screen with a menu slot fills that slot's cell once per frame
/// (the tree the presenter gets), and a slot another screen already took keeps its own tree. The menu slots are cleared
/// first, so a closed screen leaves no tree behind (`docs/specs/ui-themes-menus.md`, section 4).
#[allow(clippy::too_many_arguments)]
fn present(
    fe: Res<Frontend>,
    assets: Res<SharedAssets>,
    mut presenter: ResMut<Presenter>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut out: ResMut<UiOutput>,
    bindings: Res<Bindings>,
    input: Res<InputPresentation>,
    themes: Res<UiThemes>,
    time: Res<Time>,
) {
    let _trace = blackbox_trace::scope("Frontend: present");
    let screen = Screen { width: window.width(), height: window.height(), pixels_per_point: window.scale_factor() };
    // The presenter puts what it draws below what is already there, so the top screen goes first.
    let device = fe.script.as_ref().and_then(|s| s.device).unwrap_or(input.device);
    let frame = themes.frame();
    for slot in ElementId::ALL.iter().filter(|slot| slot.is_menu()) {
        frame.clear_menu(slot.slot());
    }
    let lists = themes.menu_lists();
    lists.clear();
    let time_s = time.elapsed_secs();
    // The Remastered map legend draws over the recoloured FEng map. The presenter puts each call below the calls before
    // it (`movie` inserts its backdrop at the front for the same reason), so the legend is presented FIRST to sit on top
    // of the map screen, which the loop presents after it. A screen stacked over the map (the GPS Yes/No `Dialog.fng`)
    // must stay on top, so the legend is skipped while the map is not the top screen; the frozen map shows behind it.
    let map_on_top = fe.screens.top() == Some(super::ids::screen::WORLD_MAP);
    let chrome = themes.world_map_chrome();
    if chrome == Chrome::Recoloured && matches!(fe.stage, Stage::Map) && map_on_top {
        let legend = slot_layer(&themes, ElementId::WorldMapChrome, time_s);
        if !legend.nodes.is_empty() {
            presenter.0.present(&legend, &assets.0, screen, &mut out);
        }
    }
    let mut claimed: Vec<ElementId> = Vec::new();
    for Presented { tree, slot, list, name } in fe.screens.presented(&bindings, device).into_iter().rev() {
        // The themed full-screen map replaces the FEng map screen, in its place in the stack (a dialog over it stays on top).
        if chrome == Chrome::Drawn && name.eq_ignore_ascii_case(super::ids::screen::WORLD_MAP) {
            presenter.0.present(&slot_layer(&themes, ElementId::WorldMapChrome, time_s), &assets.0, screen, &mut out);
            continue;
        }
        let layer = match slot {
            // The topmost screen on a slot takes it. A screen under it on the same slot is drawn as its own tree.
            Some(slot) if !claimed.contains(&slot) => {
                claimed.push(slot);
                if let Some(list) = list {
                    lists.set(slot.slot(), list);
                }
                frame.set_menu(slot.slot(), Arc::new(tree));
                slot_layer(&themes, slot, time_s)
            }
            _ => tree,
        };
        presenter.0.present(&layer, &assets.0, screen, &mut out);
    }
}
