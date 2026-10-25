//! The Bevy plugin: loads the HUD, copies the scene's state into it each frame, and presents it.

use std::sync::Arc;

use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::*;
use bevy_time::Time;
use bevy_window::{PrimaryWindow, Window};
use blackbox_feng::{PackageId, Runtime};
use blackbox_ui::{ThemeId, UiTree};
use game_install::GameDir;
use nfsmw_career::regions::MapVariant;

use super::MapPicture;
use super::bind::HudBinding;
use super::blip_styles;
use super::busted_overlay::{BustedOverlay, OVERLAY_PACKAGE};
use super::event_tiles::{EventTiles, install_loader};
use super::frame_cache::{FrameCache, TreeKey};
use super::map_source::{MapSource, resolve_stem};
use super::minimap::MinimapBinding;
use super::notify::{HudNotifications, intense};
use super::radio::RadioHud;
use super::state::{HudState, MapPosition};
use super::trax::{self, TraxBinding};
use super::viewport::{HudViewport, parameters};
use super::withdrawn::Drawn;
use crate::app::{FrameSet, Host};
use crate::career::CareerHandle;
use crate::gui::UiOutput;
use crate::settings::{Settings, SpeedUnits};
use crate::ui::present::Screen;
use crate::ui::theme::original::FengSource;
use crate::ui::theme::{ElementId, UiThemes, hud_layer, slot_layer};
use crate::ui::{AssetsLoad, Catalog, Job, Presenter, SharedAssets, UiAssets, UiLoading};

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
    language: crate::ui::LanguageWatch,
    package: PackageId,
    binding: HudBinding,
    viewport: HudViewport,
    /// The map variants whose tiles are loaded: the minimap switches only among these (`minimap::usable_variant`).
    map_variants: Vec<MapVariant>,
    /// The tiles of the running race's event, read on demand (`docs/specs/race-minimap.md`).
    event_tiles: EventTiles,
    /// The picture of the map file the minimap shows, shared with the Custom map; rebuilt when the file changes.
    map_picture: Option<Arc<MapPicture>>,
    /// The original EA Trax chyron, when the install has it.
    trax: Option<TraxBinding>,
    /// The busted overlay, when the install has it (`busted_overlay.rs`).
    overlay: Option<BustedOverlay>,
    /// The drag race HUD, when the install has it: it takes the place of the race HUD while a drag race runs.
    drag: Option<DragHud>,
    /// What the trees in the frame cell were built from, and the layers composed from them (`frame_cache.rs`).
    cache: FrameCache,
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
        // The HUD is built on a background thread once the UI assets are read, so the window opens at once. Its
        // systems wait for it (`ui/loading.rs`).
        let assets = crate::ui::ensure(app, &self.dir);
        let dir = self.dir.clone();
        app.world_mut().resource_mut::<UiLoading>().begin(LOAD);
        let ready = resource_exists::<Hud>;
        app.insert_resource(PendingHud(Job::spawn("hud-load", move || load(&dir, &assets))))
            .insert_non_send(self.career.clone())
            .insert_resource(self.initial.clone())
            .add_systems(Update, poll.in_set(FrameSet::Prepare).run_if(resource_exists::<PendingHud>))
            // The scene's update moves the world first, so the HUD reads the frame it draws.
            .add_systems(
                Update,
                sync.in_set(FrameSet::SceneUpdate)
                    .after(crate::app::render::update_scene)
                    .run_if(ready)
                    .run_if(resource_exists::<SharedAssets>),
            )
            // A `--hud-scenario` run writes its fixture over the scene's state (`devtools/hud_scenarios`); without the
            // flag the resource is absent and the system never runs.
            .add_systems(
                Update,
                crate::devtools::hud_scenarios::apply
                    .in_set(FrameSet::SceneUpdate)
                    .after(sync)
                    .before(notify)
                    .run_if(ready)
                    .run_if(resource_exists::<crate::devtools::hud_scenarios::ActiveScenario>),
            )
            .init_resource::<HudNotifications>()
            .add_systems(Update, notify.in_set(FrameSet::SceneUpdate).after(sync))
            .add_systems(Update, super::notify::output.in_set(FrameSet::SceneUpdate).after(notify))
            .add_systems(Update, super::map_keys::apply.in_set(FrameSet::SceneUpdate).after(sync))
            .add_systems(Update, present.in_set(FrameSet::Hud).run_if(ready).run_if(resource_exists::<SharedAssets>))
            .add_systems(
                Update,
                present_lan.in_set(FrameSet::Hud).after(present).run_if(ready).run_if(resource_exists::<SharedAssets>),
            );
        super::split::install(app);
    }
}

/// The name of the HUD's load in [`UiLoading`].
const LOAD: &str = "the HUD";

/// What the HUD's loader thread builds: the split-screen HUDs and the HUD, each `None` when the install lacks it.
#[derive(Default)]
struct Loaded {
    split: Option<super::split::SplitHuds>,
    hud: Option<Hud>,
}

/// The HUD being built on its loader thread.
#[derive(Resource)]
struct PendingHud(Job<Loaded>);

/// Reads the HUD's packages, waits for the UI assets that load beside them, and builds the HUDs over both. On the
/// loader thread.
fn load(dir: &GameDir, assets: &AssetsLoad) -> Loaded {
    let catalog = Catalog::load(dir, &HUD_FILES);
    let Some(assets) = assets.wait() else { return Loaded::default() };
    Loaded { split: super::split::load(dir, &assets), hud: build(dir, &assets, &catalog) }
}

/// The HUD of the install, `None` when it has no race HUD.
fn build(dir: &GameDir, assets: &UiAssets, catalog: &Catalog) -> Option<Hud> {
    let Some(package) = catalog.find(HUD_PACKAGE).cloned() else {
        log::warn!("the HUD is off: the install has no {HUD_PACKAGE}");
        return None;
    };
    log::info!(
        "HUD: {} objects, {} resources without a texture",
        package.objects.len(),
        assets.missing_resources(&package).len()
    );
    let mut runtime = crate::ui::runtime();
    let package = runtime.load(package);
    let minimap = MinimapBinding::open_city(&runtime, package, dir, assets);
    // The full map is the one `open_city` opened; the other two variants count when their tiles open too.
    let map_variants: Vec<MapVariant> = [MapVariant::Unlock1, MapVariant::Unlock2]
        .into_iter()
        .filter(|v| MinimapBinding::open_variant(&runtime, package, dir, assets, *v).is_some())
        .chain(minimap.is_some().then_some(MapVariant::Full))
        .collect();
    let mut binding = HudBinding::new(&mut runtime, package).with_minimap(minimap);
    // Before the viewport: its icons take the minimap's side with the others.
    for style in blip_styles::map_icon_styles() {
        binding.add_blip_style(&mut runtime, package, style);
    }
    let viewport = HudViewport::new(&runtime.tree(package), &binding.blip_icon_hashes());
    let trax = trax::load(dir, &mut runtime);
    let overlay = catalog.find(OVERLAY_PACKAGE).cloned().map(BustedOverlay::new);
    let drag = catalog.find(DRAG_PACKAGE).cloned().map(|drag| {
        let package = runtime.load(drag);
        let binding = HudBinding::new(&mut runtime, package);
        let viewport = HudViewport::new(&runtime.tree(package), &[]);
        DragHud { package, binding, viewport }
    });
    let event_tiles = EventTiles::new(install_loader(dir.clone()));
    Some(Hud {
        runtime,
        language: crate::ui::LanguageWatch::default(),
        package,
        binding,
        viewport,
        map_variants,
        event_tiles,
        map_picture: None,
        trax,
        overlay,
        drag,
        cache: FrameCache::default(),
    })
}

/// Puts the HUDs in place once they are built.
fn poll(mut commands: Commands, mut pending: ResMut<PendingHud>, mut loading: ResMut<UiLoading>) {
    let Some(loaded) = pending.0.take() else { return };
    commands.remove_resource::<PendingHud>();
    loading.end(LOAD);
    if let Some(split) = loaded.split {
        commands.insert_resource(split);
    }
    if let Some(hud) = loaded.hud {
        commands.insert_resource(hud);
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
    windows: Query<&Window, With<PrimaryWindow>>,
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
    // World-space markers project into the window the HUD is drawn over, in the presenter's logical pixels.
    if let (Some(camera), Ok(window)) = (state.camera.as_mut(), windows.single()) {
        camera.viewport = [window.width(), window.height()];
    }
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
        if !hud.map_picture.as_ref().is_some_and(|picture| picture.stem() == minimap.stem()) {
            hud.map_picture = Some(Arc::new(minimap.picture()));
        }
    }
    state.map_picture = hud.map_picture.clone();
    if let Some(overlay) = hud.overlay.as_mut() {
        overlay.set_shown(&mut hud.runtime, state.busted_overlay);
    }
    // The Custom map follows the car whatever the minimap setting says, so it keeps the place before the setting hides it.
    state.map_car = state.minimap;
    let orientation = settings.minimap.orientation();
    state.minimap = state.minimap.zip(orientation).map(|(at, orientation)| MapPosition { orientation, ..at });
}

/// Feeds the frame's HUD events to the notifications at the player's `[custom_hud] notifications` level, ages them, and
/// takes the dismiss action.
fn notify(
    state: Res<HudState>,
    settings: Res<Settings>,
    time: Res<Time>,
    actions: Option<Res<crate::input::ActionState>>,
    mut notifications: ResMut<HudNotifications>,
) {
    notifications.take_events(&state, settings.hud_options.notifications);
    notifications.tick(time.delta_secs(), intense(&state));
    if actions.is_some_and(|actions| actions.just_pressed(crate::input::Action::DismissNotification)) {
        notifications.dismiss_oldest();
    }
}

/// The LAN session's feed, vote, countdown and spectator panel (`frontend/lan`). They are the session's, not the car's HUD:
/// they show while the HUD is hidden (the front end, a spectator without a car, the HUD option off) and change every frame,
/// so they are not cached with the HUD's layers.
fn present_lan(
    assets: Res<SharedAssets>,
    themes: Res<UiThemes>,
    time: Res<Time>,
    mut presenter: ResMut<Presenter>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut out: ResMut<UiOutput>,
) {
    if themes.frame().lan().hud().is_none() {
        return;
    }
    let screen = Screen { width: window.width(), height: window.height(), pixels_per_point: window.scale_factor() };
    let layer = slot_layer(&themes, ElementId::LanHud, time.elapsed_secs());
    if !layer.nodes.is_empty() {
        presenter.0.present(&layer, &assets.0, screen, &mut out);
    }
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
    notifications: Res<HudNotifications>,
    settings: Res<Settings>,
    time: Res<Time>,
    assets: Res<SharedAssets>,
    themes: Res<UiThemes>,
    mut presenter: ResMut<Presenter>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut out: ResMut<UiOutput>,
    mut drawn: Local<Drawn>,
    (bindings, input): (Option<Res<crate::input::Bindings>>, Option<Res<crate::input::InputPresentation>>),
) {
    let _trace = blackbox_trace::scope("HUD: bind / animate / compose / present");
    // A theme that fills the banner with its own component draws the world's cards too, so the world's egui cards stay
    // out (`cards.rs`).
    let routed =
        !themes.hud_is_static() && themes.provided_by(ElementId::Banner).is_some_and(|id| id != ThemeId::Original);
    super::set_cards_routed(routed);
    if !state.visible || !settings.hud {
        drawn.hidden(&mut out, window.scale_factor());
        return;
    }
    drawn.shown();
    let hud = &mut *hud;
    // The RadioCard slot decides the radio look. The EA Trax chyron plays and is drawn only where the active theme
    // leaves that slot to Original; a theme that replaces it (Custom's card) gets no chyron.
    // Under the Original theme every slot is Original's, so no slot has to be resolved to know it.
    let static_hud = themes.hud_is_static();
    let chyron_shown = static_hud || themes.provided_by(ElementId::RadioCard) == Some(ThemeId::Original);
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
    hud.language.apply(&mut hud.runtime);
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
    // The Remastered cluster reads the drive state the same way.
    themes.drive().set(&state, screen);
    // The Custom HUD reads one richer feed: the frame's state (cloned once here, shared after), the options and the
    // clock. Nothing reads it under Original, so it is not filled there.
    if !static_hud {
        themes.custom().set(crate::ui::theme::custom::CustomFeed {
            hud: std::sync::Arc::new(HudState::clone(&state)),
            settings: settings.hud_options,
            units: settings.speed_units,
            transmission: settings.transmission,
            elapsed_s: time.elapsed_secs(),
            delta_s: time.delta_secs(),
            screen,
            // The notices and the feedback lane, their text in the current language. Motion reduction shows the
            // bounty delta's sum at once instead of counting it up.
            notifications: Some(notifications.view(settings.speed_units, !settings.hud_options.motion_reduction)),
            interact: bindings.as_deref().map(|bindings| {
                let device = input.as_deref().map(|input| input.device).unwrap_or_default();
                crate::ui::prompts::for_action(bindings, crate::input::Action::Interact, device)
            }),
        });
    }
    // A tree is rebuilt only when its package changed since the frame it was built for, or the viewport moved; the
    // cell then keeps its generation and the composed layers below are reused as well.
    let placement = parameters(screen, settings.hud_layout);
    let key = |package: PackageId, runtime: &Runtime| TreeKey::new(package, runtime.revision(package), placement);
    match (chyron_shown, hud.trax.as_ref()) {
        (true, Some(trax)) => {
            let chyron = trax.package();
            hud.cache.refresh(&frame, FengSource::Chyron, key(chyron, &hud.runtime), |tree| {
                hud.runtime.tree_into(chyron, tree);
                trax.place(tree, screen, settings.hud_layout);
            });
        }
        _ => hud.cache.clear(&frame, FengSource::Chyron),
    }
    // The viewport is applied to the source tree (its `index` is the position), never to the composed tree.
    hud.cache.refresh(&frame, FengSource::Hud, key(package, &hud.runtime), |tree| {
        hud.runtime.tree_into(package, tree);
        viewport.apply(tree, screen, settings.hud_layout);
    });
    match hud.overlay.as_ref().and_then(BustedOverlay::shown) {
        Some(overlay) => {
            let key = TreeKey::new(overlay, hud.runtime.revision(overlay), (1.0, 0.0));
            hud.cache.refresh(&frame, FengSource::BustedOverlay, key, |tree| hud.runtime.tree_into(overlay, tree));
        }
        None => hud.cache.clear(&frame, FengSource::BustedOverlay),
    }
    // The presenter puts each call's meshes over the ones of the calls before it, so the chyron (or the card) goes
    // first, the HUD over it, and the busted overlay last, as the presenter always drew them.
    let time_s = time.elapsed_secs();
    // Under the Original theme the composed HUD layer is the HUD tree of the cell node for node (`compose_tests.rs`;
    // only the `parent` links of the slot roots differ, and nothing draws from them), so it is presented as it is and
    // the layers kept here hold an empty one. The other themes compose theirs every frame.
    let source = static_hud.then(|| frame.get(FengSource::Hud)).flatten();
    let layers = hud.cache.layers(frame.generation(), static_hud, || {
        [
            slot_layer(&themes, ElementId::RadioCard, time_s),
            if static_hud { UiTree::default() } else { hud_layer(&themes, time_s) },
            slot_layer(&themes, ElementId::BustedOverlay, time_s),
        ]
    });
    let [radio_layer, hud_composed, busted_layer] = layers;
    let hud_tree = source.as_deref().or_else(|| (!static_hud).then_some(hud_composed));
    for layer in [Some(radio_layer), hud_tree, Some(busted_layer)].into_iter().flatten() {
        if layer.nodes.is_empty() {
            continue;
        }
        presenter.0.present(layer, &assets.0, screen, &mut out);
    }
}
