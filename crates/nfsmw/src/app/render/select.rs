//! Choosing the renderer at startup (docs/renderers.md): the `renderer` setting is a request, and a request this
//! build or this PC cannot satisfy falls back to the native renderer instead of failing to start.

use crate::settings::RendererKind;

/// Whether this build has the Bevy renderer: the `renderer-bevy` cargo feature, which is off by default.
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

/// [`decide`], then, for bevy, the adapter probe: `probe` says whether this PC can run it (and why not), and
/// a PC that cannot falls back to the native renderer too. The probe runs before the `App` is built, because
/// Bevy panics when it finds no adapter once it exists.
pub fn decide_with_probe(
    requested: RendererKind,
    bevy_compiled: bool,
    probe: impl FnOnce() -> Result<(), String>,
) -> Choice {
    let choice = decide(requested, bevy_compiled);
    if choice.renderer != RendererKind::Bevy {
        return choice;
    }
    match probe() {
        Ok(()) => choice,
        Err(why) => Choice {
            renderer: RendererKind::Blackbox,
            fallback: Some(format!("bevy was requested, but this PC cannot run it ({why}); using blackbox")),
        },
    }
}

/// The renderer to create for `requested` on graphics API `api`. A fallback is logged.
pub fn choose(requested: RendererKind, api: blackbox_gfx::GraphicsApi) -> RendererKind {
    let choice = decide_with_probe(requested, BEVY_COMPILED, || super::bevy::probe(api));
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
    fn a_pc_the_probe_rejects_falls_back_with_the_reason() {
        let choice = decide_with_probe(RendererKind::Bevy, true, || Err("no GPU adapter".to_owned()));
        assert_eq!(choice.renderer, RendererKind::Blackbox);
        assert!(choice.fallback.unwrap().contains("no GPU adapter"));
        let kept = decide_with_probe(RendererKind::Bevy, true, || Ok(()));
        assert_eq!(kept, Choice { renderer: RendererKind::Bevy, fallback: None });
    }

    #[test]
    fn the_probe_is_not_asked_for_blackbox_or_a_build_without_bevy() {
        let never = || -> Result<(), String> { panic!("probed") };
        assert_eq!(decide_with_probe(RendererKind::Blackbox, true, never).renderer, RendererKind::Blackbox);
        assert_eq!(decide_with_probe(RendererKind::Bevy, false, never).renderer, RendererKind::Blackbox);
    }

    #[test]
    #[cfg(not(feature = "renderer-bevy"))]
    fn a_default_build_has_no_bevy_renderer() {
        assert!(!BEVY_COMPILED);
        assert_eq!(choose(RendererKind::Bevy, blackbox_gfx::GraphicsApi::Auto), RendererKind::Blackbox);
    }
}
