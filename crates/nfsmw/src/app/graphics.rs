//! Hands the graphics settings to the renderer, and keeps what it made of them (docs/renderers.md).
//!
//! The settings are a *request*. The renderer maps it onto its capabilities with `blackbox_gfx::resolve`, and what
//! it could not honour comes back as [`Resolved`] downgrades and notes. They are logged once when they change and
//! kept here for the console's `gfx` command.

use bevy_ecs::prelude::*;
use blackbox_gfx::{GraphicsSettings, RenderBackend, Resolved};
use log::Level;

use super::host::Host;
use crate::settings::Settings;

/// The request the renderer last got, and what it made of it.
#[derive(Default)]
pub struct Applied {
    requested: Option<GraphicsSettings>,
    resolved: Option<Resolved>,
}

impl Applied {
    /// Give the renderer the graphics settings. A request it already has is not sent again. Returns whether it
    /// was sent. The renderer also sharpens the world's textures to match a lower resolution.
    pub fn apply(&mut self, renderer: &mut dyn RenderBackend, settings: &Settings) -> bool {
        self.apply_with(settings.graphics(), |request| renderer.apply_graphics(request))
    }

    fn apply_with(&mut self, request: GraphicsSettings, send: impl FnOnce(&GraphicsSettings) -> Resolved) -> bool {
        if self.requested == Some(request) {
            return false;
        }
        let resolved = send(&request);
        for (level, line) in change_lines(self.resolved.as_ref(), &resolved) {
            log::log!(level, "graphics: {line}");
        }
        self.requested = Some(request);
        self.resolved = Some(resolved);
        true
    }

    /// What the renderer made of the last request: the effective settings, the downgrades and the notes.
    pub fn resolved(&self) -> Option<&Resolved> {
        self.resolved.as_ref()
    }
}

/// The lines to log when `after` replaces `before`: every downgrade and note, once, when the set changes. Nothing
/// when it is unchanged, and a line saying so when a request that was downgraded is honoured again.
fn change_lines(before: Option<&Resolved>, after: &Resolved) -> Vec<(Level, String)> {
    let same = |a: &Resolved, b: &Resolved| a.downgrades == b.downgrades && a.notes == b.notes;
    let nothing = after.downgrades.is_empty() && after.notes.is_empty();
    match before {
        Some(before) if same(before, after) => return Vec::new(),
        None if nothing => return Vec::new(),
        Some(_) if nothing => return vec![(Level::Info, "every requested setting is in effect".to_owned())],
        _ => {}
    }
    let downgrades = after.downgrades.iter().map(|d| (Level::Warn, d.to_string()));
    downgrades.chain(after.notes.iter().map(|n| (Level::Info, n.to_string()))).collect()
}

/// Every frame, so the effects follow the menu, the console and the first frame after the renderer exists;
/// a request the renderer already has costs one comparison.
pub fn apply(mut host: NonSendMut<Host>, settings: Res<Settings>) {
    host.apply_graphics(&settings);
}

#[cfg(test)]
#[path = "graphics_tests.rs"]
mod tests;
