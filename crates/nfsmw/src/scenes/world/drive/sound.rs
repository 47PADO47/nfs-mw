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
    visual_rpm_fraction: Option<f32>,
}

impl SoundFeed {
    pub fn clear(&mut self) {
        self.ticks.clear();
        self.camera_contacts.clear();
        self.visual_rpm_fraction = None;
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
        pre_race: bool,
    ) {
        let t = sim.telemetry();
        let display_rpm = sim.gauge_sample().rpm;
        self.surface = sim.surface_tag().unwrap_or_else(|| SurfaceTable::hash_of("default"));
        self.latest = CarInput {
            rpm_pct: physics_rpm_fraction(display_rpm, t.idle, t.red_line),
            throttle: input.throttle,
            brake: input.brake,
            gear: t.gear + 1,
            top_gear: Some(sim.sound_top_gear()),
            speed: sim.body().linear_velocity.length(),
            nos_active: t.nos_burning,
            nos_empty: input.nos && t.has_nos && t.nos <= 0.0,
            wheels: sim.wheel_sounds(surfaces),
            up_dot: sim.up_dot(),
            pre_race,
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
            nitrous_fx: None,
            wind_fx: None,
            engine: Default::default(),
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
    /// Audio controllers return a display value; physical telemetry remains untouched.
    pub(in crate::scenes::world) fn note_visual_rpm(&mut self, fraction: Option<f32>) {
        self.sound.visual_rpm_fraction = fraction.filter(|f| f.is_finite() && (0.0..=1.0).contains(f));
    }

    pub(super) fn clear_visual_rpm(&mut self) {
        self.sound.visual_rpm_fraction = None;
    }

    /// Falls back to the engine's displayed RPM when the audio voice has no valid controller output.
    pub(super) fn hud_visual_rpm(&self, idle: f32, redline: f32, fallback: f32) -> f32 {
        visual_rpm(self.sound.visual_rpm_fraction, idle, redline, fallback)
    }

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

fn visual_rpm(fraction: Option<f32>, idle: f32, redline: f32, fallback: f32) -> f32 {
    let Some(fraction) = fraction.filter(|f| f.is_finite() && (0.0..=1.0).contains(f)) else { return fallback };
    if !idle.is_finite() || !redline.is_finite() || redline <= idle {
        return fallback;
    }
    idle + fraction * (redline - idle)
}

fn physics_rpm_fraction(display_rpm: f32, idle: f32, redline: f32) -> f32 {
    let span = redline - idle;
    if span <= 0.0 {
        return 0.0;
    }
    ((display_rpm - idle) / span).clamp(0.0, 1.0)
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
    fn visual_feedback_maps_the_original_idle_redline_span_and_falls_back_without_a_voice() {
        assert_eq!(visual_rpm(Some(0.5), 800.0, 7000.0, 900.0), 3900.0);
        for fraction in [None, Some(f32::NAN), Some(-0.1), Some(1.1)] {
            assert_eq!(visual_rpm(fraction, 800.0, 7000.0, 900.0), 900.0);
        }
        assert_eq!(visual_rpm(Some(0.5), 800.0, 800.0, 900.0), 900.0);
        let mut feed = SoundFeed { visual_rpm_fraction: Some(0.75), ..Default::default() };
        feed.clear();
        assert_eq!(feed.visual_rpm_fraction, None, "a cleared/replaced car must not retain feedback");
    }

    #[test]
    fn audio_reads_displayed_engine_rpm_without_replacing_raw_telemetry() {
        use blackbox_vehicle::{VehicleSpec, ground::NoGround};
        use nfsmw_data::car::physics::{CarBounds, CarPhysics, WallSpec};
        let spec = VehicleSpec::example();
        let bounds = CarBounds { half_dimensions: spec.dimension, pivot: glam::Vec3::ZERO };
        let mut sim = CarSim::new(
            CarPhysics {
                effect_source: 0,
                spec,
                bounds,
                tire_integration: Default::default(),
                jump_config: Default::default(),
                walls: WallSpec::default(),
                geometry: None,
                events: None,
                reactions: None,
            },
            [0.2; 4],
        );
        let input = DriveInput { throttle: 1.0, ..Default::default() };
        let dt = 1.0 / 60.0;
        let impact = sim.step_with_delta(dt, &input, &NoGround, None);
        let telemetry = sim.telemetry();
        let displayed = sim.gauge_sample().rpm;
        assert_ne!(displayed, telemetry.rpm, "the fixture must distinguish display RPM from raw engine speed");
        let mut feed = SoundFeed::default();
        feed.after_step(&mut sim, &SurfaceTable::default(), &impact, &input, dt, true);
        assert_eq!(feed.latest.rpm_pct, physics_rpm_fraction(displayed, telemetry.idle, telemetry.red_line));
        assert_ne!(feed.latest.rpm_pct, physics_rpm_fraction(telemetry.rpm, telemetry.idle, telemetry.red_line));
        assert!(feed.ticks[0].input.pre_race);
        assert_eq!(sim.telemetry().rpm, telemetry.rpm, "publishing audio input must not rewrite physics telemetry");
    }
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
