//! A mod car's own engine sound (the `[engine_sound]` table of the car manifest, docs/modding/car-sound.md):
//! looping recordings the audio mixes by engine speed. A car without them plays the game's built-in engine.

/// One looping recording and the engine speed it was made at.
#[derive(Debug, Clone, PartialEq)]
pub struct EngineLayer {
    /// The WAV file, a path of the install's layout.
    pub file: String,
    pub rpm: f32,
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
