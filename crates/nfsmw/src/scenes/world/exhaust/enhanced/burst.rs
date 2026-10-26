//! A bounded set of event-driven flashes; no continuous emission or invented crackles.
use crate::garage::BackfireEvent;
use crate::settings::BackfireStyle;

#[derive(Clone, Default)]
pub(super) struct Bursts {
    live: Vec<Pulse>,
    serial: u32,
}

#[derive(Clone)]
struct Pulse {
    kind: BackfireEvent,
    age: f32,
    seed: u32,
    lifetime: f32,
    amplitude: f32,
}

pub(super) struct Frame {
    pub age: f32,
    pub phase: f32,
    pub seed: f32,
    pub strength: f32,
    pub length: f32,
    pub width: f32,
    pub turbulence: f32,
}

fn random(mut seed: u32) -> f32 {
    seed = (seed ^ (seed >> 16)).wrapping_mul(0x85eb_ca6b);
    seed = (seed ^ (seed >> 13)).wrapping_mul(0xc2b2_ae35);
    ((seed ^ (seed >> 16)) >> 8) as f32 / 16_777_216.0
}

impl Bursts {
    pub fn clear(&mut self) {
        self.live.clear();
    }
    pub fn trigger(&mut self, kind: BackfireEvent, style: &BackfireStyle) {
        if style.duration == 0 || style.size == 0 || style.brightness == 0 {
            return;
        }
        self.serial = self.serial.wrapping_add(1);
        let seed = match style.variation {
            0 => 0x9e37_79b9,
            _ => self.serial.wrapping_mul(0x9e37_79b9),
        };
        let variation = f32::from(style.variation) / 100.0;
        let base = match kind {
            BackfireEvent::Shift => 0.14,
            BackfireEvent::Pop => 0.085,
        };
        let lifetime = base * f32::from(style.duration) / 100.0 * (1.0 + (random(seed) - 0.5) * 0.3 * variation);
        if self.live.len() == 3 {
            self.live.remove(0);
        }
        self.live.push(Pulse {
            kind,
            age: 0.004,
            seed,
            lifetime,
            amplitude: 1.0 + (random(seed ^ 17) - 0.5) * 0.4 * variation,
        });
    }

    pub fn advance(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        for pulse in &mut self.live {
            pulse.age += dt;
        }
        self.live.retain(|pulse| pulse.age < pulse.lifetime);
    }

    pub fn frames<'a>(&'a self, style: &BackfireStyle, elapsed: f32) -> impl Iterator<Item = Frame> + 'a {
        let size = f32::from(style.size) / 100.0;
        self.live.iter().filter_map(move |pulse| {
            let age = pulse.age + elapsed;
            let phase = age / pulse.lifetime;
            if phase >= 1.0 {
                return None;
            }
            let attack = (age / 0.008).min(1.0);
            let strength = attack * (1.0 - phase).powf(1.4) * pulse.amplitude;
            let (length, width) = match pulse.kind {
                BackfireEvent::Shift => (0.48, 0.18),
                BackfireEvent::Pop => (0.28, 0.14),
            };
            let expansion = 1.0 - (-age * 65.0).exp();
            Some(Frame {
                age,
                phase,
                seed: random(pulse.seed ^ 31) * 19.19,
                strength,
                length: length * size * pulse.amplitude * (0.45 + 0.65 * expansion) * (1.0 - 0.2 * phase),
                width: width * size * (0.7 + 0.8 * phase),
                turbulence: 0.35 + 0.65 * phase,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flashes_require_events_and_expire_without_a_held_emission_signal() {
        let style = BackfireStyle::default();
        let mut bursts = Bursts::default();
        assert_eq!(bursts.frames(&style, 0.0).count(), 0);
        bursts.trigger(BackfireEvent::Pop, &style);
        let seed = bursts.frames(&style, 0.0).next().unwrap().seed;
        bursts.advance(0.01);
        assert_eq!(bursts.frames(&style, 0.0).next().unwrap().seed, seed);
        bursts.advance(1.0);
        assert_eq!(bursts.frames(&style, 0.0).count(), 0);
    }

    #[test]
    fn rapid_events_are_bounded_and_zero_variation_repeats_the_same_flash() {
        let style = BackfireStyle { variation: 0, ..Default::default() };
        let mut bursts = Bursts::default();
        for _ in 0..10 {
            bursts.trigger(BackfireEvent::Shift, &style);
        }
        let frames: Vec<_> = bursts.frames(&style, 0.0).collect();
        assert_eq!(frames.len(), 3);
        assert!(frames.iter().all(|frame| frame.seed == frames[0].seed && frame.length == frames[0].length));
    }
}
