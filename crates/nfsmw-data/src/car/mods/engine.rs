//! A mod car's own engine sound (the `[engine_sound]` table of the car manifest, docs/modding/car-sound.md):
//! looping recordings the audio mixes by engine speed. A car without them plays the game's built-in engine.

/// What a layer's curve or pitch is read at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveInput {
    /// How far the engine is from idle (0) to redline (1).
    Rpm,
    /// The engine load: the throttle, 0 to 1.
    Load,
    /// The car's speed in miles per hour.
    SpeedMph,
}

/// A piecewise linear curve of one input, flat before its first and after its last point.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerCurve {
    pub input: CurveInput,
    pub points: Vec<(f32, f32)>,
}

impl LayerCurve {
    pub fn eval(&self, x: f32) -> f32 {
        let (Some(first), Some(last)) = (self.points.first(), self.points.last()) else { return 1.0 };
        if x <= first.0 {
            return first.1;
        }
        if x >= last.0 {
            return last.1;
        }
        let Some(pair) = self.points.windows(2).find(|pair| x <= pair[1].0) else { return last.1 };
        pair[0].1 + (pair[1].1 - pair[0].1) * (x - pair[0].0) / (pair[1].0 - pair[0].0)
    }
}

/// A pitch that follows an input: the playback rate is the input over `reference`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayerPitch {
    pub input: CurveInput,
    pub reference: f32,
}

/// What a layer plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerKind {
    /// A looping WAV.
    Loop,
    /// A granular engine recording (`.GIN`).
    Ginsu,
}

/// One layer of the engine: a looping recording and the engine speed it was made at, or a layer that plays by curves.
#[derive(Debug, Clone, PartialEq)]
pub struct EngineLayer {
    /// The WAV (or `.GIN`) file, a path of the install's layout.
    pub file: String,
    /// The engine speed of the recording; 0 for a layer with curves.
    pub rpm: f32,
    pub kind: LayerKind,
    /// The curves that multiply into the layer's volume; empty for a layer mixed by `rpm`.
    pub gain: Vec<LayerCurve>,
    pub pitch: Option<LayerPitch>,
    /// A curve that is the playback rate itself (1 is the recording's own speed); it takes the place of `pitch`.
    pub rate: Option<LayerCurve>,
    /// For a Ginsu layer: the frequencies from idle to redline.
    pub span: Option<(f32, f32)>,
    pub volume: f32,
}

impl EngineLayer {
    /// A layer mixed by the engine speed it was recorded at.
    pub fn classic(file: impl Into<String>, rpm: f32) -> Self {
        Self {
            file: file.into(),
            rpm,
            kind: LayerKind::Loop,
            gain: Vec::new(),
            pitch: None,
            rate: None,
            span: None,
            volume: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EngineSound {
    /// The rpm the engine idles at and the rpm of its rev limiter: what the sound's `0 to 1` engine speed spans.
    pub idle_rpm: f32,
    pub redline_rpm: f32,
    /// Slowest first.
    pub layers: Vec<EngineLayer>,
}

impl EngineSound {
    /// The engine speed in rpm at `fraction` of the way from idle to redline.
    pub fn rpm_at(&self, fraction: f32) -> f32 {
        self.idle_rpm + fraction.clamp(0.0, 1.0) * (self.redline_rpm - self.idle_rpm)
    }
}

/// A mod car's own recordings of the sounds besides its engine (the `[car_sound]` table of the car manifest,
/// docs/modding/car-sound.md). A list left empty is silence for that effect: the car plays no other car's sound.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CarEffects {
    /// WAV files played at a gear change that `shift_up` or `shift_down` does not cover.
    pub shift: Vec<String>,
    pub shift_up: Vec<String>,
    pub shift_down: Vec<String>,
    /// Loops of the turbo's whistle, mixed by how far it has spooled.
    pub turbo_whine: Vec<String>,
    pub turbo_blowoff: Vec<String>,
    pub backfire: Vec<String>,
    pub pops: Vec<String>,
    /// The car makes no gear clunk: a gear change is silent unless `shift` names recordings (an electric car).
    pub no_clunk: bool,
    /// The recordings the mod's cars share.
    pub shared: SharedSound,
}

/// The recordings the cars of a mod share (the `[car_sound.shared]` table, docs/modding/car-sound.md): body hits by
/// strength, scrape one-shots and pops. A list left empty is a sound the car does not give, and the game plays what it
/// plays for a car without it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SharedSound {
    pub impact_small: Vec<String>,
    pub impact_medium: Vec<String>,
    pub impact_big: Vec<String>,
    pub scrape: Vec<String>,
    pub pops: Vec<String>,
}

impl SharedSound {
    /// The files of the body-hit class `class`: 0 light, 1 medium, 2 heavy (anything above is heavy).
    pub fn impact_files(&self, class: usize) -> &[String] {
        match class {
            0 => &self.impact_small,
            1 => &self.impact_medium,
            _ => &self.impact_big,
        }
    }

    /// Whether the car has a body-hit recording of any class.
    pub fn has_impacts(&self) -> bool {
        !(self.impact_small.is_empty() && self.impact_medium.is_empty() && self.impact_big.is_empty())
    }
}

impl CarEffects {
    /// The files for a gear change: the ones for its direction, else the ones for any.
    pub fn shift_files(&self, up: bool) -> &[String] {
        let own = if up { &self.shift_up } else { &self.shift_down };
        if own.is_empty() { &self.shift } else { own }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shift_uses_the_files_of_its_direction_and_falls_back_to_the_general_ones() {
        let general = vec!["SOUND/ANY.WAV".to_owned()];
        let mut effects = CarEffects { shift: general.clone(), ..Default::default() };
        assert_eq!((effects.shift_files(true), effects.shift_files(false)), (&general[..], &general[..]));
        effects.shift_up = vec!["SOUND/UP.WAV".to_owned()];
        assert_eq!(effects.shift_files(true), ["SOUND/UP.WAV"]);
        assert_eq!(effects.shift_files(false), ["SOUND/ANY.WAV"]);
        assert!(CarEffects::default().shift_files(true).is_empty());
    }

    #[test]
    fn a_blow_s_class_picks_its_list_and_the_heaviest_takes_the_rest() {
        let shared = SharedSound {
            impact_small: vec!["S".into()],
            impact_medium: vec!["M".into()],
            impact_big: vec!["B".into()],
            ..Default::default()
        };
        assert_eq!(
            (shared.impact_files(0), shared.impact_files(1), shared.impact_files(2)),
            (&["S".to_owned()][..], &["M".to_owned()][..], &["B".to_owned()][..])
        );
        assert_eq!(shared.impact_files(9), ["B"]);
        assert!(shared.has_impacts() && !SharedSound::default().has_impacts());
    }
}
