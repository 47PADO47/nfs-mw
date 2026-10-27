//! The layers of `[engine_sound]`: layers mixed by engine speed and layers that play by curves.

use serde::Deserialize;

use super::ContentError;
use super::engine_curves::{Input, LayerKind};
use super::engine_sound::{self, EngineSound, Raw};

#[derive(Deserialize)]
struct File {
    engine_sound: Option<Raw>,
}

fn parse(layers: &str) -> Result<EngineSound, ContentError> {
    let text = format!("[engine_sound]\nidle_rpm = 800.0\nredline_rpm = 7500.0\nlayers = [\n{layers}\n]\n");
    let file: File = toml::from_str(&text).map_err(|error| ContentError::Parse(error.to_string()))?;
    Ok(engine_sound::check(file.engine_sound)?.unwrap())
}

const RISE: &str = "gain = [ { input = \"load\", points = [[0.0, 0.0], [1.0, 1.0]] } ]";

#[test]
fn a_layer_with_an_rpm_and_no_curves_is_mixed_by_engine_speed_as_before() {
    let sound = parse("{ file = \"SOUND/B.WAV\", rpm = 5000.0 }, { file = \"SOUND/A.WAV\", rpm = 900.0 }").unwrap();
    assert_eq!(sound.layers.iter().map(|layer| layer.rpm).collect::<Vec<_>>(), [900.0, 5000.0]);
    assert!(
        sound.layers.iter().all(|layer| layer.gain.is_empty() && layer.kind == LayerKind::Loop && layer.volume == 1.0)
    );
}

#[test]
fn a_layer_with_curves_needs_no_rpm_and_keeps_its_curves_pitch_and_volume() {
    let text = format!(
        "{{ file = \"SOUND/W.WAV\", volume = 0.6, {RISE}, pitch = {{ input = \"speed_mph\", reference = 40.0 }} }}"
    );
    let layer = parse(&text).unwrap().layers.remove(0);
    assert_eq!((layer.rpm, layer.volume), (0.0, 0.6));
    assert_eq!(layer.gain[0].input, Input::Load);
    assert_eq!(layer.pitch.map(|pitch| (pitch.input, pitch.reference)), Some((Input::SpeedMph, 40.0)));
}

#[test]
fn a_ginsu_layer_is_a_gin_file_with_a_span() {
    let text = format!("{{ file = \"SOUND/G0.GIN\", kind = \"ginsu\", span = [1790.0, 7500.0], {RISE} }}");
    let layer = parse(&text).unwrap().layers.remove(0);
    assert_eq!((layer.kind, layer.span), (LayerKind::Ginsu, Some((1790.0, 7500.0))));
}

#[test]
fn bad_layers_are_refused() {
    for (edit, layer) in [
        ("no rpm and no curves", "{ file = \"SOUND/A.WAV\" }".to_owned()),
        (
            "a Ginsu layer with a WAV",
            format!("{{ file = \"SOUND/G.WAV\", kind = \"ginsu\", span = [1000.0, 2000.0], {RISE} }}"),
        ),
        ("a Ginsu layer with no span", format!("{{ file = \"SOUND/G.GIN\", kind = \"ginsu\", {RISE} }}")),
        ("a span of a loop", format!("{{ file = \"SOUND/G.WAV\", span = [1000.0, 2000.0], {RISE} }}")),
        ("a reversed span", format!("{{ file = \"SOUND/G.GIN\", kind = \"ginsu\", span = [2000.0, 1000.0], {RISE} }}")),
        (
            "a Ginsu layer with no curves",
            "{ file = \"SOUND/G.GIN\", kind = \"ginsu\", rpm = 900.0, span = [1000.0, 2000.0] }".to_owned(),
        ),
        ("a volume of 9", format!("{{ file = \"SOUND/A.WAV\", volume = 9.0, {RISE} }}")),
        (
            "an unknown input",
            "{ file = \"SOUND/A.WAV\", gain = [ { input = \"boost\", points = [[0.0, 0.0], [1.0, 1.0]] } ] }"
                .to_owned(),
        ),
    ] {
        assert!(parse(&layer).is_err(), "{edit}");
    }
}

#[test]
fn two_layers_mixed_by_one_rpm_are_refused_but_curve_layers_share_none() {
    assert!(parse("{ file = \"SOUND/A.WAV\", rpm = 900.0 }, { file = \"SOUND/B.WAV\", rpm = 900.0 }").is_err());
    let both = format!("{{ file = \"SOUND/A.WAV\", {RISE} }}, {{ file = \"SOUND/B.WAV\", {RISE} }}");
    assert_eq!(parse(&both).unwrap().layers.len(), 2);
}
