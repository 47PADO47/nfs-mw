//! The light the menu platform's scenery is drawn with. The original routes platform groups through the ordinary world
//! shaders, which read the shared time-of-day lighting object; in the front-end game state its sun stays at or above the
//! state's phase limit. Spec: `docs/specs/menu-backdrop-light.md`.

use blackbox_gfx::SurfaceLight;
use game_install::GameDir;
use glam::Vec3;
use nfsmw_data::day_night::DayNightConfig;
use nfsmw_data::time_of_day::{atmosphere, shadows, sun_direction, sun_phase_limit};

/// The game state value the front end runs in (`docs/specs/menu-backdrop-light.md`).
const FRONT_END_STATE: u32 = 3;
/// The lighting object's second orbit offset (`+0x10`) at its constructor default.
const ANGLE_B: f32 = 0.5;
/// No weather runs behind the menus: the clear-sky presets.
const CLEAR_SKY: f32 = 0.0;

/// The scenery light of the menu stages. The original turns only the platform's solids, so the light stays in the world frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BackdropLight {
    /// The direction the light travels.
    pub light_dir: Vec3,
    pub surface: SurfaceLight,
}

impl BackdropLight {
    /// The light, or `None` when the install's time-of-day presets are unusable: the scenery then keeps the
    /// renderer's fixed terms rather than invented colours.
    pub fn read(dir: &GameDir) -> Option<Self> {
        let db = nfsmw_data::shared::attributes(dir).ok()?;
        let presets = nfsmw_data::time_of_day::load(&db)?;
        Self::from_presets(&presets)
    }

    fn from_presets(presets: &nfsmw_data::time_of_day::Presets) -> Option<Self> {
        let lo = sun_phase_limit(FRONT_END_STATE);
        let to_sun = Vec3::from_array(sun_direction(lo, lo, DayNightConfig::default().sun_azimuth, ANGLE_B));
        let inputs = atmosphere(presets, CLEAR_SKY, to_sun.z)?;
        let surface = SurfaceLight {
            shadow_strength: shadows::strength(CLEAR_SKY, to_sun.z),
            ambient: inputs.lighting.ambient.map(|channel| channel.min(1.0)),
            // The common PC shader updater caps diffuse before the material's Lambert term.
            diffuse: inputs.lighting.diffuse.map(|channel| channel.min(1.0)),
            ambient_alpha: inputs.ambient_alpha,
            specular: inputs.lighting.specular,
            specular_power: inputs.specular_power,
            reflection_scale: 0.0,
            reflective_specular_scale: 1.0,
        };
        Some(Self { light_dir: -to_sun, surface })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r3_the_front_end_sun_sits_at_its_state_phase_limit() {
        let lo = sun_phase_limit(FRONT_END_STATE);
        assert_eq!(lo, 0.35);
        let to_sun = Vec3::from_array(sun_direction(lo, lo, DayNightConfig::default().sun_azimuth, ANGLE_B));
        assert!(to_sun.z > 0.3, "the front-end sun is well above the horizon: {to_sun}");
    }

    #[test]
    #[ignore = "requires installed attributes; set NFSMW_GAME_DIR"]
    fn r3_installed_presets_give_a_lit_front_end() {
        let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
        let light = BackdropLight::read(&dir).expect("usable presets");
        assert!(light.surface.diffuse.iter().any(|c| *c > 0.1), "{light:?}");
        assert_eq!(light.surface.ambient_alpha, 4.0);
        assert!(light.light_dir.z < 0.0, "light travels down");
    }
}
