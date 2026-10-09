//! The Bevy side of the front end: drives the flow every frame and draws the screens.

use bevy_app::{App, AppExit, Plugin, Update};
use bevy_ecs::prelude::*;
use bevy_time::Time;
use bevy_window::{PrimaryWindow, Window};
use blackbox_feng::runtime::pad;
use game_install::GameDir;

use super::flow::{Boot, Frontend, Stage, Start};
use super::logic::Env;
use super::screens::Screens;
use super::script::{self, UiScript};
use crate::app::{FrameSet, Host};
use crate::devtools::Console;
use crate::gui::UiOutput;
use crate::input::{Action, ActionState, Bindings, InputPresentation, MouseCapture};
use crate::settings::Settings;
use crate::ui::present::Screen;
use crate::ui::{Catalog, Presenter, SCREEN_FILES, SharedAssets};

/// Longest step the screens' clock takes, so a stall does not skip animations.
const MAX_STEP: f32 = 0.1;

pub struct FrontendPlugin {
    pub dir: GameDir,
    pub start: Start,
    /// A scripted pad that replaces the real input (`--ui-script`).
    pub script: Option<UiScript>,
}

impl Plugin for FrontendPlugin {
    fn build(&self, app: &mut App) {
        let Some(assets) = crate::ui::ensure(app, &self.dir) else { return };
        let catalog = Catalog::load(&self.dir, &SCREEN_FILES);
        let screens = Screens::new(catalog, assets);
        app.insert_resource(Frontend::new(screens, self.dir.clone(), self.start.clone(), self.script.clone()))
            .add_systems(Update, update.in_set(FrameSet::Frontend))
            .add_systems(Update, present.in_set(FrameSet::Hud));
    }
}

/// The pad mask the real input makes this frame.
fn pad_mask(actions: &ActionState) -> u32 {
    [
        (Action::MenuUp, pad::UP),
        (Action::MenuDown, pad::DOWN),
        (Action::MenuLeft, pad::LEFT),
        (Action::MenuRight, pad::RIGHT),
        (Action::MenuAccept, pad::ACCEPT),
        (Action::MenuBack, pad::BACK),
        (Action::MenuStart, pad::START),
    ]
    .into_iter()
    .filter(|(action, _)| actions.pressed(*action))
    .fold(0, |mask, (_, bit)| mask | bit)
}

#[allow(clippy::too_many_arguments)]
fn update(
    mut fe: ResMut<Frontend>,
    mut host: NonSendMut<Host>,
    actions: Res<ActionState>,
    mut settings: ResMut<Settings>,
    time: Res<Time>,
    console: Res<Console>,
    mut capture: ResMut<MouseCapture>,
    mut exit: MessageWriter<AppExit>,
    input: Res<InputPresentation>,
) {
    let (fe, host) = (&mut *fe, &mut *host);
    if host.renderer.is_none() {
        return;
    }
    let mut changed = std::mem::take(&mut fe.changed);
    // The screens work on a copy so the resource is only marked as changed when a setting really changed.
    let mut current = *settings;
    let mut env = Env { settings: &mut current, changed: &mut changed };
    fe.begin(host, &mut env);

    // Escape belongs to the front end unless the console had it (it closes the console on the same press).
    let console_had_it = console.open || fe.console_was_open;
    fe.console_was_open = console.open;
    host.cancel_handled = !console.open;
    host.flow_driven = true;

    // A script replaces the input and the clock.
    let scripted = fe.script.as_mut().and_then(UiScript::next_mask);
    host.hold_capture = fe.script.as_ref().is_some_and(|s| !s.is_over());
    let dt = if scripted.is_some() { script::STEP } else { time.delta_secs().min(MAX_STEP) };
    let mut mask = scripted.unwrap_or_else(|| pad_mask(&actions));
    // A click does what accept does while the game boots: it skips a movie and continues from the title screen.
    if matches!(fe.stage, Stage::Boot(_)) && actions.pressed(Action::Click) {
        mask |= pad::ACCEPT;
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
        Stage::Playing => {
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
        }
        Stage::Boot(Boot::Splash) | Stage::Menus | Stage::Paused => {
            if actions.just_pressed(Action::MenuQuit) && fe.screens.top() == Some(super::ids::screen::MAIN_MENU) {
                fe.screens.post_to_top(super::ids::QUIT_BUTTON, super::ids::MOUSE_LEFT_RELEASED);
            }
            let commands = fe.screens.update(dt, mask, &mut env);
            fe.apply(commands, host, &mut env);
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

fn present(
    fe: Res<Frontend>,
    assets: Res<SharedAssets>,
    mut presenter: ResMut<Presenter>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut out: ResMut<UiOutput>,
    bindings: Res<Bindings>,
    input: Res<InputPresentation>,
) {
    let screen = Screen { width: window.width(), height: window.height(), pixels_per_point: window.scale_factor() };
    // The presenter puts what it draws below what is already there, so the top screen goes first.
    let device = fe.script.as_ref().and_then(|s| s.device).unwrap_or(input.device);
    for tree in fe.screens.presented_trees(&bindings, device).iter().rev() {
        presenter.0.present(tree, &assets.0, screen, &mut out);
    }
}
