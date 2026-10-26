//! A manifest's engine layer as the layer the audio plays.

use blackbox_modding::content::{EngineCurve, EngineInput, EngineLayer as ManifestLayer, EngineLayerKind};
use nfsmw_data::car::mods::{CurveInput, EngineLayer, LayerCurve, LayerKind, LayerPitch};

fn input(input: EngineInput) -> CurveInput {
    match input {
        EngineInput::Rpm => CurveInput::Rpm,
        EngineInput::Load => CurveInput::Load,
        EngineInput::SpeedMph => CurveInput::SpeedMph,
    }
}

fn curve(curve: &EngineCurve) -> LayerCurve {
    LayerCurve { input: input(curve.input), points: curve.points.clone() }
}

/// The layer `layer` of `[engine_sound]`.
pub(super) fn layer_of(layer: &ManifestLayer) -> EngineLayer {
    EngineLayer {
        file: layer.file.clone(),
        rpm: layer.rpm,
        kind: match layer.kind {
            EngineLayerKind::Loop => LayerKind::Loop,
            EngineLayerKind::Ginsu => LayerKind::Ginsu,
        },
        gain: layer.gain.iter().map(curve).collect(),
        pitch: layer.pitch.map(|pitch| LayerPitch { input: input(pitch.input), reference: pitch.reference }),
        span: layer.span,
        volume: layer.volume,
    }
}
