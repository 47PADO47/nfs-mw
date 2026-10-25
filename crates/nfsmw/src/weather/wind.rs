//! The wind vector a player view hands to the scenery sway: the track's `Wind` layer, low-passed by distance and
//! turned into the camera's frame. Spec: `docs/specs/scenery-sway.md` ("The wind vector").
use blackbox_hash::bstring_hash;
use blackbox_paramap::Layer;
use glam::Vec3;

/// The layer field that holds the wind's heading in degrees.
const ANGLE_FIELD: usize = 1;
/// Moves shorter than this blend the new sample in by the fraction moved; longer ones take it whole.
const BLEND_DISTANCE: f32 = 50.0;
/// The two player views.
const VIEWS: usize = 2;

/// The last sample of one view: where it was taken and the blended heading.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Sample {
    at: [f32; 2],
    degrees: f32,
}

#[derive(Debug, Clone, Default)]
pub struct Wind {
    layer: Option<Layer>,
    samples: [Option<Sample>; VIEWS],
}

impl Wind {
    pub fn read(metadata: &[u8]) -> anyhow::Result<Self> {
        let layer = Layer::read_all(metadata)?.into_iter().find(|l| l.name_hash == bstring_hash("Wind"));
        Ok(Self { layer, samples: [None; VIEWS] })
    }

    /// The wind vector of player view `view` for a camera at `position` looking along `forward` (world, Z up): the
    /// heading in the camera's right, up and forward components. Zero without a `Wind` layer, which the sway takes
    /// as its fallback axis.
    pub fn axis(&mut self, view: usize, position: Vec3, forward: Vec3) -> Vec3 {
        let Some(degrees) = self.heading(view, [position.x, position.y]) else { return Vec3::ZERO };
        let units = (degrees * 65536.0) as i32 / 360;
        let (sin, cos) = (f32::from(units as u16) * std::f32::consts::TAU / 65536.0).sin_cos();
        let wind = Vec3::new(sin, cos, 0.0);
        let forward = forward.normalize_or_zero();
        let right = forward.cross(Vec3::Z).normalize_or_zero();
        let up = right.cross(forward);
        Vec3::new(wind.dot(right), wind.dot(up), wind.dot(forward))
    }

    /// The blended heading in degrees at `at` for `view`, or `None` without a layer.
    fn heading(&mut self, view: usize, at: [f32; 2]) -> Option<f32> {
        let layer = self.layer.as_ref()?;
        let slot = self.samples.get_mut(view)?;
        let sample = layer.set_at(at[0], at[1]).and_then(|set| layer.float(set, ANGLE_FIELD));
        let Some(sample) = sample.filter(|v| v.is_finite()) else { return slot.map(|s| s.degrees) };
        let degrees = blend(slot.as_ref(), at, sample);
        *slot = Some(Sample { at, degrees });
        Some(degrees)
    }
}

/// The original's low-pass: the first sample whole, then `p * sample + (1 - p) * previous` with `p` the fraction of
/// the blend distance moved, or the sample whole after a longer move.
fn blend(previous: Option<&Sample>, at: [f32; 2], sample: f32) -> f32 {
    let Some(previous) = previous else { return sample };
    let moved = (at[0] - previous.at[0]).hypot(at[1] - previous.at[1]);
    if moved >= BLEND_DISTANCE {
        return sample;
    }
    let p = moved / BLEND_DISTANCE;
    p * sample + (1.0 - p) * previous.degrees
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_sample_and_long_moves_are_taken_whole() {
        assert_eq!(blend(None, [0.0, 0.0], 90.0), 90.0);
        let previous = Sample { at: [0.0, 0.0], degrees: 10.0 };
        assert_eq!(blend(Some(&previous), [30.0, 40.0], 90.0), 90.0);
    }

    #[test]
    fn short_moves_blend_by_the_distance() {
        let previous = Sample { at: [0.0, 0.0], degrees: 10.0 };
        assert_eq!(blend(Some(&previous), [0.0, 0.0], 90.0), 10.0, "standing still keeps the heading");
        assert!((blend(Some(&previous), [25.0, 0.0], 90.0) - 50.0).abs() < 1e-4);
    }

    #[test]
    fn no_layer_is_no_wind() {
        assert_eq!(Wind::default().axis(0, Vec3::ZERO, Vec3::Y), Vec3::ZERO);
    }

    #[test]
    #[ignore = "requires the original track pack"]
    fn installed_track_has_a_wind_heading() {
        let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
        let bytes = nfsmw_data::read_unwrapped(&dir, "TRACKS/L2RA.BUN").unwrap();
        let wind = Wind::read(&bytes).unwrap();
        let layer = wind.layer.as_ref().expect("installed Wind parameter map");
        assert!(layer.set_count() > 0);
        assert!((0..layer.set_count()).all(|set| layer.float(set, ANGLE_FIELD).is_some()), "field 1 is a float");
    }
}
