//! Developer tools: the performance overlay now, the F12 console next. Their state is plain data; the
//! panels that draw it are thin egui views, so a full Bevy move changes the views, not the tools.

mod console;
mod logbuf;
mod logfile;
mod metrics;
mod overlay;
mod readout;

use bevy_app::{App, Last, Plugin, Update};
use bevy_ecs::prelude::*;
use bevy_time::{Real, Time};
use bevy_window::{PrimaryWindow, Window};

use crate::gui::{Gui, GuiInput, UiOutput};
use crate::settings::Settings;
pub use logbuf::install as install_logging;
use metrics::Metrics;
pub use metrics::ShowMetrics;
pub use readout::ShowReadout;

pub use console::{Console, start as start_console};

pub struct DevToolsPlugin;

impl Plugin for DevToolsPlugin {
    fn build(&self, app: &mut App) {
        console::build(app);
        app.init_resource::<Metrics>()
            .init_resource::<crate::garage::Overlay>()
            .add_systems(Update, run_ui.in_set(crate::app::FrameSet::Ui))
            .add_systems(Last, metrics::collect);
    }
}

/// Run the UI pass: feed egui this frame's input, draw the panels, publish the output.
#[allow(clippy::too_many_arguments)]
fn run_ui(
    gui: Res<Gui>,
    mut input: ResMut<GuiInput>,
    mut out: ResMut<UiOutput>,
    window: Single<&Window, With<PrimaryWindow>>,
    time: Res<Time<Real>>,
    mut settings: ResMut<Settings>,
    metrics: Res<Metrics>,
    mut console: ResMut<Console>,
    hud: Option<Res<crate::hud::HudState>>,
    radio: Option<Res<crate::hud::RadioHud>>,
    mut host: NonSendMut<crate::app::Host>,
    mut garage: ResMut<crate::garage::Overlay>,
    mut capture: ResMut<crate::input::MouseCapture>,
    mut focus: ResMut<crate::input::UiFocus>,
) {
    let (width, height) = (window.width(), window.height());
    let raw = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, height))),
        time: Some(time.elapsed_secs_f64()),
        predicted_dt: time.delta_secs(),
        events: std::iter::once(egui::Event::ModifiersChanged(input.modifiers))
            .chain(std::mem::take(&mut input.events))
            .collect(),
        focused: window.focused,
        ..Default::default()
    };
    gui.frame(raw, window.scale_factor(), &mut out, |ctx| {
        // The radio's card belongs to the HUD: it shows when the HUD does, and is placed by the HUD layout. The EA
        // Trax chyron is drawn by the HUD itself, so the card only shows in the custom style.
        if let (Some(radio), Some(hud)) = (&radio, &hud)
            && hud.visible
            && settings.hud
            && settings.radio_hud == crate::settings::RadioHudStyle::Custom
        {
            let screen = crate::ui::present::Screen { width, height, pixels_per_point: window.scale_factor() };
            crate::hud::show_radio(ctx, radio, screen, settings.hud_layout);
        }
        // The scene's panels first: the metrics sit beside them, so they know how far in to go.
        let left = host.scene.ui(ctx);
        overlay::show(ctx, settings.show_metrics, &metrics, left);
        console::show(ctx, &mut console);
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
}
