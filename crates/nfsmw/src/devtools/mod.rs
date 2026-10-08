//! Developer tools: the performance overlay now, the F12 console next. Their state is plain data; the
//! panels that draw it are thin egui views, so a full Bevy move changes the views, not the tools.

mod console;
mod logbuf;
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
    settings: Res<Settings>,
    metrics: Res<Metrics>,
    mut console: ResMut<Console>,
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
        overlay::show(ctx, settings.show_metrics, &metrics);
        console::show(ctx, &mut console);
    });
}
