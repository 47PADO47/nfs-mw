//! Decides which rows of the shared cloud map the next pass rebuilds.
//!
//! Wind scroll and a slowly moving sun change every frame, but re-marching the whole map for them costs more
//! than the clouds are worth. Those changes rebuild one horizontal band per call, cycling through the bands, so
//! every texel is still a complete single-frame march and at most `BANDS - 1` calls old. A coverage, warmth,
//! sun or camera change that is visible at once rebuilds every row immediately. No texel blends with history.

/// Horizontal bands the map is split into. Four bands make the steady wind cost a quarter of a full march.
pub(super) const BANDS: u32 = 4;
const ALL_STALE: u32 = (1 << BANDS) - 1;
/// Squared change of a sun or light vector (about one degree) above which the whole map rebuilds at once.
const BIG_VECTOR_MOVE: f32 = 3.0e-4;
/// Change of coverage or sunset warmth above which the whole map rebuilds at once.
const BIG_SCALAR_MOVE: f32 = 0.02;
/// Change of the wind scroll above which the whole map rebuilds at once: a clock jump, not a frame of wind (a frame
/// moves it by hundredths at most).
const BIG_SCROLL_MOVE: f32 = 1.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) struct Key {
    pub scroll: f32,
    pub coverage: f32,
    pub sun: [f32; 4],
    pub light: [f32; 3],
    pub warmth: f32,
}

fn distance_squared(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(a, b)| (a - b).powi(2)).sum()
}

impl Key {
    /// Whether the change from `previous` is too large to show a few frames late in some bands.
    fn jumped_from(&self, previous: &Key) -> bool {
        (self.scroll - previous.scroll).abs() > BIG_SCROLL_MOVE
            || (self.coverage - previous.coverage).abs() > BIG_SCALAR_MOVE
            || (self.warmth - previous.warmth).abs() > BIG_SCALAR_MOVE
            || distance_squared(&self.sun, &previous.sun) > BIG_VECTOR_MOVE
            || distance_squared(&self.light, &previous.light) > BIG_VECTOR_MOVE
    }
}

/// Which rows a pass redraws.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Rebuild {
    Full,
    Band(u32),
}

impl Rebuild {
    /// First row and row count to scissor for a map `height` rows tall.
    pub fn rows(self, height: u32) -> (u32, u32) {
        let Rebuild::Band(band) = self else { return (0, height) };
        let (start, end) = (band * height / BANDS, (band + 1) * height / BANDS);
        (start, end - start)
    }
}

#[derive(Default)]
pub(super) struct Schedule {
    key: Option<Key>,
    stale: u32,
    cursor: u32,
}

impl Schedule {
    /// The rebuild to record for this call, if any. `moved` is a camera move far enough to need a full march.
    pub fn plan(&mut self, key: Key, moved: bool) -> Option<Rebuild> {
        let Some(previous) = self.key else { return self.full(key) };
        if moved || key.jumped_from(&previous) {
            return self.full(key);
        }
        if key != previous {
            self.key = Some(key);
            self.stale = ALL_STALE;
        }
        self.next_band().map(Rebuild::Band)
    }

    fn full(&mut self, key: Key) -> Option<Rebuild> {
        self.key = Some(key);
        self.stale = 0;
        Some(Rebuild::Full)
    }

    fn next_band(&mut self) -> Option<u32> {
        let mut order = (0..BANDS).map(|offset| (self.cursor + offset) % BANDS);
        let band = order.find(|&band| (self.stale >> band) & 1 == 1)?;
        self.stale &= !(1 << band);
        self.cursor = (band + 1) % BANDS;
        Some(band)
    }
}

#[cfg(test)]
#[path = "cloud_schedule_tests.rs"]
mod tests;
