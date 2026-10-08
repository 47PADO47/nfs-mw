//! The Bevy plugin: loads the HUD, copies the scene's state into it each frame, and presents it.

use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::*;
use bevy_time::Time;
use bevy_window::{PrimaryWindow, Window};
use blackbox_feng::{PackageId, Runtime};
use game_install::GameDir;

use super::assets::HudAssets;
use super::bind::HudBinding;
use super::present::{BlackboxPresenter, Screen};
use super::state::HudState;
use crate::app::{FrameSet, Host};
use crate::gui::UiOutput;

/// Longest step the HUD clock takes, so a stall does not skip animations.
const MAX_STEP: f32 = 0.1;

pub struct HudPlugin {
    pub dir: GameDir,
    /// What the HUD shows until a scene provides its own state.
    pub initial: HudState,
}

#[derive(Resource)]
struct Hud {
    runtime: Runtime,
    package: PackageId,
    binding: HudBinding,
    assets: HudAssets,
    presenter: BlackboxPresenter,
}

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        let assets = match HudAssets::load(&self.dir) {
            Ok(a) => a,
            Err(e) => {
                log::warn!("the HUD is off: {e:#}");
                return;
            }
        };
        let mut runtime = Runtime::new();
        if let Some(strings) = assets.strings.clone() {
            runtime.set_string_resolver(move |label| strings.get(label));
        }
        let package = runtime.load(assets.package.clone());
        let binding = HudBinding::new(&mut runtime, package);
        app.insert_resource(Hud { runtime, package, binding, assets, presenter: BlackboxPresenter::default() })
            .insert_resource(self.initial.clone())
            .add_systems(Update, sync.in_set(FrameSet::SceneUpdate))
            .add_systems(Update, present.in_set(FrameSet::Hud));
    }
}

/// Take the state the scene wants shown. A scene without telemetry leaves the idle HUD (`--hud` in a viewer).
fn sync(host: NonSend<Host>, mut state: ResMut<HudState>) {
    if let Some(s) = host.scene.hud_state() {
        *state = s;
    }
}

fn present(
    mut hud: ResMut<Hud>,
    state: Res<HudState>,
    time: Res<Time>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut out: ResMut<UiOutput>,
) {
    if !state.visible {
        return;
    }
    let hud = &mut *hud;
    hud.binding.apply(&mut hud.runtime, &state);
    hud.runtime.update(time.delta_secs().min(MAX_STEP));
    let _ = hud.runtime.take_outgoing();
    let tree = hud.runtime.tree(hud.package);
    let screen = Screen { width: window.width(), height: window.height(), pixels_per_point: window.scale_factor() };
    hud.presenter.present(&tree, &hud.assets, screen, &mut out);
}
