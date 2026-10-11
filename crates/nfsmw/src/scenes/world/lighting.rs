//! The world's lighting as the scene draws it: the scattering fog, the surface light and the fog colour, from the
//! time-of-day presets and the sun.

use super::{CLEAR, WorldScene};

/// The preset blend weight: the world's eased location value (`+0x290`), 0.0685 at the checked spot. Porting
/// that eased value is not yet done, so this is the observed value held constant.
const LOCATION_BLEND: f32 = 0.0685;
/// The scattering scalars at the values seen in the game (`+0x90`, `+0x94`, `+0x9C`); not yet matched by name.
const SCATTER_SKY_FIELD: f32 = 3.0;
const SCATTER_FOG_FIELD: f32 = 2.27;
const SCATTER_G: f32 = -0.47;

impl WorldScene {
    /// The scattering fog from the lighting: the sky colour (Rayleigh) and the haze colour (Mie), both mixed by the
    /// sun's height and the blend weight. The three scalars (`+0x90`, `+0x94`, `+0x9C`) are held at the values seen
    /// live, because their attribute names are not matched yet. `None` if the presets are missing.
    pub(super) fn scattering(&self) -> Option<blackbox_gfx::Scattering> {
        let presets = self.time_of_day.as_ref()?;
        let sun = self.sun.to_sun();
        let lighting = nfsmw_data::time_of_day::lighting(presets, LOCATION_BLEND, sun.z)?;
        let s = nfsmw_data::fog::scatter(
            lighting.fog_sky,
            lighting.fog_haze,
            SCATTER_SKY_FIELD,
            SCATTER_FOG_FIELD,
            SCATTER_G,
        );
        Some(blackbox_gfx::Scattering { rayleigh: s.br, mie: s.bm, g: s.g, to_sun: sun })
    }

    /// The ambient and diffuse colours of the world surfaces from the lighting (`+0x30` and `+0x20` of the lighting
    /// object). The specular colour is not used: the world surfaces have no specular term; the game's specular
    /// applies to the car's glossy parts through their own rig.
    pub(super) fn surface_light(&self) -> Option<blackbox_gfx::SurfaceLight> {
        let presets = self.time_of_day.as_ref()?;
        let sun = self.sun.to_sun();
        let lighting = nfsmw_data::time_of_day::lighting(presets, LOCATION_BLEND, sun.z)?;
        Some(blackbox_gfx::SurfaceLight { ambient: lighting.ambient, diffuse: lighting.diffuse })
    }

    /// The fog colour for the sun's height and the blend weight `t`.
    pub(super) fn fog_colour(&self) -> [f32; 3] {
        let Some(presets) = &self.time_of_day else {
            return CLEAR;
        };
        nfsmw_data::time_of_day::fog_colour(presets, LOCATION_BLEND, self.sun.to_sun().z).unwrap_or(CLEAR)
    }
}
