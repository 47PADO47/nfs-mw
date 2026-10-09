//! Mapping a request onto what a renderer can do: [`resolve`].
//!
//! Pure and deterministic, so every rule is unit-tested without a GPU. The rules, in order:
//!
//! 1. The request is sanitised (clamped) first, silently.
//! 2. A tone-mapping curve or bloom the renderer lacks is turned off.
//! 3. An unavailable upscaler falls back along `fsr4 > fsr3 > fsr1 > bilinear > off` and
//!    `dlss > fsr3 > fsr1 > bilinear > off`.
//! 4. A temporal upscaler (FSR 3, FSR 4, DLSS) replaces anti-aliasing: it is turned off, with a downgrade
//!    saying so. Otherwise an unavailable method falls back: `taa > fxaa > off`, `smaa > fxaa > off`.
//! 5. The render scale: a requested temporal upscaler lets its quality mode decide; `Upscaler::Off` draws
//!    at the output size; the result is clamped to the renderer's range.
//! 6. Ray tracing the renderer lacks is turned off. Without a denoiser it is allowed, with a note.

use std::fmt;

use crate::{Antialiasing, GraphicsSettings, RayTracing, Tonemap, Upscaler};

use super::{Capabilities, RtSupport, Setting};

/// One setting that is not what the user asked for, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Downgrade {
    pub setting: Setting,
    pub requested: String,
    pub effective: String,
    pub reason: String,
}

impl fmt::Display for Downgrade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {} -> {} ({})", self.setting, self.requested, self.effective, self.reason)
    }
}

/// A caveat on a setting that is kept as asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub setting: Setting,
    pub text: String,
}

impl fmt::Display for Note {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.setting, self.text)
    }
}

/// The outcome of [`resolve`].
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    /// What the renderer will actually run.
    pub effective: GraphicsSettings,
    /// Every request that was changed, in the order the rules ran.
    pub downgrades: Vec<Downgrade>,
    /// Caveats on requests that were kept.
    pub notes: Vec<Note>,
}

impl Resolved {
    /// Whether every request was honoured.
    pub fn is_exact(&self) -> bool {
        self.downgrades.is_empty()
    }

    /// The downgrade of one setting, if it has one.
    pub fn downgrade_of(&self, setting: Setting) -> Option<&Downgrade> {
        self.downgrades.iter().find(|d| d.setting == setting)
    }
}

/// Map `requested` onto `caps`. See the module docs for the rules.
pub fn resolve(requested: &GraphicsSettings, caps: &Capabilities) -> Resolved {
    let requested = requested.sanitized();
    let mut out = Resolved { effective: requested, downgrades: Vec::new(), notes: Vec::new() };
    tonemap(&mut out, caps);
    bloom(&mut out, caps);
    upscaler(&mut out, caps, requested.upscaler);
    antialiasing(&mut out, caps);
    render_scale(&mut out, caps, &requested);
    ray_tracing(&mut out, caps);
    out
}

impl Resolved {
    fn downgrade(&mut self, setting: Setting, requested: impl ToString, effective: impl ToString, reason: String) {
        let (requested, effective) = (requested.to_string(), effective.to_string());
        self.downgrades.push(Downgrade { setting, requested, effective, reason });
    }

    fn note(&mut self, setting: Setting, text: &str) {
        self.notes.push(Note { setting, text: text.to_owned() });
    }
}

fn unsupported(caps: &Capabilities) -> String {
    format!("not supported by the {} renderer on {}", caps.renderer, caps.api)
}

fn tonemap(out: &mut Resolved, caps: &Capabilities) {
    let asked = out.effective.post.tonemap;
    if asked == Tonemap::Off || caps.tonemaps.contains(asked) {
        return;
    }
    out.effective.post.tonemap = Tonemap::Off;
    out.downgrade(Setting::Tonemap, asked.name(), Tonemap::Off.name(), unsupported(caps));
}

fn bloom(out: &mut Resolved, caps: &Capabilities) {
    let asked = out.effective.post.bloom_intensity;
    if asked <= 0.0 || caps.bloom {
        return;
    }
    out.effective.post.bloom_intensity = 0.0;
    out.downgrade(Setting::Bloom, format!("{asked:.2}"), "off", unsupported(caps));
}

/// The upscalers to try for a request, best first. `Off` is last and always works.
fn upscaler_chain(requested: Upscaler) -> &'static [Upscaler] {
    use Upscaler::*;
    match requested {
        Off => &[Off],
        Bilinear => &[Bilinear, Off],
        Fsr1 => &[Fsr1, Bilinear, Off],
        Fsr3 => &[Fsr3, Fsr1, Bilinear, Off],
        Fsr4 => &[Fsr4, Fsr3, Fsr1, Bilinear, Off],
        Dlss => &[Dlss, Fsr3, Fsr1, Bilinear, Off],
    }
}

fn upscaler(out: &mut Resolved, caps: &Capabilities, requested: Upscaler) {
    let available = |u: &Upscaler| *u == Upscaler::Off || caps.upscalers.contains(*u);
    let chosen = upscaler_chain(requested).iter().copied().find(available).unwrap_or(Upscaler::Off);
    out.effective.upscaler = chosen;
    if chosen == requested {
        return;
    }
    out.downgrade(Setting::Upscaler, requested.name(), chosen.name(), unsupported(caps));
}

fn antialiasing(out: &mut Resolved, caps: &Capabilities) {
    let asked = out.effective.post.antialiasing;
    if asked == Antialiasing::Off {
        return;
    }
    let upscaler = out.effective.upscaler;
    if upscaler.is_temporal() {
        out.effective.post.antialiasing = Antialiasing::Off;
        let reason = format!("{} does its own anti-aliasing", upscaler.name());
        out.downgrade(Setting::Antialiasing, asked.name(), Antialiasing::Off.name(), reason);
        return;
    }
    let chain: &[Antialiasing] = match asked {
        Antialiasing::Taa | Antialiasing::Smaa => &[asked, Antialiasing::Fxaa, Antialiasing::Off],
        _ => &[asked, Antialiasing::Off],
    };
    let chosen = chain.iter().copied().find(|a| *a == Antialiasing::Off || caps.antialiasing.contains(*a));
    let chosen = chosen.unwrap_or(Antialiasing::Off);
    out.effective.post.antialiasing = chosen;
    if chosen == asked {
        return;
    }
    out.downgrade(Setting::Antialiasing, asked.name(), chosen.name(), unsupported(caps));
}

fn render_scale(out: &mut Resolved, caps: &Capabilities, requested: &GraphicsSettings) {
    let asked = requested.render_scale;
    let from_quality = requested.upscaler.is_temporal();
    let mut scale = match from_quality {
        true => requested.upscale_quality.render_scale(),
        false => asked,
    };
    if out.effective.upscaler == Upscaler::Off && scale != 1.0 {
        scale = 1.0;
        if !from_quality {
            out.downgrade(Setting::RenderScale, format!("{asked:.2}"), "1.00", "upscaling is off".to_owned());
        }
    }
    let (lo, hi) = (caps.render_scale.0.min(caps.render_scale.1), caps.render_scale.0.max(caps.render_scale.1));
    let clamped = scale.clamp(lo, hi);
    out.effective.render_scale = clamped;
    if clamped == scale {
        return;
    }
    let reason = format!("the {} renderer supports {lo:.2} to {hi:.2}", caps.renderer);
    out.downgrade(Setting::RenderScale, format!("{scale:.2}"), format!("{clamped:.2}"), reason);
}

fn ray_tracing(out: &mut Resolved, caps: &Capabilities) {
    let asked = out.effective.ray_tracing;
    if asked == RayTracing::Off {
        return;
    }
    let RtSupport::Available { denoiser } = caps.ray_tracing else {
        out.effective.ray_tracing = RayTracing::Off;
        out.downgrade(Setting::RayTracing, asked.name(), RayTracing::Off.name(), unsupported(caps));
        return;
    };
    if denoiser.is_none() {
        out.note(Setting::RayTracing, "no denoiser is available, so ray-traced lighting will be noisy");
    }
}
