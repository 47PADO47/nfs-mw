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
                nfsmw_data::time_of_day::car_specular_scale(presets, self.atmosphere.intensity(), self.sun.to_sun().z)
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
            self.sun.to_sun().z,
        )
    }

    /// The scattering fog from the lighting: the sky colour (Rayleigh) and the haze colour (Mie), both mixed by the
    /// sun's height and authored cloud intensity. Scalar scales come from the same blended presets.
    /// `None` if the required authored inputs are missing or invalid.
    pub(super) fn scattering(&self) -> Option<blackbox_gfx::Scattering> {
        let sun = self.sun.to_sun();
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
        Some(blackbox_gfx::SurfaceLight {
            shadow_strength: nfsmw_data::time_of_day::shadows::strength(
                self.atmosphere.intensity(),
                self.sun.to_sun().z,
            ),
            ambient: inputs.lighting.ambient.map(|channel| channel.min(1.0)),
            // The common PC shader updater caps diffuse before the material's Lambert term.
            diffuse: inputs.lighting.diffuse.map(|channel| channel.min(1.0)),
            ambient_alpha: inputs.ambient_alpha,
            specular: inputs.lighting.specular,
            specular_power: inputs.specular_power,
            reflection_scale: self.wet_road.contribution(),
            reflective_specular_scale: 1.0 - self.atmosphere.intensity(),
        })
    }

    /// The fog colour for the sun's height and the blend weight `t`.
    pub(super) fn fog_colour(&self) -> [f32; 3] {
        self.atmosphere_inputs().map_or(CLEAR, |inputs| inputs.lighting.fog_haze)
    }
}
