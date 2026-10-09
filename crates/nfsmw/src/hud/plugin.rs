//! The Bevy plugin: loads the HUD, copies the scene's state into it each frame, and presents it.

use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::*;
use bevy_time::Time;
use bevy_window::{PrimaryWindow, Window};
use blackbox_feng::{PackageId, Runtime};
use game_install::GameDir;

use super::bind::HudBinding;
use super::minimap::MinimapBinding;
use super::radio::RadioHud;
use super::state::{HudState, MapPosition};
use super::trax::{self, TraxBinding};
use super::viewport::HudViewport;
use crate::app::{FrameSet, Host};
use crate::gui::UiOutput;
use crate::settings::{RadioHudStyle, Settings};
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
    /// The original EA Trax chyron, when the install has it.
    trax: Option<TraxBinding>,
}

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<super::RadioHud>().add_systems(Update, super::radio::update.in_set(FrameSet::SceneUpdate));
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
        let trax = trax::load(&self.dir, &mut runtime);
        app.insert_resource(Hud { runtime, package, binding, viewport, trax })
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
    radio: Res<RadioHud>,
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
    let ea_trax = settings.radio_hud == RadioHudStyle::EaTrax;
    hud.binding.apply(&mut hud.runtime, &state);
    if let Some(trax) = hud.trax.as_mut() {
        let song = ea_trax.then(|| radio.song().zip(radio.serial())).flatten();
        trax.apply(&mut hud.runtime, song);
    }
    hud.runtime.update(time.delta_secs().min(MAX_STEP));
    if let Some(trax) = hud.trax.as_ref() {
        trax.keep_text(&mut hud.runtime);
    }
    let _ = hud.runtime.take_outgoing();
    let screen = Screen { width: window.width(), height: window.height(), pixels_per_point: window.scale_factor() };
    // The presenter puts each call's meshes in front of the ones of the calls before it, so the chyron is drawn
    // first and the HUD after it: the chyron then sits on top of the gauges.
    if let (true, Some(trax)) = (ea_trax, hud.trax.as_ref()) {
        let chyron = hud.runtime.tree(trax.package());
        presenter.0.present(&chyron, &assets.0, screen, &mut out);
    }
    let mut tree = hud.runtime.tree(hud.package);
    hud.viewport.apply(&mut tree, screen, settings.hud_layout);
    presenter.0.present(&tree, &assets.0, screen, &mut out);
}
