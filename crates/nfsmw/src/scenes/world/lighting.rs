//! The world's lighting as the scene draws it: the scattering fog, the surface light and the fog colour, from the
//! time-of-day presets and the sun.

use super::{CLEAR, WorldScene};

mod clouds;
pub(super) use clouds::Clouds;

impl WorldScene {
    pub(super) fn car_specular_scale(&self) -> f32 {
        self.time_of_day
            .as_ref()
            .and_then(|presets| {
                nfsmw_data::time_of_day::car_specular_scale(
                    presets,
                    self.atmosphere.intensity(),
                    self.sun.preset_height(),
                )
            })
            .unwrap_or(1.0)
    }

    pub(super) fn update_atmosphere(&mut self, dt: f32) {
        let position = self.focus();
        let rain = self.rain.target();
        self.atmosphere.update(dt, position, rain);
    }

    fn atmosphere_inputs(&self) -> Option<nfsmw_data::time_of_day::Atmosphere> {
        nfsmw_data::time_of_day::atmosphere(
            self.time_of_day.as_ref()?,
            self.atmosphere.intensity(),
            self.sun.preset_height(),
        )
    }

    /// The scattering fog from the lighting: the sky colour (Rayleigh) and the haze colour (Mie), both mixed by the
    /// sun's height and authored cloud intensity. Scalar scales come from the same blended presets.
    /// `None` if the required authored inputs are missing or invalid.
    pub(super) fn scattering(&self) -> Option<blackbox_gfx::Scattering> {
        // The glow centres on the sun the sky draws; without the full cycle that is the scene light.
        let sun = self.sun.visible();
        let inputs = self.atmosphere_inputs()?;
        let s = nfsmw_data::fog::scatter(
            inputs.lighting.fog_sky,
            inputs.lighting.fog_haze,
            inputs.sky_scale,
            inputs.haze_scale,
            inputs.g,
        );
        Some(blackbox_gfx::Scattering {
            rayleigh: s.br,
            mie: s.bm,
            g: s.g,
            to_sun: sun,
            distance_scale: inputs.distance_scale,
            in_scatter_scale: inputs.in_scatter_scale,
        })
    }

    /// Authored world lighting. Normal-mapped materials consume specular colour/power;
    /// each other material family keeps its own response and cars use their separate rig.
    pub(super) fn surface_light(&self) -> Option<blackbox_gfx::SurfaceLight> {
        let inputs = self.atmosphere_inputs()?;
        let diffuse = inputs.lighting.diffuse.map(|channel| channel.min(1.0));
        let (keep, fill) = self.moonlight(diffuse);
        let moon = |colour: [f32; 3]| self.moon_colour(colour).map(|channel| channel * keep);
        let ambient = std::array::from_fn(|i| (inputs.lighting.ambient[i] + fill[i]).min(1.0));
        Some(blackbox_gfx::SurfaceLight {
            shadow_strength: nfsmw_data::time_of_day::shadows::strength(
                self.atmosphere.intensity(),
                self.sun.preset_height(),
            ),
            ambient,
            // The common PC shader updater caps diffuse before the material's Lambert term.
            diffuse: moon(diffuse),
            ambient_alpha: inputs.ambient_alpha,
            specular: moon(inputs.lighting.specular),
            specular_power: inputs.specular_power,
            reflection_scale: self.wet_road.contribution(),
            reflective_specular_scale: 1.0 - self.atmosphere.intensity(),
        })
    }

    /// At night the sun stands in for the moon: the share of its direct light `diffuse` to keep (the moonlight
    /// share, less under clouds) and the ambient light that replaces the rest, so the night stays as bright but its
    /// shadows and highlights are weak.
    fn moonlight(&self, diffuse: [f32; 3]) -> (f32, [f32; 3]) {
        let [night, share, block] = self.moonlight;
        let kept = share * (1.0 - block * self.atmosphere.intensity().clamp(0.0, 1.0));
        let keep = 1.0 + (kept - 1.0) * night.clamp(0.0, 1.0);
        (keep, self.moon_colour(diffuse).map(|channel| channel * (1.0 - keep) * 0.35))
    }

    /// `colour` as moonlight at night: the presets give the night the sunset's warm light, the moon's is a cool white
    /// of the same brightness.
    fn moon_colour(&self, colour: [f32; 3]) -> [f32; 3] {
        const MOON: [f32; 3] = [0.88, 1.0, 1.17];
        let night = self.moonlight[0].clamp(0.0, 1.0);
        let luma = colour[0] * 0.2126 + colour[1] * 0.7152 + colour[2] * 0.0722;
        std::array::from_fn(|i| colour[i] + (luma * MOON[i] - colour[i]) * night)
    }

    /// The ambient light the moonlight conversion adds (`moonlight`), for car rigs whose fills a probe replaces.
    pub(super) fn moon_fill(&self) -> glam::Vec3 {
        let Some(inputs) = self.atmosphere_inputs() else { return glam::Vec3::ZERO };
        glam::Vec3::from_array(self.moonlight(inputs.lighting.diffuse.map(|channel| channel.min(1.0))).1)
    }

    /// The fog colour for the sun's height and the blend weight `t`.
    pub(super) fn fog_colour(&self) -> [f32; 3] {
        self.atmosphere_inputs().map_or(CLEAR, |inputs| inputs.lighting.fog_haze)
    }
}
