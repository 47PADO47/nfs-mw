//! The audio data of one bike, resolved from its RIDE 6 data assets (`docs/formats/ride6-audio.md`).
//!
//! The resolved file is a small JSON the user's own tooling writes from their install (the scratch resolver reads the
//! Unreal packages; nothing in it is audio). It names the Wwise events a bike plays and the numbers of its
//! backfire. This module reads it and merges the bike's own numbers over the defaults of the base data asset, the way the
//! game's property inheritance does.

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;

/// A Wwise bank the event lives in.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Bank {
    pub name: String,
    pub id: u32,
    /// The bank's packaged path, e.g. `WwiseAudio/Event/<name>.bnk`.
    pub file: String,
    pub has_media: bool,
    pub kind: String,
}

/// A stream of audio an event plays, by its Wwise media id.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Media {
    pub id: u32,
    /// The source recording's name in the authoring tool, e.g. `Backfire_<bike>_01.wav`.
    pub name: String,
    /// The packaged path, `WwiseAudio/Media/<id>.wem`.
    pub file: String,
}

/// One Wwise event a bike plays.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Event {
    pub name: String,
    pub event_id: u32,
    pub banks: Vec<Bank>,
    /// The media the event lists itself; an engine event lists none (its media are inside its user bank).
    pub media: Vec<Media>,
    pub min_duration_s: Option<f32>,
    pub max_duration_s: Option<f32>,
    pub infinite: Option<bool>,
}

/// The events of a bike, by the role the game gives them.
#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Events {
    pub engine_event: Option<Event>,
    pub exhaust_event: Option<Event>,
    pub backfire_bang_event: Option<Event>,
    pub backfire_flame_event: Option<Event>,
    pub gear_up_change_event: Option<Event>,
    pub gear_down_change_event: Option<Event>,
    pub supercharger_event: Option<Event>,
}

/// The resolved audio data of one chassis and variant (`Stock` or `Custom`).
#[derive(Debug, Clone, Deserialize)]
pub struct BikeAudioData {
    pub chassis: String,
    pub variant: String,
    pub data_asset: String,
    pub events: Events,
    audio_effects_default: Value,
    audio_effects_override: Option<Value>,
}

/// The numbers of the overrun backfire (section "Backfire" of `docs/specs/ride6-engine-sound.md`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OverrunParams {
    pub simulation_enabled: bool,
    pub new_system: bool,
    pub accumulator_rate: f32,
    pub throttle_arm: f32,
    pub throttle_fire: f32,
    pub window_seconds: f32,
    pub flame_delay: f32,
    pub flame_min_rpm: f32,
    pub bang_audio_threshold: f32,
    pub bang_effect_threshold: f32,
    pub flame_effect_threshold: f32,
    /// The chance of a bang at a shut driver throttle and at a fully open one.
    pub probability_closed: f32,
    pub probability_open: f32,
}

impl OverrunParams {
    /// Whether the bike backfires by the accumulator system the `[car_sound.overrun]` table describes: its simulation is on
    /// and it uses the new system. A bike with the simulation off never bangs, and the one bike that uses the old system is
    /// left to the core rule (spec sections 5.3 and 5.4).
    pub fn applies(&self) -> bool {
        self.simulation_enabled && self.new_system
    }
}

impl BikeAudioData {
    /// Read the resolved JSON.
    pub fn from_json(text: &str) -> Result<Self> {
        serde_json::from_str(text).context("the resolved bike audio JSON")
    }

    /// One number of a table of the effects: the bike's own value, else the base asset's.
    fn effect(&self, table: &str, key: &str) -> Option<&Value> {
        let own = self.audio_effects_override.as_ref().and_then(|effects| effects.get(table)).and_then(|t| t.get(key));
        own.or_else(|| self.audio_effects_default.get(table).and_then(|t| t.get(key)))
    }

    fn number(&self, table: &str, key: &str) -> Result<f32> {
        let value = self.effect(table, key).with_context(|| format!("{table}.{key} is in neither data asset"))?;
        let Some(number) = value.as_f64() else { anyhow::bail!("{table}.{key} is not a number") };
        Ok(number as f32)
    }

    fn flag(&self, table: &str, key: &str) -> Result<bool> {
        self.effect(table, key).and_then(Value::as_bool).with_context(|| format!("{table}.{key} is not a flag"))
    }

    /// The backfire numbers, the bike's own over the base asset's.
    pub fn overrun(&self) -> Result<OverrunParams> {
        const T: &str = "BikeEffectsParametersValue";
        let range = |axis: &str| -> Result<f32> {
            let value = self.effect(T, "BackfireBangProbabilityRange").and_then(|range| range.get(axis));
            let number =
                value.and_then(Value::as_f64).with_context(|| format!("BackfireBangProbabilityRange.{axis}"))?;
            Ok(number as f32)
        };
        Ok(OverrunParams {
            simulation_enabled: self.flag(T, "bBackfireSimulationEnabled")?,
            new_system: self.flag(T, "bNewBackfireSystemEnabled")?,
            accumulator_rate: self.number(T, "AccumulatorDeltaFactor")?,
            throttle_arm: self.number(T, "BackfireEffectThrottleInitialThreshold")?,
            throttle_fire: self.number(T, "BackfireEffectThrottleTriggerThreshold")?,
            window_seconds: self.number(T, "BackfireMaximumDeltaTime")?,
            flame_delay: self.number(T, "FlameEffectDelayTime")?,
            flame_min_rpm: self.number(T, "FlameEffectMinRPM")?,
            bang_audio_threshold: self.number(T, "BackfireBangAudiotIntensityThreshold")?,
            bang_effect_threshold: self.number(T, "BackfireBangParticlesIntensityThreshold")?,
            flame_effect_threshold: self.number(T, "BackfireFlameParticlesIntensityThreshold")?,
            probability_closed: range("X")?,
            probability_open: range("Y")?,
        })
    }

    /// Whether the bike's variant plays a bang recording at all (`Stock` bikes mostly do not).
    pub fn has_bang_sound(&self) -> bool {
        self.events.backfire_bang_event.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
      "chassis": "X", "variant": "Custom", "data_asset": "/Game/a/B",
      "events": { "EngineEvent": null, "BackfireBangEvent": {
        "name": "Backfire_X", "event_id": 1, "banks": [], "media": [
          {"id": 7, "name": "Backfire_X_01.wav", "file": "WwiseAudio/Media/7.wem"}],
        "min_duration_s": 0.02, "max_duration_s": 0.05, "infinite": false } },
      "audio_effects_default": { "BikeEffectsParametersValue": {
        "bBackfireSimulationEnabled": false, "bNewBackfireSystemEnabled": false,
        "AccumulatorDeltaFactor": 0.2, "BackfireEffectThrottleInitialThreshold": 0.9,
        "BackfireEffectThrottleTriggerThreshold": 0.1, "BackfireMaximumDeltaTime": 2.0,
        "FlameEffectDelayTime": 0.2, "FlameEffectMinRPM": 0.1,
        "BackfireBangAudiotIntensityThreshold": 0.85, "BackfireBangParticlesIntensityThreshold": 0.95,
        "BackfireFlameParticlesIntensityThreshold": 0.7,
        "BackfireBangProbabilityRange": {"X": 0.8, "Y": 0.3} } },
      "audio_effects_override": { "BikeEffectsParametersValue": {
        "bBackfireSimulationEnabled": true, "bNewBackfireSystemEnabled": true,
        "BackfireBangAudiotIntensityThreshold": 0.95 } }
    }"#;

    #[test]
    fn the_bikes_numbers_win_over_the_base_asset() {
        let data = BikeAudioData::from_json(SAMPLE).unwrap();
        let p = data.overrun().unwrap();
        assert!(p.simulation_enabled && p.new_system && p.applies());
        assert_eq!(p.bang_audio_threshold, 0.95);
        assert_eq!(p.accumulator_rate, 0.2);
        assert_eq!((p.probability_closed, p.probability_open), (0.8, 0.3));
        assert!(data.has_bang_sound());
        assert_eq!(data.events.backfire_bang_event.unwrap().media[0].id, 7);
    }

    #[test]
    fn a_bike_with_the_simulation_off_or_the_old_system_gets_no_table() {
        let off = SAMPLE.replace("\"bBackfireSimulationEnabled\": true", "\"bBackfireSimulationEnabled\": false");
        assert!(!BikeAudioData::from_json(&off).unwrap().overrun().unwrap().applies());
        let old = SAMPLE.replace("\"bNewBackfireSystemEnabled\": true", "\"bNewBackfireSystemEnabled\": false");
        assert!(!BikeAudioData::from_json(&old).unwrap().overrun().unwrap().applies());
    }

    #[test]
    fn a_missing_number_is_an_error() {
        let text = SAMPLE.replace("\"AccumulatorDeltaFactor\": 0.2,", "");
        assert!(BikeAudioData::from_json(&text).unwrap().overrun().is_err());
    }
}
