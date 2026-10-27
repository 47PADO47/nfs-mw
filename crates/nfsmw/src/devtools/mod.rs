//! Developer tools: the performance overlay now, the F12 console next. Their state is plain data; the
//! panels that draw it are thin egui views, so a full Bevy move changes the views, not the tools.

pub(crate) mod car_list;
mod console;
mod extra_options;
mod graphics_lab;
pub(crate) mod hud_scenarios;
mod logbuf;
mod logfile;
mod metrics;
mod overlay;
mod readout;
mod theme_style;
pub(crate) mod trace;
mod ui_gate;

use bevy_app::{App, Last, Plugin, Update};
use bevy_ecs::prelude::*;
use bevy_time::{Real, Time};
use bevy_window::{PrimaryWindow, Window};

use crate::gui::{Gui, GuiInput, UiOutput};
use crate::settings::Settings;
pub(crate) use graphics_lab::GraphicsLab;
pub use logbuf::install as install_logging;
use metrics::Metrics;
pub use metrics::ShowMetrics;
pub use readout::ShowReadout;

pub use console::{Console, start as start_console};

pub struct DevToolsPlugin;

impl Plugin for DevToolsPlugin {
    fn build(&self, app: &mut App) {
        console::build(app);
        extra_options::build(app);
        graphics_lab::build(app);
        trace::build(app);
        app.init_resource::<Metrics>()
            .init_resource::<crate::garage::Overlay>()
            .add_systems(Update, run_ui.in_set(crate::app::FrameSet::Ui))
            .add_systems(Last, metrics::collect.before(crate::app::pacing::end_of_frame));
    }
}

/// The LAN menu's state, when the front end has one.
type LanParams<'w> =
    (Option<ResMut<'w, crate::frontend::lan::LanUi>>, Option<NonSendMut<'w, crate::frontend::lan::LanBackend>>);

/// Run the UI pass: feed egui this frame's input, draw the panels, publish the output.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)] // Bevy takes at most 16 system parameters, so the last ones are one tuple
fn run_ui(
    gui: Res<Gui>,
    themes: Res<crate::ui::theme::UiThemes>,
    mut input: ResMut<GuiInput>,
    mut out: ResMut<UiOutput>,
    window: Single<&Window, With<PrimaryWindow>>,
    time: Res<Time<Real>>,
    mut settings: ResMut<Settings>,
    (mut lab, mut extra): (ResMut<graphics_lab::GraphicsLab>, ResMut<extra_options::Overlay>),
    metrics: Res<Metrics>,
    mut console: ResMut<Console>,
    mut host: NonSendMut<crate::app::Host>,
    mut garage: ResMut<crate::garage::Overlay>,
    mut capture: ResMut<crate::input::MouseCapture>,
    mut focus: ResMut<crate::input::UiFocus>,
    platform: NonSend<crate::gui::Platform>,
    // Bevy takes at most 16 system parameters: the pass's own memory and the LAN menu's state are one tuple.
    (mut styled_for, mut settled, mut skipped, mut lan, scenario): (
        Local<Option<blackbox_ui::Skin>>,
        Local<ui_gate::Settled>,
        Local<bool>,
        LanParams<'_>,
        Option<Res<hud_scenarios::ActiveScenario>>,
    ),
) {
    let _trace = blackbox_trace::scope("UI: build panels");
    // A frame with nothing egui-drawn up skips the whole pass (run, tessellation, conversion, output). The pass that
    // follows the last panel clears the renderer's layer; after it nothing is handed over until a panel is up again.
    let visible = ui_gate::Visible {
        scene: host.scene.wants_ui(),
        console: console.open,
        garage: garage.open,
        graphics_lab: lab.open
            || extra.open
            || host.scene.extra_options().is_some_and(|s| s.testing.active() || s.restore_environment.is_some()),
        metrics: settings.show_metrics != ShowMetrics::Off,
        readout: !metrics.hud.is_empty(),
        trace: trace::indicator_up(&lab.recorder),
        platform_feedback: platform.visible(),
        scenario: scenario.is_some(),
    };
    if !ui_gate::egui_needed(visible, *settled) {
        // No panel could take the events: drop them rather than let them pile up.
        input.events.clear();
        *skipped = true;
        // What the lab's `show` does to the lighting options whether it is open or not.
        if let Some(clean) = graphics_lab::out_of_range_lighting(&settings) {
            settings.appearance.lighting = clean;
        }
        return;
    }
    let (width, height) = (window.width(), window.height());
    let mut events = vec![egui::Event::ModifiersChanged(input.modifiers)];
    // After skipped frames egui has not seen the pointer move: tell it where it is, so a panel that opens under it
    // hovers at once.
    if std::mem::take(&mut *skipped) {
        events.extend(input.pointer.map(egui::Event::PointerMoved));
    }
    events.append(&mut input.events);
    let raw = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, height))),
        time: Some(time.elapsed_secs_f64()),
        predicted_dt: time.delta_secs(),
        events,
        focused: window.focused,
        ..Default::default()
    };
    // The panels draw in the active theme's palette, radius, padding and text size, as the game does. The style is
    // built and set only when the skin it comes from changes, not every frame.
    if styled_for.as_ref() != Some(themes.skin()) {
        gui.ctx.set_global_style(theme_style::style_from(themes.skin()));
        *styled_for = Some(themes.skin().clone());
    }
    let (lan_ui, lan_backend) = (&mut lan.0, &mut lan.1);
    gui.frame(raw, window.scale_factor(), &mut out, |ctx| {
        let actions = match host.scene.extra_options() {
            Some(state) => crate::extra_options::view::show(ctx, &mut extra.open, state, &mut settings),
            None => {
                if extra.open {
                    egui::Window::new("Extra options · F5").open(&mut extra.open).show(ctx, |ui| {
                        ui.label("Start driving to use camera and world tools.");
                    });
                }
                Vec::new()
            }
        };
        for action in actions {
            let host = &mut *host;
            if let Some(renderer) = host.renderer.as_mut() {
                let result = host.scene.apply_extra_option(renderer.as_mut(), action);
                if let Some(state) = host.scene.extra_options() {
                    state.message = Some(match result {
                        Ok(()) => "Applied".into(),
                        Err(e) => format!("{e:#}"),
                    });
                }
            }
        }
        if !extra.open
            && let Some(state) = host.scene.extra_options()
            && state.testing.active()
            && !state.hide_ui
        {
            egui::Area::new("extra_overrides_indicator".into()).anchor(egui::Align2::RIGHT_TOP, [-12.0, 12.0]).show(
                ctx,
                |ui| {
                    if ui.button("Testing overrides active · F5").clicked() {
                        extra.open = true;
                    }
                },
            );
        }
        // The scene's panels first: the metrics sit beside them, so they know how far in to go.
        let left = host.scene.ui(ctx);
        let net = lan_ui.as_deref_mut().and_then(|ui| crate::frontend::lan::stats_line(ui, lan_backend.as_deref_mut()));
        let net = overlay::net_line_shown(settings.show_metrics, settings.net.show_stats, net);
        overlay::show(ctx, settings.show_metrics, &metrics, left, net.as_deref());
        trace::indicator(ctx, &lab.recorder);
        platform.show(ctx);
        if let Some(scenario) = scenario.as_deref() {
            hud_scenarios::label(ctx, scenario.0);
        }
        console::show(ctx, &mut console);
        graphics_lab::show_world(ctx, &mut lab, &mut settings, &metrics, host.scene.as_mut());
        if garage.open {
            let mut open = true;
            let request =
                host.scene.garage().and_then(|model| crate::garage::show(ctx, &mut open, model, &mut settings));
            if let Some(selection) = request {
                let host = &mut *host;
                if let Some(renderer) = host.renderer.as_mut() {
                    let result = host
                        .scene
                        .apply_garage(renderer.as_mut(), selection)
                        .map(|_| "Car and paint applied".to_owned())
                        .map_err(|e| format!("{e:#}"));
                    if let Some(model) = host.scene.garage() {
                        model.message = Some(result);
                    }
                }
            }
            if !open {
                garage.set_open(false, &mut capture, &mut focus);
            }
        }
    });
    if host.scene.screenshot_ui_hidden() {
        extra.open = false;
    }
    extra.sync(&mut capture, &mut focus);
    let drew_nothing = out.layer.as_ref().is_some_and(|layer| layer.meshes.is_empty());
    *settled = ui_gate::Settled::after_pass(
        drew_nothing,
        gui.ctx.egui_wants_pointer_input(),
        gui.ctx.egui_wants_keyboard_input(),
    );
}
