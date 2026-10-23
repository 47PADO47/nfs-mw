//! The Bevy side of the split-screen HUDs: loading them with the HUD plugin, and the system that, while the scene says a
//! split-screen race is on, feeds the two players' states in and draws their two HUDs through the shared presenter. The
//! single-player HUD rests in that time (`hud/plugin.rs` hides it).

use bevy_app::{App, Update};
use bevy_ecs::prelude::*;
use bevy_time::Time;
use bevy_window::{PrimaryWindow, Window};
use blackbox_gfx::UiLayer;
use game_install::GameDir;

use super::huds::SplitHuds;
use super::pursuit;
use crate::app::{FrameSet, Host};
use crate::gui::{Gui, UiOutput};
use crate::hud::state::HudState;
use crate::hud::withdrawn::Drawn;
use crate::settings::{Settings, SpeedUnits};
use crate::ui::present::Screen;
use crate::ui::{Presenter, SharedAssets, UiAssets};

/// Adds the system that draws the split-screen HUDs. It waits for them: the HUD's loader thread builds them ([`load`]).
pub fn install(app: &mut App) {
    app.add_systems(
        Update,
        present.in_set(FrameSet::Hud).run_if(resource_exists::<SplitHuds>).run_if(resource_exists::<SharedAssets>),
    );
}

/// Loads the split-screen HUDs, once the shared assets exist. `None` when the install lacks them.
pub fn load(dir: &GameDir, assets: &UiAssets) -> Option<SplitHuds> {
    SplitHuds::load(dir, assets)
}

/// The state of each player's HUD for this frame; `None` while no split-screen race is on. A player the scene gives no
/// state to has a hidden HUD.
fn states(host: &Host, settings: &Settings) -> Option<[HudState; 2]> {
    if !host.scene.split_active() {
        return None;
    }
    let state = |player: usize| {
        let mut state = host.scene.split_hud_state(player).unwrap_or(HudState { visible: false, ..Default::default() });
        // The unit is the player's setting, not the scene's.
        state.use_mph = settings.speed_units == SpeedUnits::Imperial;
        state
    };
    Some([state(0), state(1)])
}

/// Draws both HUDs. The presenter puts its meshes in front of the frame's UI layer, so a picture the app draws under the
/// HUDs (the two half-screen views) has to be put at the front of the layer after this system (`FrameSet::Hud`).
#[allow(clippy::too_many_arguments)]
fn present(
    host: NonSend<Host>,
    settings: Res<Settings>,
    time: Res<Time>,
    assets: Res<SharedAssets>,
    mut huds: ResMut<SplitHuds>,
    mut presenter: ResMut<Presenter>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut out: ResMut<UiOutput>,
    gui: Res<Gui>,
    mut drawn: Local<Drawn>,
) {
    let _trace = blackbox_trace::scope("Split HUD: bind / animate / present");
    let Some(states) = states(&host, &settings).filter(|_| settings.hud) else {
        drawn.hidden(&mut out, window.scale_factor());
        return;
    };
    drawn.shown();
    let screen = Screen { width: window.width(), height: window.height(), pixels_per_point: window.scale_factor() };
    let Some(trees) = huds.frame(&states, time.delta_secs(), &assets.0, screen, settings.hud_layout) else { return };
    // The presenter puts each call under the earlier ones, so the first player's (with the minimap that crosses the rule)
    // goes first and ends over the second's.
    for tree in trees.iter().flatten() {
        presenter.0.present(tree, &assets.0, screen, &mut out);
    }
    // The chase strips go over the packages: appended, so drawn after them.
    let strips = gui.tessellate(pursuit::shapes(&states, screen), screen.pixels_per_point);
    let layer =
        out.layer.get_or_insert_with(|| UiLayer { pixels_per_point: screen.pixels_per_point, ..Default::default() });
    layer.meshes.extend(strips);
}
