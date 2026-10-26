//! The keyed curves of a particle's size, rotation and colour over its life.

/// A cubic through four keyed values: `value(key[i]) == values[i]`. `t` runs from 0 (born) to 1 (at the
/// emitter's nominal life). Keys that coincide fall back to straight lines between the values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Curve {
    keys: [f32; 4],
    values: [f32; 4],
    cubic: bool,
}

impl Curve {
    pub fn new(keys: [f32; 4], values: [f32; 4]) -> Self {
        let distinct = (0..4).all(|i| (i + 1..4).all(|j| (keys[i] - keys[j]).abs() > 1e-6));
        Self { keys, values, cubic: distinct }
    }

    pub fn value(&self, t: f32) -> f32 {
        if !self.cubic {
            return self.linear(t);
        }
        let mut sum = 0.0;
        for i in 0..4 {
            let mut term = self.values[i];
            for j in (0..4).filter(|&j| j != i) {
                term *= (t - self.keys[j]) / (self.keys[i] - self.keys[j]);
            }
            sum += term;
        }
        sum
    }

    fn linear(&self, t: f32) -> f32 {
        let mut order = [0usize, 1, 2, 3];
        order.sort_by(|&a, &b| self.keys[a].total_cmp(&self.keys[b]));
        let (first, last) = (order[0], order[3]);
        if t <= self.keys[first] {
            return self.values[first];
        }
        for pair in order.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if t > self.keys[b] {
                continue;
            }
            let span = self.keys[b] - self.keys[a];
            if span <= 1e-6 {
                return self.values[b];
            }
            return self.values[a] + (self.values[b] - self.values[a]) * (t - self.keys[a]) / span;
        }
        self.values[last]
    }
}

/// One key of a [`Track`]: where it sits along the life and what the particle looks like there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackKey {
    /// 0 (born) to 1 (the end of the emitter's nominal life), rising along the track.
    pub t: f32,
    /// Full width of the sprite.
    pub size: f32,
    /// Rotation in degrees added to the initial angle.
    pub angle: f32,
    /// RGBA, straight.
    pub color: [u8; 4],
}

/// The look of a particle over its life as straight lines between any number of keys (the emitter's four-key cubics are
/// used when it has none). Before the first key and after the last the nearest key's values hold.
pub fn track_at(keys: &[TrackKey], t: f32) -> Option<(f32, f32, [f32; 4])> {
    let (first, last) = (keys.first()?, keys.last()?);
    let colour = |k: &TrackKey| k.color.map(f32::from);
    if t <= first.t {
        return Some((first.size, first.angle, colour(first)));
    }
    for pair in keys.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        if t > b.t {
            continue;
        }
        let span = b.t - a.t;
        let u = if span > 1e-6 { ((t - a.t) / span).clamp(0.0, 1.0) } else { 1.0 };
        let mix = |x: f32, y: f32| x + (y - x) * u;
        let (ca, cb) = (colour(a), colour(b));
        return Some((mix(a.size, b.size), mix(a.angle, b.angle), std::array::from_fn(|i| mix(ca[i], cb[i]))));
    }
    Some((last.size, last.angle, colour(last)))
}
