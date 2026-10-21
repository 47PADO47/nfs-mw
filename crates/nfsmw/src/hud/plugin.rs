//! The Bevy plugin: loads the HUD, copies the scene's state into it each frame, and presents it.

use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::*;
use bevy_time::Time;
use bevy_window::{PrimaryWindow, Window};
use blackbox_feng::{PackageId, Runtime};
use blackbox_ui::ThemeId;
use game_install::GameDir;
use nfsmw_career::regions::MapVariant;

use super::bind::HudBinding;
use super::blip_styles;
use super::busted_overlay::{BustedOverlay, OVERLAY_PACKAGE};
use super::event_tiles::{EventTiles, install_loader};
use super::map_source::{MapSource, resolve_stem};
use super::minimap::MinimapBinding;
use super::radio::RadioHud;
use super::state::{HudState, MapPosition};
use super::trax::{self, TraxBinding};
use super::viewport::HudViewport;
use crate::app::{FrameSet, Host};
use crate::career::CareerHandle;
use crate::gui::UiOutput;
use crate::settings::{Settings, SpeedUnits};
use crate::ui::present::Screen;
use crate::ui::theme::original::FengSource;
use crate::ui::theme::{ElementId, UiThemes, hud_layer, slot_layer};
use crate::ui::{Catalog, Presenter, SharedAssets};

/// The package that is the in-game HUD, and the files that hold it.
const HUD_PACKAGE: &str = "HUD_SingleRace.fng";
/// The HUD of a drag race (layout number 2, `docs/specs/race-mode.md` section 6).
const DRAG_PACKAGE: &str = "HUD_Drag.fng";
const HUD_FILES: [&str; 2] = ["GLOBAL/InGameB.bun", "GLOBAL/INGAMEC.BUN"];

pub struct HudPlugin {
    pub dir: GameDir,
    /// What the HUD shows until a scene provides its own state.
    pub initial: HudState,
    /// The career the minimap's map variant follows (the same handle the front end and the world hold). Empty outside
    /// career mode, where the full map shows.
    pub career: CareerHandle,
}

#[derive(Resource)]
struct Hud {
    runtime: Runtime,
    package: PackageId,
    binding: HudBinding,
    viewport: HudViewport,
    /// The map variants whose tiles are loaded: the minimap switches only among these (`minimap::usable_variant`).
    map_variants: Vec<MapVariant>,
    /// The tiles of the running race's event, read on demand (`docs/specs/race-minimap.md`).
    event_tiles: EventTiles,
    /// The original EA Trax chyron, when the install has it.
    trax: Option<TraxBinding>,
    /// The busted overlay, when the install has it (`busted_overlay.rs`).
    overlay: Option<BustedOverlay>,
    /// The drag race HUD, when the install has it: it takes the place of the race HUD while a drag race runs.
    drag: Option<DragHud>,
}

/// `HUD_Drag.fng` with its own binding and viewport, in the same runtime.
struct DragHud {
    package: PackageId,
    binding: HudBinding,
    viewport: HudViewport,
}

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<super::RadioHud>().add_systems(Update, super::radio::update.in_set(FrameSet::SceneUpdate));
        let Some(assets) = crate::ui::ensure(app, &self.dir) else { return };
        super::split::install(app, &self.dir, &assets);
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
        let mut runtime = crate::ui::runtime();
        if let Some(strings) = assets.strings.clone() {
            runtime.set_string_resolver(move |label| strings.get(label));
        }
        let package = runtime.load(package);
        let minimap = MinimapBinding::open_city(&runtime, package, &self.dir, &assets);
        // The full map is the one `open_city` opened; the other two variants count when their tiles open too.
        let map_variants: Vec<MapVariant> = [MapVariant::Unlock1, MapVariant::Unlock2]
            .into_iter()
            .filter(|v| MinimapBinding::open_variant(&runtime, package, &self.dir, &assets, *v).is_some())
            .chain(minimap.is_some().then_some(MapVariant::Full))
            .collect();
        let mut binding = HudBinding::new(&mut runtime, package).with_minimap(minimap);
        // Before the viewport: its icons take the minimap's side with the others.
        for style in blip_styles::map_icon_styles() {
            binding.add_blip_style(&mut runtime, package, style);
        }
        let viewport = HudViewport::new(&runtime.tree(package), &binding.blip_icon_hashes());
        let trax = trax::load(&self.dir, &mut runtime);
        let overlay = catalog.find(OVERLAY_PACKAGE).cloned().map(BustedOverlay::new);
        let drag = catalog.find(DRAG_PACKAGE).cloned().map(|drag| {
            let package = runtime.load(drag);
            let binding = HudBinding::new(&mut runtime, package);
            let viewport = HudViewport::new(&runtime.tree(package), &[]);
            DragHud { package, binding, viewport }
        });
        let event_tiles = EventTiles::new(install_loader(self.dir.clone()));
        app.insert_resource(Hud {
            runtime,
            package,
            binding,
            viewport,
            map_variants,
            event_tiles,
            trax,
            overlay,
            drag,
        })
        .insert_non_send(self.career.clone())
        .insert_resource(self.initial.clone())
        .add_systems(Update, sync.in_set(FrameSet::SceneUpdate))
        .add_systems(Update, present.in_set(FrameSet::Hud));
    }
}

/// Take the state the scene wants shown. A scene without telemetry leaves the idle HUD (`--hud` in a viewer).
/// The minimap setting decides whether the scene's map position is shown and how the picture is turned. The career
/// stage decides which map the minimap shows, and a running race's own tiles take its place (`hud/map_source.rs`).
fn sync(
    host: NonSend<Host>,
    career: NonSend<CareerHandle>,
    settings: Res<Settings>,
    assets: Res<SharedAssets>,
    mut state: ResMut<HudState>,
    mut hud: ResMut<Hud>,
) {
    let _trace = blackbox_trace::scope("HUD: sync telemetry");
    match host.scene.hud_state() {
        Some(s) => *state = s,
        // A scene without a HUD (a menu after the player quit the world): no race decides the map any more.
        None => state.map_source = MapSource::Stage,
    }
    // A split-screen race draws the two players' HUDs instead (`hud/split`).
    if host.scene.split_active() {
        state.visible = false;
    }
    // The unit is the player's setting, not the scene's: every HUD look reads it from here.
    state.use_mph = settings.speed_units == SpeedUnits::Imperial;
    let hud = &mut *hud;
    hud.event_tiles.update(state.map_source.event(), &assets.0);
    let stem = resolve_stem(
        &state.map_source,
        career.with(|c| c.stage()),
        |v| hud.map_variants.contains(&v),
        hud.event_tiles.ready_stem(),
    );
    if let Some(minimap) = hud.binding.minimap_mut() {
        minimap.set_stem(stem);
    }
    if let Some(overlay) = hud.overlay.as_mut() {
        overlay.set_shown(&mut hud.runtime, state.busted_overlay);
    }
    let orientation = settings.minimap.orientation();
    state.minimap = state.minimap.zip(orientation).map(|(at, orientation)| MapPosition { orientation, ..at });
}

/// Draws the HUD through the theme composer (`ui/theme/compose.rs`). Each frame the plugin stores each source's tree in
/// the frame cell, with the transforms the presenter always got (the chyron placed, the HUD viewported, the busted
/// overlay as it is). The layers are then composed from the active theme's slots and presented in order: the radio
/// layer, the HUD, the busted overlay.
#[allow(clippy::too_many_arguments)]
fn present(
    mut hud: ResMut<Hud>,
    state: Res<HudState>,
    radio: Res<RadioHud>,
    settings: Res<Settings>,
    time: Res<Time>,
    assets: Res<SharedAssets>,
    themes: Res<UiThemes>,
    mut presenter: ResMut<Presenter>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut out: ResMut<UiOutput>,
) {
    let _trace = blackbox_trace::scope("HUD: bind / animate / compose / present");
    if !state.visible || !settings.hud {
        return;
    }
    let hud = &mut *hud;
    // The RadioCard slot decides the radio look. The EA Trax chyron plays and is drawn only where the active theme
    // leaves that slot to Original; a theme that replaces it (Custom's card) gets no chyron.
    let chyron_shown = themes.provided_by(ElementId::RadioCard) == Some(ThemeId::Original);
    // A drag race shows its own package; the other package rests (its objects are not updated or drawn).
    let drag = hud.drag.as_mut().filter(|_| state.drag);
    let (package, binding, viewport) = match drag {
        Some(drag) => (drag.package, &mut drag.binding, &drag.viewport),
        None => (hud.package, &mut hud.binding, &hud.viewport),
    };
    binding.set_mirror_shown(settings.rear_view_mirror);
    binding.advance_mirror_fade(time.delta_secs());
    binding.advance_blips(time.delta_secs());
    binding.apply(&mut hud.runtime, &state);
    if let Some(trax) = hud.trax.as_mut() {
        let song = chyron_shown.then(|| radio.song().zip(radio.serial())).flatten();
        trax.apply(&mut hud.runtime, song);
    }
    hud.runtime.update(time.delta_secs());
    binding.finish_frame(&mut hud.runtime, &state);
    if let Some(trax) = hud.trax.as_ref() {
        trax.keep_text(&mut hud.runtime);
    }
    let _ = hud.runtime.take_outgoing();
    let screen = Screen { width: window.width(), height: window.height(), pixels_per_point: window.scale_factor() };
    let frame = themes.frame();
    // The card places itself from the HUD layout, so it takes the feed, not the viewport.
    themes.radio().set(&radio, screen, settings.hud_layout);
    // The Custom speedometer and tachometer read the drive state the same way.
    themes.drive().set(&state, screen);
    match (chyron_shown, hud.trax.as_ref()) {
        (true, Some(trax)) => {
            let mut chyron = hud.runtime.tree(trax.package());
            trax.place(&mut chyron, screen, settings.hud_layout);
            frame.set(FengSource::Chyron, chyron);
        }
        _ => frame.clear(FengSource::Chyron),
    }
    // The viewport is applied to the source tree (its `index` is the position), never to the composed tree.
    let mut tree = hud.runtime.tree(package);
    viewport.apply(&mut tree, screen, settings.hud_layout);
    frame.set(FengSource::Hud, tree);
    match hud.overlay.as_ref().and_then(|overlay| overlay.tree(&hud.runtime)) {
        Some(overlay) => frame.set(FengSource::BustedOverlay, overlay),
        None => frame.clear(FengSource::BustedOverlay),
    }
    // The presenter puts each call's meshes over the ones of the calls before it, so the chyron (or the card) goes
    // first, the HUD over it, and the busted overlay last, as the presenter always drew them.
    let time_s = time.elapsed_secs();
    let layers = [
        slot_layer(&themes, ElementId::RadioCard, time_s),
        hud_layer(&themes, time_s),
        slot_layer(&themes, ElementId::BustedOverlay, time_s),
    ];
    for layer in &layers {
        if layer.nodes.is_empty() {
            continue;
        }
        presenter.0.present(layer, &assets.0, screen, &mut out);
    }
}
