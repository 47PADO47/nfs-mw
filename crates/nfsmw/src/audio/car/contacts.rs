//! Persistent authored scraping and data-driven impacts from physical contact records.
use super::{CarContact, ContactSource};
use crate::audio::aems::{EffectModule, EffectSampler};
use crate::audio::mixer::{
    CarMixer,
    collision::{CollisionFrame, CollisionMixer},
};
use crate::audio::{Audio, AudioView, CarEvent, Group};
use blackbox_carsound::CarInput;
use nfsmw_data::car::physics::SurfaceTable;
use nfsmw_data::sound::{CarSound, ImpactSound};
use std::collections::HashMap;

type Key = (ContactSource, u64, u32, bool, bool);
const SCRAPE_BANK: &str = "IG_GLOBAL/FX_Misc_MB.abk";

struct Scrape {
    module: EffectModule,
    mixer: Option<CollisionMixer>,
    sound: ImpactSound,
    contact: CarContact,
    magnitude: f32,
    timeout: f32,
    release: Option<f32>,
}

#[derive(Default)]
pub(super) struct ContactPlayback {
    scrapes: HashMap<Key, Scrape>,
    failed: bool,
}

/// Keep the strongest response for repeated shape/probe observations of one source/material.
pub(super) fn coalesced(contacts: &[CarContact]) -> Vec<CarContact> {
    let mut out: Vec<CarContact> = Vec::new();
    for &contact in contacts {
        let key = key(contact);
        if let Some(prior) = out.iter_mut().find(|c| self::key(**c) == key) {
            if contact.delta_v > prior.delta_v
                || (contact.delta_v == prior.delta_v && contact.sliding_speed() > prior.sliding_speed())
            {
                *prior = contact;
            }
            continue;
        }
        out.push(contact);
    }
    out
}

fn key(c: CarContact) -> Key {
    (c.source, c.identity, c.surface, c.bottom, c.front)
}

impl ContactPlayback {
    #[cfg(test)]
    pub fn active_count(&self) -> usize {
        self.scrapes.len()
    }
    pub fn observe(
        &mut self,
        audio: &mut Audio,
        sound: &CarSound,
        mixer: Option<&CarMixer>,
        contacts: &[CarContact],
        sim_dt: f32,
    ) -> Vec<CarEvent> {
        for scrape in self.scrapes.values_mut() {
            if scrape.release.is_some() {
                continue;
            }
            scrape.timeout -= sim_dt;
            if scrape.timeout < 0.0 {
                scrape.release = Some(0.25);
            }
        }
        let default = SurfaceTable::hash_of("default");
        let mut events = Vec::new();
        for contact in coalesced(contacts) {
            let severity = if contact.bottom { contact.closing_speed() } else { contact.delta_v };
            if contact.velocity.length_squared() > 1.0
                && let Some(rule) = sound.collision.rule(contact.hit_kind(), contact.surface, default, contact.front)
                && let Some(magnitude) = rule.magnitude(severity).filter(|m| *m > 0.0)
            {
                events.push(CarEvent {
                    kind: contact.hit_kind(),
                    surface: contact.surface,
                    magnitude,
                    front: contact.front,
                    smackable: rule.sound.has("SMOKABLE"),
                    position: contact.render_point(),
                });
            }
            let speed = contact.sliding_speed();
            if speed <= 1.0 {
                continue;
            }
            let key = key(contact);
            let rule = sound.collision.rule(contact.scrape_kind(), contact.surface, default, contact.front);
            let eligible = rule.and_then(|rule| rule.magnitude(speed).map(|m| (rule.sound, m.max(0.1))));
            let Some((selected, magnitude)) = eligible.filter(|(s, _)| !s.scrape_effect.is_empty()) else {
                if let Some(scrape) = self.scrapes.get_mut(&key) {
                    scrape.release.get_or_insert(0.25);
                }
                continue;
            };
            if let Some(scrape) = self.scrapes.get_mut(&key) {
                scrape.contact = contact;
                scrape.sound = selected.clone();
                scrape.magnitude = magnitude;
                scrape.timeout = 0.2;
                scrape.release = None;
                continue;
            }
            if self.failed {
                continue;
            }
            let module =
                match EffectModule::load(audio, SCRAPE_BANK, "FX_Scrape", contact.identity as u32 ^ contact.surface) {
                    Ok(module) => module,
                    Err(error) => {
                        log::warn!("authored scrape unavailable: {error}");
                        self.failed = true;
                        continue;
                    }
                };
            self.scrapes.insert(
                key,
                Scrape {
                    module,
                    mixer: mixer.map(CarMixer::collision),
                    sound: selected.clone(),
                    contact,
                    magnitude,
                    timeout: 0.2,
                    release: None,
                },
            );
        }
        events
    }

    pub fn update(&mut self, audio: &mut Audio, dt: f32, input: &CarInput, view: &AudioView) {
        self.scrapes.retain(|_, scrape| {
            if let Some(left) = &mut scrape.release {
                *left -= dt;
                if *left < 0.0 {
                    return false;
                }
            }
            let levels = scrape.mixer.as_mut().map(|m| {
                m.update(
                    dt,
                    CollisionFrame {
                        sound: &scrape.sound,
                        magnitude: scrape.magnitude,
                        point: scrape.contact.render_point(),
                        view,
                        input,
                        scrape: true,
                    },
                )
            });
            let volume = levels.map_or(1.0, |l| l.volume);
            let pitch = levels.map_or(1.0, |l| l.pitch);
            let azimuth = levels.map_or(0, |l| l.azimuth);
            let wet = levels.map_or(0.0, |l| l.wet);
            let params = [
                (volume * 32767.0) as i32,
                (pitch * 4096.0).clamp(0.0, 9000.0) as i32,
                i32::from(azimuth),
                0,
                (scrape.magnitude * 127.0) as i32,
                25_000,
                0,
                32767,
                (wet * 32767.0) as i32,
            ];
            scrape.module.update(&mut EffectSampler(audio, Group::Sfx), dt, &params)
        });
    }
}
