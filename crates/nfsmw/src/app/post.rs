//! Hands the post-processing settings (tone mapping, bloom, anti-aliasing) to the renderer.

use bevy_ecs::prelude::*;

use super::host::Host;
use crate::settings::{Settings, post_effects};

/// Every frame, so the effects follow the menu, the console and the first frame after the renderer
/// exists; the renderer ignores a value it already runs.
pub fn apply(mut host: NonSendMut<Host>, settings: Res<Settings>) {
    let Some(renderer) = host.renderer.as_mut() else { return };
    renderer.set_post_effects(post_effects(&settings));
}
