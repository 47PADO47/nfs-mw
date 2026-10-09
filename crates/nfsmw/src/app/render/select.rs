//! Choosing the renderer at startup (docs/renderers.md): the `renderer` setting is a request, and a request this
//! build or this PC cannot satisfy falls back to the native renderer instead of failing to start.

use crate::settings::RendererKind;

/// Whether this build has the Bevy renderer: the `renderer-bevy` cargo feature, which is off by default and
/// which no build uses until the Bevy backend lands.
pub const BEVY_COMPILED: bool = cfg!(feature = "renderer-bevy");

/// The renderer to create, and why it is not the requested one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    pub renderer: RendererKind,
    /// Set when the request could not be honoured.
    pub fallback: Option<String>,
}

/// Map the request onto what the build can do. Pure, so the rules are testable.
pub fn decide(requested: RendererKind, bevy_compiled: bool) -> Choice {
    if requested == RendererKind::Bevy && !bevy_compiled {
        let fallback = "bevy was requested, but this build has no bevy renderer (it needs the renderer-bevy cargo \
                        feature); using blackbox"
            .to_owned();
        return Choice { renderer: RendererKind::Blackbox, fallback: Some(fallback) };
    }
    Choice { renderer: requested, fallback: None }
}

/// The renderer to create for `requested`. A fallback is logged.
pub fn choose(requested: RendererKind) -> RendererKind {
    let choice = decide(requested, BEVY_COMPILED);
    if let Some(reason) = &choice.fallback {
        log::error!("renderer: {reason}");
    }
    choice.renderer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blackbox_is_always_available() {
        for compiled in [false, true] {
            assert_eq!(
                decide(RendererKind::Blackbox, compiled),
                Choice { renderer: RendererKind::Blackbox, fallback: None }
            );
        }
    }

    #[test]
    fn bevy_without_the_feature_falls_back_to_blackbox_with_a_reason() {
        let choice = decide(RendererKind::Bevy, false);
        assert_eq!(choice.renderer, RendererKind::Blackbox);
        let reason = choice.fallback.expect("a reason is logged");
        assert!(reason.contains("renderer-bevy") && reason.contains("using blackbox"), "{reason}");
    }

    #[test]
    fn bevy_with_the_feature_is_kept() {
        assert_eq!(decide(RendererKind::Bevy, true), Choice { renderer: RendererKind::Bevy, fallback: None });
    }

    #[test]
    #[cfg(not(feature = "renderer-bevy"))]
    fn this_build_has_no_bevy_renderer_yet() {
        assert!(!BEVY_COMPILED);
        assert_eq!(choose(RendererKind::Bevy), RendererKind::Blackbox);
    }
}
