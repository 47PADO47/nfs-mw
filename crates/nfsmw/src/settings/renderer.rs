//! The `renderer` setting: which renderer draws the game (docs/renderers.md). It is not the `backend`
//! setting, which picks the graphics API (Vulkan, DX12, OpenGL) the renderer uses.

/// Who draws the frame. Changing it takes effect at the next start.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RendererKind {
    /// The native Black Box renderer: lean, runs on every API and on low-end PCs. The default.
    #[default]
    Blackbox,
    /// The optional Bevy renderer, with the extra upscalers, temporal anti-aliasing and ray tracing. Only a build
    /// with the `renderer-bevy` cargo feature has it; any other build falls back to `blackbox`.
    Bevy,
}

names!(RendererKind, "blackbox or bevy", [(Self::Blackbox, "blackbox"), (Self::Bevy, "bevy")]);
