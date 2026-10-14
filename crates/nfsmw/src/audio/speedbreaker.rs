//! Real-time authored pitch envelope. Spec: `speedbreaker-audio.md`.
use super::{Audio, trace::Record};
use blackbox_mixmap::{EventOutput, InputKey, MixMap, Mixer};
use std::sync::Arc;

const MAP: &str = "SOUND/MIXMAPS/MAPOUTPUT.mxb";

#[derive(Default)]
pub(super) struct Slot {
    mixer: Option<Mixer>,
    failed: bool,
    previous: EventOutput,
    timeline: super::aems::timeline::Timeline,
}

impl Slot {
    pub fn output(&self) -> Option<EventOutput> {
        self.mixer.as_ref().and_then(|mixer| mixer.event_output(0, 0, 2)).or(Some(EventOutput::default()))
    }
}

impl Audio {
    pub(super) fn update_speedbreaker(&mut self, active: bool, dt: f32) {
        if self.speedbreaker.failed || !dt.is_finite() || dt <= 0.0 {
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
            self.speedbreaker.mixer = Some(Mixer::new(Arc::new(map), &[1]));
        }
        self.speedbreaker.timeline.push(dt, &[i32::from(active) * 32767]);
        let mixer = self.speedbreaker.mixer.as_mut().unwrap();
        for _ in 0..480 {
            let Some((_, words)) = self.speedbreaker.timeline.next() else { break };
            mixer.set_input(InputKey::controller(0, 0, 0, 8), words[0]);
            mixer.process(1.0 / 60.0);
        }
        let Some(output) = mixer.event_output(0, 0, 2) else {
            self.speedbreaker.failed = true;
            self.trace.record(Record {
                event: "module_unavailable",
                class: "SPEEDBREAKER_AUDIO",
                detail: "authored main pitch event absent",
                ..Default::default()
            });
            return;
        };
        if active || output != self.speedbreaker.previous {
            self.trace.record(Record {
                event: "module_tick",
                class: "SPEEDBREAKER_AUDIO",
                bank: MAP,
                params: &[i32::from(active) * 32767, output.position, output.level],
                logical_ms: self.trace.now_ms(),
                success: Some(true),
                ..Default::default()
            });
        }
        self.speedbreaker.previous = output;
    }
}

#[cfg(test)]
mod tests;
