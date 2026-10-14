//! Physics telemetry for authored tyre, transmission and road sound producers.
use super::{CarSim, PHYSICS_WHEEL};
use crate::audio::{CarAudioTelemetry, RoadContext, RoadTerrain};
use nfsmw_data::car::physics::SurfaceTable;

impl CarSim {
    pub fn audio_telemetry(&self, surfaces: &SurfaceTable) -> CarAudioTelemetry {
        let v = &self.vehicle;
        let speed = v.speed();
        let skid = |i| v.wheel(i).lateral_speed;
        let front = (skid(0) + skid(1)) * 0.5;
        let steer = (v.wheel(0).steer_angle + v.wheel(1).steer_angle) * 0.5;
        let enabled = v.forward_speed() > 0.0 && speed > 1.0;
        let understeer = match enabled && steer * front < 0.0 {
            true => (front.abs() / speed).min(1.0),
            false => 0.0,
        };
        let oversteer = match enabled {
            true => ((skid(2) + skid(3)).abs() * 0.5 / speed).min(1.0),
            false => 0.0,
        };
        let terrain = PHYSICS_WHEEL.map(|i| {
            let key = v.wheel_surface_tag(i).unwrap_or(0);
            let audio = surfaces.audio(key);
            RoadTerrain {
                key,
                transition_on: audio.transition_on,
                transition_off: audio.transition_off,
                secondary: audio.secondary,
            }
        });
        CarAudioTelemetry {
            forward_speed: v.forward_speed(),
            understeer,
            oversteer,
            road: Some(RoadContext { terrain, damage: [0; 4] }),
        }
    }
}
