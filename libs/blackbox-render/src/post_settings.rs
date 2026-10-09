//! Which post-process effects run and how strong they are. Pure data: the GPU side is in `gpu/post/`.

/// How the HDR scene image is mapped to the displayable range.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Tonemap {
    /// No curve: the resolve pass clamps to the output range, as before tone mapping existed.
    #[default]
    Off,
    /// A filmic ACES fit (Krzysztof Narkowicz's rational approximation of the ACES RRT+ODT curve). It
    /// compresses highlights, adds contrast and maps 1.0 to about 0.80, so it visibly darkens an image
    /// authored for a clamped pipeline.
    Aces,
}

/// Anti-aliasing applied to the render-size image, before any upscaling.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Antialiasing {
    #[default]
    Off,
    /// Fast approximate anti-aliasing: a single fullscreen pass that smooths edges found by luma contrast.
    Fxaa,
}

/// One effect of the chain, named for planning and tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostEffect {
    Bloom,
    Tonemap,
    Fxaa,
}

pub const MIN_EXPOSURE: f32 = 0.1;
pub const MAX_EXPOSURE: f32 = 8.0;
pub const MAX_BLOOM_INTENSITY: f32 = 2.0;
pub const MAX_BLOOM_THRESHOLD: f32 = 4.0;
/// Bloom starts at this brightness (linear, 1.0 is white) unless the caller picks another.
pub const DEFAULT_BLOOM_THRESHOLD: f32 = 0.8;

/// The post-process effects. [`Default`] runs none of them, so the frame is the plain clamped scene.
///
/// The chain order is fixed: bloom, then tone mapping, then anti-aliasing (which needs the final,
/// display-range colours), then the renderer's own resolve (and later upscaling) pass.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PostSettings {
    pub tonemap: Tonemap,
    /// Linear scale applied before the tone-mapping curve ([`MIN_EXPOSURE`]..=[`MAX_EXPOSURE`]; 1.0 is neutral).
    pub exposure: f32,
    /// How much of the blurred bright areas is added back (0.0 turns bloom off; at most [`MAX_BLOOM_INTENSITY`]).
    pub bloom_intensity: f32,
    /// The brightness above which pixels bloom (0.0..=[`MAX_BLOOM_THRESHOLD`]; a soft knee starts a little below).
    pub bloom_threshold: f32,
    pub antialiasing: Antialiasing,
}

impl Default for PostSettings {
    fn default() -> Self {
        Self {
            tonemap: Tonemap::Off,
            exposure: 1.0,
            bloom_intensity: 0.0,
            bloom_threshold: DEFAULT_BLOOM_THRESHOLD,
            antialiasing: Antialiasing::Off,
        }
    }
}

impl PostSettings {
    /// The values clamped into their ranges; non-finite values fall back to the defaults.
    pub fn sanitized(self) -> Self {
        let defaults = Self::default();
        let pick = |v: f32, default: f32, lo: f32, hi: f32| match v.is_finite() {
            true => v.clamp(lo, hi),
            false => default,
        };
        Self {
            exposure: pick(self.exposure, defaults.exposure, MIN_EXPOSURE, MAX_EXPOSURE),
            bloom_intensity: pick(self.bloom_intensity, defaults.bloom_intensity, 0.0, MAX_BLOOM_INTENSITY),
            bloom_threshold: pick(self.bloom_threshold, defaults.bloom_threshold, 0.0, MAX_BLOOM_THRESHOLD),
            ..self
        }
    }

    /// The enabled effects in the order they run.
    pub fn effects(&self) -> Vec<PostEffect> {
        let mut effects = Vec::new();
        if self.bloom_intensity > 0.0 {
            effects.push(PostEffect::Bloom);
        }
        if self.tonemap != Tonemap::Off {
            effects.push(PostEffect::Tonemap);
        }
        if self.antialiasing != Antialiasing::Off {
            effects.push(PostEffect::Fxaa);
        }
        effects
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_runs_nothing() {
        assert!(PostSettings::default().effects().is_empty());
        assert_eq!(PostSettings::default().sanitized(), PostSettings::default());
    }

    #[test]
    fn effects_run_in_the_documented_order_and_toggle_independently() {
        let all = PostSettings {
            tonemap: Tonemap::Aces,
            bloom_intensity: 0.5,
            antialiasing: Antialiasing::Fxaa,
            ..PostSettings::default()
        };
        assert_eq!(all.effects(), [PostEffect::Bloom, PostEffect::Tonemap, PostEffect::Fxaa]);
        let no_bloom = PostSettings { bloom_intensity: 0.0, ..all };
        assert_eq!(no_bloom.effects(), [PostEffect::Tonemap, PostEffect::Fxaa]);
        let only_aa = PostSettings { antialiasing: Antialiasing::Fxaa, ..PostSettings::default() };
        assert_eq!(only_aa.effects(), [PostEffect::Fxaa]);
        let only_bloom = PostSettings { bloom_intensity: 0.1, ..PostSettings::default() };
        assert_eq!(only_bloom.effects(), [PostEffect::Bloom]);
    }

    #[test]
    fn sanitizing_clamps_and_replaces_non_finite_values() {
        let wild = PostSettings {
            exposure: 100.0,
            bloom_intensity: -3.0,
            bloom_threshold: f32::NAN,
            ..PostSettings::default()
        };
        let clean = wild.sanitized();
        assert_eq!(clean.exposure, MAX_EXPOSURE);
        assert_eq!(clean.bloom_intensity, 0.0);
        assert_eq!(clean.bloom_threshold, DEFAULT_BLOOM_THRESHOLD);
        assert_eq!(PostSettings { exposure: 0.0, ..PostSettings::default() }.sanitized().exposure, MIN_EXPOSURE);
        assert_eq!(PostSettings { exposure: f32::INFINITY, ..PostSettings::default() }.sanitized().exposure, 1.0);
    }
}
