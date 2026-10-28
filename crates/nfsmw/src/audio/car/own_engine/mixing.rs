//! What each layer of a mod car's engine should sound like for the live values of the car: the layers mixed by the engine
//! speed they were recorded at (equal power, on a log scale) and the layers that play by their own curves.

use nfsmw_data::car::mods::{CurveInput, EngineLayer, EngineSound, LayerKind};

/// A layer mixed by engine speed plays no faster or slower than this, so one recording is not asked to cover the whole
/// range.
pub(super) const RATE_RANGE: (f32, f32) = (0.55, 1.8);
/// A layer whose pitch follows an input plays within this range of its own speed.
const FOLLOW_RANGE: (f32, f32) = (0.25, 4.0);

/// What one layer should sound like right now.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayerMix {
    pub gain: f32,
    pub rate: f32,
    /// For a Ginsu layer: the frequency its recording is played at; 0 for a loop.
    pub frequency: f32,
}

/// The live values of the car that the layers' curves read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EngineInputs {
    /// How far the engine is from idle (0) to redline (1).
    pub rpm: f32,
    /// The engine load: the throttle, 0 to 1.
    pub load: f32,
    pub speed_mph: f32,
}

impl EngineInputs {
    fn read(&self, input: CurveInput) -> f32 {
        match input {
            CurveInput::Rpm => self.rpm,
            CurveInput::Load => self.load,
            CurveInput::SpeedMph => self.speed_mph,
        }
    }
}

/// The mix of every layer of `sound` at engine speed `rpm`, all of them mixed by speed. The layers that have curves are
/// mixed by [`mix_inputs`].
#[cfg(test)]
pub fn mix(sound: &EngineSound, rpm: f32) -> Vec<LayerMix> {
    let layers: Vec<&EngineLayer> = sound.layers.iter().collect();
    by_speed(&layers, rpm)
}

/// The mix of every layer of `sound` for `inputs`: layers with curves play by them, the others are mixed by the engine
/// speed among themselves.
pub fn mix_inputs(sound: &EngineSound, inputs: &EngineInputs) -> Vec<LayerMix> {
    let plain: Vec<&EngineLayer> = sound.layers.iter().filter(|layer| layer.gain.is_empty()).collect();
    let mut by_rpm = by_speed(&plain, sound.rpm_at(inputs.rpm)).into_iter();
    sound
        .layers
        .iter()
        .map(|layer| match layer.gain.is_empty() {
            true => by_rpm.next().unwrap_or(LayerMix { gain: 0.0, rate: 1.0, frequency: 0.0 }),
            false => by_curves(layer, inputs),
        })
        .collect()
}

fn by_curves(layer: &EngineLayer, inputs: &EngineInputs) -> LayerMix {
    let gain = layer.gain.iter().fold(layer.volume, |gain, curve| gain * curve.eval(inputs.read(curve.input)));
    let rate = match (&layer.rate, layer.pitch) {
        (Some(curve), _) => curve.eval(inputs.read(curve.input)),
        (None, Some(pitch)) => inputs.read(pitch.input) / pitch.reference,
        (None, None) => 1.0,
    }
    .clamp(FOLLOW_RANGE.0, FOLLOW_RANGE.1);
    let frequency = match (layer.kind, layer.span) {
        (LayerKind::Ginsu, Some((low, high))) => low + inputs.rpm.clamp(0.0, 1.0) * (high - low),
        _ => 0.0,
    };
    LayerMix { gain: gain.max(0.0), rate, frequency }
}

/// The layers mixed by engine speed: loudest at their own rpm, equal power into their neighbours.
fn by_speed(layers: &[&EngineLayer], rpm: f32) -> Vec<LayerMix> {
    let mut out: Vec<LayerMix> = layers
        .iter()
        .map(|layer| LayerMix { gain: 0.0, rate: (rpm / layer.rpm).clamp(RATE_RANGE.0, RATE_RANGE.1), frequency: 0.0 })
        .collect();
    let Some(last) = out.len().checked_sub(1) else { return out };
    let Some(above) = layers.iter().position(|layer| layer.rpm >= rpm) else {
        out[last].gain = 1.0;
        return out;
    };
    if above == 0 {
        out[0].gain = 1.0;
        return out;
    }
    let below = above - 1;
    let span = (layers[above].rpm / layers[below].rpm).ln();
    let t = ((rpm / layers[below].rpm).ln() / span).clamp(0.0, 1.0);
    out[below].gain = (t * std::f32::consts::FRAC_PI_2).cos();
    out[above].gain = (t * std::f32::consts::FRAC_PI_2).sin();
    out
}

#[cfg(test)]
mod tests {
    use nfsmw_data::car::mods::{LayerCurve, LayerPitch};

    use super::*;

    fn sound(layers: Vec<EngineLayer>) -> EngineSound {
        EngineSound { idle_rpm: 800.0, redline_rpm: 7000.0, layers }
    }

    fn curved(file: &str, gain: Vec<LayerCurve>) -> EngineLayer {
        EngineLayer { gain, ..EngineLayer::classic(file, 0.0) }
    }

    fn curve(input: CurveInput, points: &[(f32, f32)]) -> LayerCurve {
        LayerCurve { input, points: points.to_vec() }
    }

    fn at(rpm: f32, load: f32, speed_mph: f32) -> EngineInputs {
        EngineInputs { rpm, load, speed_mph }
    }

    #[test]
    fn a_layer_plays_by_the_product_of_its_curves_and_its_volume() {
        let mut layer = curved(
            "G.GIN",
            vec![curve(CurveInput::Rpm, &[(0.0, 0.0), (0.5, 1.0)]), curve(CurveInput::Load, &[(0.0, 0.0), (1.0, 2.0)])],
        );
        layer.volume = 0.5;
        let engine = sound(vec![layer]);
        let mixed = mix_inputs(&engine, &at(0.25, 0.5, 0.0))[0];
        assert!((mixed.gain - 0.5 * 0.5 * 1.0).abs() < 1e-6, "{mixed:?}");
        assert_eq!(mixed.rate, 1.0, "no pitch rule: the layer plays at its own speed");
        assert_eq!(mix_inputs(&engine, &at(0.25, 0.0, 0.0))[0].gain, 0.0, "no load, no sound");
    }

    #[test]
    fn a_pitch_that_follows_the_speed_is_the_speed_over_its_reference_within_limits() {
        let mut whine = curved("W.WAV", vec![curve(CurveInput::SpeedMph, &[(10.0, 0.0), (20.0, 1.0), (90.0, 1.0)])]);
        whine.pitch = Some(LayerPitch { input: CurveInput::SpeedMph, reference: 40.0 });
        let engine = sound(vec![whine]);
        assert!((mix_inputs(&engine, &at(0.9, 1.0, 60.0))[0].rate - 1.5).abs() < 1e-6);
        assert_eq!(mix_inputs(&engine, &at(0.9, 1.0, 400.0))[0].rate, FOLLOW_RANGE.1);
        assert_eq!(mix_inputs(&engine, &at(0.9, 1.0, 1.0))[0].rate, FOLLOW_RANGE.0);
        assert_eq!(mix_inputs(&engine, &at(0.9, 1.0, 5.0))[0].gain, 0.0, "below its window");
    }

    #[test]
    fn a_rate_curve_is_the_playback_rate_and_takes_the_place_of_a_pitch_rule() {
        let mut motor = curved("M.WAV", vec![curve(CurveInput::SpeedMph, &[(0.0, 1.0), (150.0, 1.0)])]);
        motor.rate = Some(curve(CurveInput::SpeedMph, &[(0.0, 1.0), (60.0, 2.0), (150.0, 2.2)]));
        motor.pitch = Some(LayerPitch { input: CurveInput::SpeedMph, reference: 40.0 });
        let engine = sound(vec![motor]);
        let rate = |mph: f32| mix_inputs(&engine, &at(0.0, 0.0, mph))[0].rate;
        assert_eq!(rate(0.0), 1.0, "a standing car plays the recording at its own speed");
        assert!((rate(30.0) - 1.5).abs() < 1e-6);
        assert!(rate(60.0) > rate(30.0) && rate(150.0) > rate(60.0), "the motor rises with the speed");
        assert!((rate(300.0) - 2.2).abs() < 1e-6, "flat after the last point");
        let mut wild = curved("M.WAV", vec![curve(CurveInput::SpeedMph, &[(0.0, 1.0), (10.0, 1.0)])]);
        wild.rate = Some(curve(CurveInput::SpeedMph, &[(0.0, 0.0), (10.0, 8.0)]));
        let wild = sound(vec![wild]);
        assert_eq!(mix_inputs(&wild, &at(0.0, 0.0, 0.0))[0].rate, FOLLOW_RANGE.0);
        assert_eq!(mix_inputs(&wild, &at(0.0, 0.0, 10.0))[0].rate, FOLLOW_RANGE.1);
    }

    #[test]
    fn a_ginsu_layer_is_played_at_the_frequency_its_span_gives_the_engine_speed() {
        let mut ginsu = curved("G.GIN", vec![curve(CurveInput::Rpm, &[(0.0, 1.0), (1.0, 1.0)])]);
        ginsu.kind = LayerKind::Ginsu;
        ginsu.span = Some((1800.0, 7500.0));
        let engine = sound(vec![ginsu]);
        assert_eq!(mix_inputs(&engine, &at(0.0, 1.0, 0.0))[0].frequency, 1800.0);
        assert_eq!(mix_inputs(&engine, &at(1.0, 1.0, 0.0))[0].frequency, 7500.0);
        assert!((mix_inputs(&engine, &at(0.5, 1.0, 0.0))[0].frequency - 4650.0).abs() < 1e-3);
    }

    #[test]
    fn layers_mixed_by_speed_and_layers_with_curves_do_not_disturb_each_other() {
        let engine = sound(vec![
            EngineLayer::classic("A.WAV", 800.0),
            curved("W.WAV", vec![curve(CurveInput::Load, &[(0.0, 0.25), (1.0, 0.25)])]),
            EngineLayer::classic("B.WAV", 3200.0),
        ]);
        let mixed = mix_inputs(&engine, &at(0.0, 1.0, 0.0));
        assert_eq!((mixed[0].gain, mixed[1].gain, mixed[2].gain), (1.0, 0.25, 0.0));
        let power = |inputs: &EngineInputs| {
            let m = mix_inputs(&engine, inputs);
            m[0].gain * m[0].gain + m[2].gain * m[2].gain
        };
        assert!((power(&at(0.3, 1.0, 0.0)) - 1.0).abs() < 1e-4, "the two plain layers keep their equal power");
    }

    #[test]
    fn an_engine_with_no_plain_layer_mixes_nothing_by_speed() {
        let engine = sound(vec![curved("W.WAV", vec![curve(CurveInput::Load, &[(0.0, 1.0), (1.0, 1.0)])])]);
        assert_eq!(mix_inputs(&engine, &at(0.5, 0.5, 0.0)).len(), 1);
        assert!(by_speed(&[], 3000.0).is_empty());
    }
}
