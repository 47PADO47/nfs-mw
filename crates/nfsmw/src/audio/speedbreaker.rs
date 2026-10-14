//! Shared real-time authored mode envelopes. Spec: `speedbreaker-audio.md`.
use super::{Audio, trace::Record};
use blackbox_mixmap::{EventOutput, InputKey, MixMap, Mixer};
use std::sync::Arc;

const MAP: &str = "SOUND/MIXMAPS/MAPOUTPUT.mxb";
const EVENTS: [u8; 6] = [0, 2, 6, 7, 8, 11];
pub(super) type Mode = [EventOutput; 6];

#[derive(Default)]
pub(super) struct Slot {
    mixer: Option<Mixer>,
    failed: bool,
    previous: Mode,
    moment: moment::Moment,
    pub(in crate::audio) view: super::AudioView,
    timeline: super::aems::timeline::Timeline,
}

impl Slot {
    #[cfg(test)]
    pub fn output(&self) -> Option<EventOutput> {
        self.mixer.as_ref().and_then(|mixer| mixer.event_output(0, 0, 2)).or(Some(EventOutput::default()))
    }

    pub fn snapshot(&self) -> Option<Mode> {
        let mut result = Mode::default();
        if let Some(mixer) = &self.mixer {
            for (index, event) in EVENTS.into_iter().enumerate() {
                result[index] = mixer.event_output(0, 0, event)?;
            }
        }
        Some(result)
    }
}

pub(super) fn publish(mixer: &mut Mixer, view: &super::AudioView) {
    mixer.set_input(InputKey::controller(0, 0, 0, 8), i32::from(view.speedbreaker) * 32767);
    for (index, event) in EVENTS.into_iter().enumerate() {
        mixer.override_event(0, 0, event, view.speedbreaker_mix.map(|outputs| outputs[index]));
    }
}

impl Audio {
    pub(super) fn update_speedbreaker(&mut self, active: bool, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        if self.speedbreaker.failed {
            return;
        }
        if self.speedbreaker.mixer.is_none() {
            if !active {
                return;
            }
            let loaded = self
                .dir
                .read(MAP)
                .map_err(|e| e.to_string())
                .and_then(|bytes| MixMap::parse(&bytes).map_err(|e| e.to_string()));
            let map = match loaded {
                Ok(map) => map,
                Err(error) => {
                    self.speedbreaker.failed = true;
                    self.trace.record(Record {
                        event: "module_unavailable",
                        class: "SPEEDBREAKER_AUDIO",
                        detail: &error,
                        ..Default::default()
                    });
                    log::warn!("Speedbreaker pitch map unavailable: {error}");
                    return;
                }
            };
            let mut mixer = Mixer::new(Arc::new(map), &[1, 1]);
            mixer.attach(blackbox_mixmap::ObjectRef { state: 0, instance: 0, object: 6 }, true);
            mixer.attach(blackbox_mixmap::ObjectRef { state: 1, instance: 0, object: 1 }, true);
            self.speedbreaker.mixer = Some(mixer);
        }
        self.speedbreaker.timeline.push(dt, &[i32::from(active) * 32767]);
        let mixer = self.speedbreaker.mixer.as_mut().unwrap();
        for _ in 0..480 {
            let Some((_, words)) = self.speedbreaker.timeline.next() else { break };
            moment::feed_position(mixer, self.speedbreaker.view);
            mixer.set_input(InputKey::controller(0, 0, 0, 8), words[0]);
            mixer.process(1.0 / 60.0);
        }
        let music = blackbox_mixmap::ObjectRef { state: 1, instance: 0, object: 1 };
        if let Some(output) = &self.output {
            output.music_filter.set(mixer.filter(music, 9).unwrap_or(65_535.0));
        }
        let Some(outputs) = self.speedbreaker.snapshot() else {
            self.speedbreaker.failed = true;
            self.trace.record(Record {
                event: "module_unavailable",
                class: "SPEEDBREAKER_AUDIO",
                detail: "authored breaker event absent",
                ..Default::default()
            });
            return;
        };
        for (index, output) in outputs.into_iter().enumerate() {
            if !active && output == self.speedbreaker.previous[index] {
                continue;
            }
            self.trace.record(Record {
                event: "module_tick",
                class: "SPEEDBREAKER_AUDIO",
                bank: MAP,
                params: &[i32::from(active) * 32767, output.position, output.level, i32::from(EVENTS[index])],
                logical_ms: self.trace.now_ms(),
                success: Some(true),
                ..Default::default()
            });
        }
        self.speedbreaker.previous = outputs;
        self.update_breaker_moment(active, dt);
    }
}

mod moment;
#[cfg(test)]
mod tests;
