//! Ordered telemetry/contact history for audio, independent of render-frame grouping.
use super::input::DriveInput;
use super::sim::{CarSim, Telemetry};
use super::walls::Impact;
use crate::audio::{CarContact, CarSoundState, CarSoundTick};
use blackbox_carsound::CarInput;
use nfsmw_data::car::physics::{PerformanceCategory, PerformancePackage, SurfaceTable};
use nfsmw_data::sound::SoundUpgrades;

#[derive(Debug, Default)]
pub struct SoundFeed {
    ticks: Vec<CarSoundTick>,
    camera_contacts: Vec<CarContact>,
    latest: CarInput,
    telemetry: crate::audio::CarAudioTelemetry,
    surface: u32,
}

impl SoundFeed {
    pub fn clear(&mut self) {
        self.ticks.clear();
        self.camera_contacts.clear();
    }
    fn append_world_contacts(&mut self, contacts: Vec<CarContact>, player_advanced: bool, dt: f32) {
        if !player_advanced {
            self.ticks.push(CarSoundTick {
                sim_dt: dt,
                telemetry: self.telemetry,
                input: self.latest,
                contacts,
                surface: self.surface,
            });
            return;
        }
        if let Some(tick) = self.ticks.last_mut() {
            tick.contacts.extend(contacts);
        }
    }

    pub fn after_step(
        &mut self,
        sim: &mut CarSim,
        surfaces: &SurfaceTable,
        impact: &Impact,
        input: &DriveInput,
        dt: f32,
    ) {
        let t = sim.telemetry();
        let span = t.red_line - t.idle;
        self.surface = sim.surface_tag().unwrap_or_else(|| SurfaceTable::hash_of("default"));
        self.latest = CarInput {
            rpm_pct: if span > 0.0 { ((t.rpm - t.idle) / span).clamp(0.0, 1.0) } else { 0.0 },
            throttle: input.throttle,
            brake: input.brake,
            gear: t.gear + 1,
            top_gear: Some(sim.sound_top_gear()),
            speed: sim.body().linear_velocity.length(),
            nos_active: t.nos_burning,
            nos_empty: input.nos && t.has_nos && t.nos <= 0.0,
            wheels: sim.wheel_sounds(surfaces),
            up_dot: sim.up_dot(),
            ..Default::default()
        };
        self.telemetry = sim.audio_telemetry(surfaces);
        let mut contacts = impact.sounds.clone();
        contacts.extend(sim.take_sound_contacts());
        self.camera_contacts.clone_from(&contacts);
        self.ticks.push(CarSoundTick {
            sim_dt: dt,
            telemetry: self.telemetry,
            input: self.latest,
            contacts,
            surface: self.surface,
        });
    }

    /// Latest completed physics tick, retained independently of the audio drain.
    pub(super) fn camera_contacts(&self) -> &[CarContact] {
        &self.camera_contacts
    }

    /// Reading the latest state never repeats contacts from an earlier physics tick.
    pub fn state(&mut self, car: &str, _t: &Telemetry, performance: PerformancePackage) -> CarSoundState {
        CarSoundState {
            telemetry: self.telemetry,
            car: car.to_owned(),
            upgrades: sound_upgrades(performance),
            input: self.latest,
            events: Vec::new(),
            scrape: None,
            surface: self.surface,
            ticks: std::mem::take(&mut self.ticks),
            view: Default::default(),
        }
    }
}

impl super::Drive {
    /// Nearby prop sounds belong to the completed tick, independently of player camera impacts.
    pub(in crate::scenes::world) fn append_world_sound_contacts(
        &mut self,
        contacts: Vec<CarContact>,
        player_advanced: bool,
        dt: f32,
    ) {
        if self.sim.is_none() {
            self.sound.clear();
            return;
        }
        self.sound.append_world_contacts(contacts, player_advanced, dt);
    }
}

fn sound_upgrades(performance: PerformancePackage) -> SoundUpgrades {
    SoundUpgrades {
        engine: i32::from(performance.get(PerformanceCategory::Engine)),
        transmission: i32::from(performance.get(PerformanceCategory::Transmission)),
        induction: i32::from(performance.get(PerformanceCategory::Induction)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn installed_engine_transmission_and_induction_select_the_sound_set() {
        let package = PerformancePackage { levels: [4, 3, 2, 1, 2, 3, 4], junkman: 0x7f };
        assert_eq!(sound_upgrades(package), SoundUpgrades { engine: 2, transmission: 1, induction: 3 });
        let unrelated = PerformancePackage { levels: [4, 4, 4, 0, 0, 0, 4], junkman: 0x7f };
        assert_eq!(sound_upgrades(unrelated), SoundUpgrades::default());
    }

    #[test]
    fn surrounding_contacts_keep_fixed_tick_order_without_entering_the_camera_feed() {
        use crate::audio::{ContactSource, ContactVehicle};
        use glam::Vec3;

        let mut feed = SoundFeed::default();
        for identity in [11, 12, 13] {
            feed.append_world_contacts(
                vec![CarContact {
                    sound_profile: 0,
                    vehicle: Some(ContactVehicle { identity, profile: 17 }),
                    audio_zone: None,
                    detached: false,
                    source: ContactSource::World,
                    identity: 0,
                    surface: 0,
                    point: Vec3::ZERO,
                    normal: Vec3::X,
                    velocity: Vec3::NEG_X * 10.0,
                    delta_v: 3.0,
                    front: false,
                    bottom: false,
                }],
                false,
                1.0 / 144.0,
            );
        }
        assert_eq!(feed.ticks.len(), 3, "traffic-only ticks must not merge into an old player tick");
        assert!(feed.ticks.iter().all(|tick| tick.sim_dt == 1.0 / 144.0));
        assert_eq!(
            feed.ticks.iter().map(|tick| tick.contacts[0].vehicle.unwrap().identity).collect::<Vec<_>>(),
            [11, 12, 13]
        );
        assert!(feed.camera_contacts().is_empty());
        let state = feed.state("BMWM3GTR", &Telemetry::default(), PerformancePackage::default());
        assert_eq!(state.ticks.len(), 3);
        assert!(feed.state("BMWM3GTR", &Telemetry::default(), PerformancePackage::default()).ticks.is_empty());
    }
}
