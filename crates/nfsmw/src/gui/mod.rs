//! The immediate-mode UI host (egui): turns Bevy's input into egui events, and egui's output into the
//! renderer-neutral [`UiLayer`](blackbox_gfx::UiLayer) that the render bridge draws.
//!
//! Nothing here draws or touches the renderer. The panels (metrics, console) live in `devtools` and only
//! see an `egui::Context`; under a full Bevy move this host would be replaced by `bevy_egui`.

mod input;
mod keys;
mod output;
mod platform;

use bevy_app::{App, Plugin, PreUpdate};
use bevy_ecs::prelude::*;
use bevy_input::InputSystems;
use blackbox_gfx::UiMesh;

pub use input::GuiInput;
pub use output::{OwnedPatch, UiOutput};
pub(crate) use platform::Platform;

/// The egui context, shared by every panel.
#[derive(Resource, Clone, Default)]
pub struct Gui {
    pub ctx: egui::Context,
}

pub struct GuiPlugin;

impl Plugin for GuiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Gui>()
            .init_resource::<GuiInput>()
            .init_resource::<UiOutput>()
            .insert_non_send(Platform::default())
            .add_systems(PreUpdate, input::collect.after(InputSystems))
            .add_systems(bevy_app::PostUpdate, platform::apply);
    }
}

impl Gui {
    /// Run one egui pass: `input` in, `panels` draw, the result converted into `out`.
    pub fn frame(
        &self,
        raw: egui::RawInput,
        pixels_per_point: f32,
        out: &mut UiOutput,
        panels: impl FnOnce(&egui::Context),
    ) {
        self.ctx.set_pixels_per_point(pixels_per_point);
        self.ctx.begin_pass(raw);
        panels(&self.ctx);
        let full = self.ctx.end_pass();
        output::convert(&self.ctx, full, out);
    }

    /// Shapes drawn outside an egui pass (the split-screen strips, drawn over the packages): tessellated into the
    /// renderer's meshes at `pixels_per_point`.
    pub fn tessellate(&self, shapes: Vec<egui::epaint::ClippedShape>, pixels_per_point: f32) -> Vec<UiMesh> {
        output::to_meshes(self.ctx.tessellate(shapes, pixels_per_point))
    }
}
