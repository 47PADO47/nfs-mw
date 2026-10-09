//! The Bevy plugin: loads the HUD, copies the scene's state into it each frame, and presents it.

use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::*;
use bevy_time::Time;
use bevy_window::{PrimaryWindow, Window};
use blackbox_feng::{PackageId, Runtime};
use game_install::GameDir;

use super::bind::HudBinding;
use super::minimap::MinimapBinding;
use super::state::{HudState, MapPosition};
use super::viewport::HudViewport;
use crate::app::{FrameSet, Host};
use crate::gui::UiOutput;
use crate::settings::Settings;
use crate::ui::present::Screen;
use crate::ui::{Catalog, Presenter, SharedAssets};

/// Longest step the HUD clock takes, so a stall does not skip animations.
const MAX_STEP: f32 = 0.1;

/// The package that is the in-game HUD, and the files that hold it.
const HUD_PACKAGE: &str = "HUD_SingleRace.fng";
const HUD_FILES: [&str; 2] = ["GLOBAL/InGameB.bun", "GLOBAL/INGAMEC.BUN"];

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
    viewport: HudViewport,
}

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        let Some(assets) = crate::ui::ensure(app, &self.dir) else { return };
        let catalog = Catalog::load(&self.dir, &HUD_FILES);
        let Some(package) = catalog.find(HUD_PACKAGE).cloned() else {
            log::warn!("the HUD is off: the install has no {HUD_PACKAGE}");
            return;
        };
        log::info!(
            "HUD: {} objects, {} resources without a texture",
            package.objects.len(),
            assets.missing_resources(&package).len()
        );
        let mut runtime = Runtime::new();
        if let Some(strings) = assets.strings.clone() {
            runtime.set_string_resolver(move |label| strings.get(label));
        }
        let package = runtime.load(package);
        let minimap = MinimapBinding::open_city(&runtime, package, &self.dir, &assets);
        let binding = HudBinding::new(&mut runtime, package).with_minimap(minimap);
        let viewport = HudViewport::new(&runtime.tree(package));
        app.insert_resource(Hud { runtime, package, binding, viewport })
            .insert_resource(self.initial.clone())
            .add_systems(Update, sync.in_set(FrameSet::SceneUpdate))
            .add_systems(Update, present.in_set(FrameSet::Hud));
    }
}

/// Take the state the scene wants shown. A scene without telemetry leaves the idle HUD (`--hud` in a viewer).
/// The minimap setting decides whether the scene's map position is shown and how the picture is turned.
fn sync(host: NonSend<Host>, settings: Res<Settings>, mut state: ResMut<HudState>) {
    if let Some(s) = host.scene.hud_state() {
        *state = s;
    }
    let orientation = settings.minimap.orientation();
    state.minimap = state.minimap.zip(orientation).map(|(at, orientation)| MapPosition { orientation, ..at });
}

#[allow(clippy::too_many_arguments)]
fn present(
    mut hud: ResMut<Hud>,
    state: Res<HudState>,
    settings: Res<Settings>,
    time: Res<Time>,
    assets: Res<SharedAssets>,
    mut presenter: ResMut<Presenter>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut out: ResMut<UiOutput>,
) {
    if !state.visible || !settings.hud {
        return;
    }
    let hud = &mut *hud;
    hud.binding.apply(&mut hud.runtime, &state);
    hud.runtime.update(time.delta_secs().min(MAX_STEP));
    let _ = hud.runtime.take_outgoing();
    let mut tree = hud.runtime.tree(hud.package);
    let screen = Screen { width: window.width(), height: window.height(), pixels_per_point: window.scale_factor() };
    hud.viewport.apply(&mut tree, screen, settings.hud_layout);
    presenter.0.present(&tree, &assets.0, screen, &mut out);
}
