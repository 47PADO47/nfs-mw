//! Appearance from THREE.Fire (MIT); triggers remain the installed game's triggers.
use blackbox_gfx::{FlameAppearance, FlameVolume, PixelFormat, RenderBackend, TextureDesc, TextureHandle};
use glam::{Mat4, Vec3};
mod burst;
mod burst_render;
mod contact;
mod motion;

#[cfg(test)]
mod installed_tests;
#[cfg(test)]
mod tests;

#[derive(Default)]
pub(super) struct Enhanced {
    pub enabled: bool,
    pub texture: Option<TextureHandle>,
    pub appearance: crate::garage::ExhaustSelection,
    motion: motion::Motion,
    visual_motion: Option<motion::Motion>,
    visual_position: Option<Vec3>,
    visual_time: Option<f32>,
    visual_strength: Option<f32>,
    velocity: Vec3,
    preview: Option<std::time::Instant>,
    time: f32,
    nitrous: f32,
    pub backfire_style: crate::settings::BackfireStyle,
    /// The car is a bike: its bursts are drawn at `BIKE_FLAME_SCALE` of their size.
    pub bike: bool,
    bursts: burst::Bursts,
    visual_bursts: Option<burst::Bursts>,
    burst_preview: Option<(std::time::Instant, burst::Bursts)>,
    pose: Mat4,
    ignition: u32,
    was_burning: bool,
    seed: f32,
}

impl Enhanced {
    /// Presentation runs once per rendered frame, using the world's dilated animation delta.
    /// Physics owns ignition/state; it no longer quantizes visible flow during Speedbreaker.
    pub fn advance_visual(&mut self, dt: f32, pose: Mat4) {
        if dt <= 0.0 {
            return;
        }
        match &mut self.visual_bursts {
            Some(bursts) => bursts.advance(dt),
            None => self.visual_bursts = Some(self.bursts.clone()),
        }
        let position = pose.transform_point3(Vec3::ZERO);
        let velocity = self.visual_position.map_or(self.velocity, |previous| {
            let delta = position - previous;
            if delta.length() > 10.0 + self.velocity.length() * dt * 4.0 {
                return self.velocity;
            }
            delta / dt
        });
        self.visual_position = Some(position);
        let motion = self.visual_motion.get_or_insert_with(Default::default);
        motion.step(dt, pose, velocity);
        let rate = match self.was_burning {
            true => (2.0 + (motion.speed / 45.0).min(1.5)) * f32::from(self.appearance.authored().flow) / 100.0,
            false => 1.0,
        };
        match &mut self.visual_time {
            Some(time) => *time += dt * rate,
            None => self.visual_time = Some(self.time),
        }
        let target = f32::from(self.was_burning);
        let response = match self.was_burning {
            true => 45.0,
            false => 25.0,
        };
        match &mut self.visual_strength {
            Some(strength) => *strength += (target - *strength) * (1.0 - (-response * dt).exp()),
            None => self.visual_strength = Some(self.nitrous),
        }
    }

    pub fn set_preview(&mut self, on: bool) {
        if !on {
            self.preview = None;
            return;
        }
        self.preview.get_or_insert_with(std::time::Instant::now);
    }
    pub fn set_pose(&mut self, pose: Mat4) {
        self.pose = pose;
    }

    pub fn load(&mut self, renderer: &mut dyn RenderBackend) {
        if !self.enabled || self.texture.is_some() {
            return;
        }
        let bytes = include_bytes!("../../../../../../assets/effects/three-fire.png");
        let result = (|| -> anyhow::Result<_> {
            let mut reader = png::Decoder::new(std::io::Cursor::new(bytes)).read_info()?;
            let mut rgb = vec![0; reader.output_buffer_size().unwrap_or(0)];
            let info = reader.next_frame(&mut rgb)?;
            let rgba: Vec<_> =
                rgb[..info.buffer_size()].as_chunks::<3>().0.iter().flat_map(|p| [p[0], p[1], p[2], 255]).collect();
            Ok(renderer.create_texture(&TextureDesc {
                label: "THREE.Fire radial-height lookup (MIT)",
                width: info.width,
                height: info.height,
                format: PixelFormat::Rgba8,
                mips: vec![&rgba],
            }))
        })();
        match result {
            Ok(texture) => self.texture = Some(texture),
            Err(error) => log::warn!("enhanced exhaust lookup: {error:#}"),
        }
    }

    pub fn step(&mut self, dt: f32, car: &super::CarState, event: Option<crate::garage::BackfireEvent>) {
        self.step_sized(dt, car, event.map(|event| (event, 1.0)));
    }

    /// [`Self::step`] with the size of the burst the event starts, as a factor on its own (a bike's pop is its shift burst
    /// at `trigger::BIKE_BACKFIRE_SCALE_OVER_SHIFT`).
    pub fn step_sized(&mut self, dt: f32, car: &super::CarState, event: Option<(crate::garage::BackfireEvent, f32)>) {
        self.bursts.advance(dt);
        match car.nitrous {
            true => {
                self.bursts.clear();
                if let Some(bursts) = &mut self.visual_bursts {
                    bursts.clear();
                }
            }
            false => {
                if let Some((event, size)) = event {
                    self.bursts.trigger_sized(event, &self.backfire_style, size);
                    if let Some(bursts) = &mut self.visual_bursts {
                        bursts.trigger_sized(event, &self.backfire_style, size);
                    }
                }
            }
        }
        self.velocity = car.velocity;
        self.motion.step(dt, car.to_world, car.velocity);
        if car.nitrous && !self.was_burning {
            self.ignition = self.ignition.wrapping_add(1);
            let mut hash = self.ignition.wrapping_mul(0x9e37_79b9);
            hash = (hash ^ (hash >> 16)).wrapping_mul(0x85eb_ca6b);
            self.seed = (hash >> 8) as f32 * (19.19 / 16_777_216.0);
        }
        self.was_burning = car.nitrous;
        let flow_rate = match car.nitrous {
            true => 2.0 + (car.velocity.length() / 45.0).min(1.5),
            false => 1.0,
        };
        let flow = match car.nitrous {
            true => f32::from(self.appearance.authored().flow) / 100.0,
            false => 1.0,
        };
        self.time += dt * flow_rate * flow;
        self.pose = car.to_world;
        let target = f32::from(car.nitrous);
        let response = match car.nitrous {
            true => 45.0,
            false => 25.0,
        };
        self.nitrous += (target - self.nitrous) * (1.0 - (-response * dt).exp());
    }

    pub fn age(&mut self, dt: f32) {
        if dt > 0.0 {
            self.was_burning = false;
        }
        self.time += dt;
        self.nitrous *= (-25.0 * dt).exp();
        self.bursts.advance(dt);
    }

    pub fn disconnect(&mut self) {
        self.visual_motion = None;
        self.visual_position = None;
        self.visual_time = None;
        self.visual_strength = None;
        self.preview = None;
        self.motion = motion::Motion::default();
        self.was_burning = false;
        self.nitrous = 0.0;
        self.bursts.clear();
        self.visual_bursts = None;
        self.burst_preview = None;
    }

    pub fn preview_backfire(&mut self, event: crate::garage::BackfireEvent) {
        let mut bursts = self.burst_preview.take().map(|(_, bursts)| bursts).unwrap_or_default();
        bursts.clear();
        bursts.trigger(event, &self.backfire_style);
        self.burst_preview = Some((std::time::Instant::now(), bursts));
    }

    pub fn clear_backfire_preview(&mut self) {
        self.burst_preview = None;
    }

    pub fn geometry(&self, pipes: &[Mat4], out: &mut Vec<FlameVolume>) -> bool {
        let Some(texture) = self.texture.filter(|_| self.enabled) else { return false };
        let nitrous = self.visual_strength.unwrap_or(self.nitrous);
        let motion = self.visual_motion.as_ref().unwrap_or(&self.motion);
        let speed = match self.preview.is_some() {
            true => 0.0,
            false => motion.speed,
        };
        if self.burst_preview.is_some() && self.preview.is_none() {
            return self.burst_geometry(pipes, texture, motion, out);
        }
        let blue = self.preview.is_some() || nitrous > 0.01;
        if !blue {
            return self.burst_geometry(pipes, texture, motion, out);
        }
        if !self.was_burning && self.preview.is_none() {
            self.burst_geometry(pipes, texture, motion, out);
        }
        let strength = match (self.preview.is_some(), blue) {
            (true, _) => 1.0,
            (false, true) => nitrous,
            _ => 0.0,
        };
        let time = self.preview.map_or(self.visual_time.unwrap_or(self.time) * 3.0, |started| {
            started.elapsed().as_secs_f32() * 3.0 * 2.0 * f32::from(self.appearance.authored().flow) / 100.0
        });
        if strength <= 0.01 {
            return true;
        }
        let authored = self.appearance.authored();
        if blue && (authored.length == 0 || authored.width == 0 || authored.brightness == 0 || authored.density == 0) {
            return true;
        }
        for (index, pipe) in pipes.iter().enumerate() {
            let settings = &authored;
            let speed_fraction = (speed / 70.0).clamp(0.0, 1.0);
            let speed_stretch = 1.0 + 0.2 * speed_fraction * speed_fraction * (3.0 - 2.0 * speed_fraction);
            let response_scale = f32::from(settings.response) / 100.0;
            let pressure = (1.0 + (motion.stretch.max(0.92) - 1.0) * response_scale).clamp(0.85, 1.15);
            let length = match blue {
                true => 1.15 * f32::from(settings.length) / 100.0 * speed_stretch * pressure,
                false => 0.32,
            };
            let plume_length = length * (0.35 + strength * 0.65);
            // The installed marker emits along local +Z; THREE.Fire's flow axis is +Y.
            let axis = Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2);
            // Leave room around the visible flame for the shader's bending tip.
            let width = match blue {
                true => 0.32 * f32::from(settings.width) / 100.0,
                false => 0.26,
            };
            let jet = (self.pose * *pipe).transform_vector3(Vec3::Z).normalize_or_zero();
            let upward = match blue {
                true => {
                    Vec3::Z * plume_length * (0.17 * f32::from(settings.curl) / 100.0).min(0.32) / (1.0 + speed / 100.0)
                }
                false => Vec3::ZERO,
            };
            // Airflow is relative to each pipe, so side exits sweep rearward rather
            // than being rotated into a rear-exhaust shape at the nozzle.
            let airflow = match blue && self.preview.is_none() {
                true => {
                    let wind = -motion.velocity.normalize_or_zero();
                    let crosswind = wind - jet * wind.dot(jet);
                    crosswind * plume_length * 0.14 * response_scale * (speed_stretch - 1.0) / 0.2
                }
                false => Vec3::ZERO,
            };
            let response = match blue && self.preview.is_none() {
                true => {
                    // Older gas retains the previous jet direction. Taking the cross
                    // product per pipe also gives side exits the correct lag direction.
                    let sharpness = ((motion.angular_velocity.length() - 0.35) / 1.65).clamp(0.0, 1.0);
                    let lag = 0.10 + 0.08 * sharpness;
                    let turn = -motion.angular_velocity.cross(jet) * lag;
                    ((turn + motion.bend / 0.8) * response_scale * plume_length).clamp_length_max(plume_length * 0.55)
                }
                false => Vec3::ZERO,
            };
            // One force balance and one bend profile for heat, airflow and motion.
            // Remove axial displacement so the nozzle pressure owns flame length.
            let force = upward + airflow + response;
            let bend = (force - jet * force.dot(jet)).clamp_length_max(plume_length * 0.65);
            let proxy_width = width
                * match blue {
                    true => 1.5,
                    false => 1.0,
                }
                + 2.0 * bend.length();
            let radial_scale = proxy_width / width;
            let scale = Vec3::new(proxy_width, plume_length, proxy_width);
            let to_world = self.pose * *pipe * axis * Mat4::from_scale(scale) * Mat4::from_translation(Vec3::Y * 0.5);
            let mut appearance = FlameAppearance::default();
            if blue {
                let tint = |rgb: [u8; 3], base: [f32; 3]| {
                    Vec3::from_array(std::array::from_fn(|i| f32::from(rgb[i]) / base[i]))
                };
                appearance.root_tint = tint(settings.root, [80.0, 160.0, 255.0]);
                appearance.tail_tint = tint(settings.tail, [255.0, 180.0, 80.0]);
                let palette = |rgb: [u8; 3], baseline: [u8; 3]| {
                    let color = Vec3::from_array(rgb.map(|v| f32::from(v) / 255.0));
                    let reference = Vec3::from_array(baseline.map(|v| f32::from(v) / 255.0));
                    color.extend((color.distance(reference) * 1.8).min(1.0)).to_array()
                };
                appearance.root_color = palette(settings.root, [80, 160, 255]);
                appearance.tail_color = palette(settings.tail, [255, 180, 80]);
                appearance.density = f32::from(settings.density) / 100.0;
                appearance.radial_scale = radial_scale;
                appearance.turbulence = f32::from(settings.turbulence) / 100.0;
                appearance.bend = to_world.inverse().transform_vector3(bend);
            }
            out.push(FlameVolume {
                texture,
                to_world,
                time,
                seed: self.seed + index as f32 * 7.17,
                blue,
                intensity: strength
                    * match blue {
                        true => 0.65 * f32::from(settings.brightness) / 100.0,
                        false => 0.22,
                    },
                appearance,
            });
        }
        true
    }

    pub fn contact(
        &self,
        world: &blackbox_collision::CollisionWorld,
        layer: &mut blackbox_gfx::EffectLayer,
        start: usize,
    ) {
        for volume in &mut layer.flames[start..] {
            let (root, tail) = match volume.blue {
                true => (self.appearance.root, self.appearance.tail),
                false => (self.backfire_style.root, self.backfire_style.tail),
            };
            contact::apply(world, volume, &mut layer.glows, root, tail);
        }
    }
}
