//! Ordered controller updates and the shared ownership router.
use super::*;

impl Audio {
    pub(super) fn advance_car(&mut self, car: &mut CarAudio, frame: &DriveFrame<'_>) {
        let view = super::super::AudioView {
            environment: frame.telemetry.environment.unwrap_or(frame.view.environment),
            ..*frame.view
        };
        car.event_delay = frame.delay;
        let sample_available = car.aems.as_ref().is_some_and(AemsLayer::engine_available);
        if car.sample_available && !sample_available {
            log::warn!("{}: engine sample layer failed; Ginsu remains audible at the limiter", car.car);
        }
        car.sample_available = sample_available;
        car.engine.set_redline_sample_available(sample_available);
        // Keep the recording-range ratio separate from the dynamic mixer's pitch.
        let input = CarInput { pitch_multiplier: 1.0, ..*frame.input };
        let out = car.engine.update(frame.dt, &input);
        let mut commands = std::mem::take(&mut car.commands);
        commands.clear();
        let landing = car.effects.update(frame.dt, &input, &out, &mut commands);
        if let Some(mixer) = car.mixer.as_mut() {
            mixer.set_view(view);
            mixer.set_landing(landing);
            let sparks = car.aems.as_ref().is_some_and(AemsLayer::sparks);
            car.levels = mixer.update(
                frame.dt,
                &Frame {
                    input: &input,
                    engine: &out,
                    effects: car.effects.signals(),
                    master_volume: car.master_volume,
                    sparks,
                },
            );
        }
        let wet_supported = self.update_wet(car.levels.environment.requested_patch);
        car.levels.environment.wet_supported = wet_supported;
        car.handle.set_wet_gain(car.levels.environment.ginsu_wet as f32 / 32767.0);
        let levels = car.levels;
        car.handle.set_tick(
            EngineMix {
                accel: LoopMix {
                    frequency: out.accel_loop.frequency,
                    volume: out.accel_volume * levels.engine_volume,
                    pitch: out.accel_loop.playback_rate * levels.engine_pitch,
                },
                decel: LoopMix {
                    frequency: out.decel_loop.frequency,
                    volume: out.decel_volume * levels.engine_volume,
                    pitch: out.decel_loop.playback_rate * levels.engine_pitch,
                },
                low_pass_hz: out.low_pass_hz.min(levels.engine_low_pass_hz),
                azimuth: levels.engine_azimuth as u16,
                dry: (levels.environment.ginsu_dry.clamp(0, 32767) >> 8) as f32 / 127.0,
            },
            out.tick_revision,
        );
        if out.redlining != car.redlining {
            car.redlining = out.redlining;
            log::debug!("{}: engine limiter {}", car.car, if out.redlining { "starts" } else { "ends" });
        }
        if let Some(layer) = car.aems.as_mut() {
            let below_min = if out.accel_loop.frequency > 0.0 {
                (out.ginsu_frequency / out.accel_loop.frequency).min(1.0)
            } else {
                1.0
            };
            layer.set_synth_frequency(car.handle.current_frequency() * below_min);
            layer.update(&mut KiraSampler(self), frame.dt, &Feed { engine: &out, levels: &levels });
        }
        let hits = car.contacts.observe(self, &car.sound, car.mixer.as_ref(), frame.contacts, frame.sim_dt);
        for event in hits.iter().chain(frame.events) {
            self.play_impact(car, event, &input, &view);
        }
        if let Some(landing) = landing {
            self.play_landing(car, landing);
        }
        let landing_level = car.mixer.as_ref().map_or((1.0, 0), CarMixer::landing_level);
        car.impacts.update(self, frame.dt, frame.delay, &input, &view, landing_level);
        car.contacts.update(self, frame.dt, &input, &view);
        let tyre_frame = tyres::TyreFrame {
            understeer: frame.telemetry.understeer,
            oversteer: frame.telemetry.oversteer,
            side_surface: [[0, 3], [1, 2]]
                .map(|side| side.into_iter().map(|i| i32::from(input.wheels[i].skid_surface)).max().unwrap_or(0)),
            skid_type: 0,
        };
        let tyres_owned =
            car.tyres.update(self, &car.sound, &input, tyre_frame, car.mixer.as_ref().map(CarMixer::tyres), frame.dt);
        car.transmission.update(
            self,
            &car.sound,
            transmission::TransmissionFrame {
                forward_speed: frame.telemetry.forward_speed,
                shifting: out.shift_state != blackbox_carsound::ShiftState::None,
            },
            car.mixer.as_ref().map(CarMixer::transmission),
            frame.dt,
        );
        car.ambient.update_with_context(
            self,
            &input,
            &commands,
            car.mixer.as_ref().map_or_else(Default::default, CarMixer::ambient),
            frame.dt,
            frame.telemetry.road.as_ref(),
        );
        let power_levels = car.mixer.as_ref().map(CarMixer::power);
        let power_frame = power_effects::PowerFrame {
            physics_rpm: out.physics_rpm,
            rpm_at_shift: out.rpm_at_shift,
            camera_pov_zero: view.view_value == 0,
            engine_torque: out.eng_torque,
            physics_torque: out.physics_torque,
            shifting: out.shift_state != blackbox_carsound::ShiftState::None,
            max_physics_rpm: frame.telemetry.max_physics_rpm,
            exhaust_rotation: levels.engine_azimuth,
        };
        let routed: Vec<bool> = commands
            .iter()
            .map(|command| match *command {
                SoundCommand::Play { sound, .. } => car.power.play(self, &car.sound, power_frame, power_levels, sound),
                _ => false,
            })
            .collect();
        car.power.update(self, &car.sound, &input, power_frame, power_levels, frame.dt);
        for (command, power_event) in commands.drain(..).zip(routed) {
            routing::dispatch(self, car, command, tyres_owned, power_event);
        }
        if self.trace.enabled() {
            self.trace.note(
                "engine_revision",
                &format!(
                    "car={} requested={} consumed={:?} frequency={}",
                    car.car,
                    out.tick_revision,
                    car.handle.consumed_revision(),
                    car.handle.current_frequency()
                ),
            );
        }
        car.commands = commands;
    }
}
