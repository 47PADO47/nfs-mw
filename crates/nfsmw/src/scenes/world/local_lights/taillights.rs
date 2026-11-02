//! Short, unshadowed player tail-light spill. This is an artistic enhancement.
use super::super::car_lights::{CarLights, Kind, LampState};
use blackbox_gfx::LocalLight;
use glam::{Mat4, Vec3};

/// Append the rear spill of the lit tail lamps to `out`.
pub(super) fn emit(lamps: &CarLights, transform: Mat4, state: LampState, out: &mut Vec<LocalLight>) {
    let strength = match (state.dark, state.headlights, state.brake > 0.0) {
        (true, _, _) | (_, false, false) => 0.0,
        (_, _, true) => 0.65,
        _ => 0.15,
    };
    if strength <= 0.0 {
        return;
    }
    let lit =
        lamps.lamps().iter().filter(|l| l.kind == Kind::Tail && l.flags & state.broken == 0).take(2).map(|lamp| {
            let rear = transform.transform_vector3(lamp.direction).normalize_or_zero();
            let up = transform.transform_vector3(Vec3::Z).normalize_or_zero();
            LocalLight {
                position: (transform.transform_point3(lamp.at) + rear * 0.15).to_array(),
                direction: (rear - up * 0.35).normalize_or_zero().to_array(),
                casts_shadow: false,
                shadow_exclude: None,
                upper_plane: None,
                range: 5.0,
                outer_cos: 0.15,
                inner_cos: 0.72,
                color: (Vec3::new(1.0, 0.025, 0.012) * strength).to_array(),
            }
        });
    out.extend(lit);
}
#[cfg(test)]
mod tests {
    use super::super::super::car_lights::Lamp;
    use super::*;
    fn emit(lamps: &CarLights, transform: Mat4, state: LampState) -> Vec<LocalLight> {
        let mut out = Vec::new();
        super::emit(lamps, transform, state, &mut out);
        out
    }
    #[test]
    fn running_braking_damage_and_engine_control_rear_spill() {
        let lamps = CarLights::new(vec![
            Lamp { at: Vec3::ZERO, direction: Vec3::NEG_X, kind: Kind::Tail, flags: 8 },
            Lamp { at: Vec3::Y, direction: Vec3::NEG_X, kind: Kind::Tail, flags: 16 },
        ]);
        assert!(emit(&lamps, Mat4::IDENTITY, LampState::default()).is_empty());
        let running = LampState { headlights: true, ..Default::default() };
        let on = emit(&lamps, Mat4::IDENTITY, running);
        assert_eq!(on.len(), 2);
        assert!(on.iter().all(|l| !l.casts_shadow && l.direction[2] < 0.0));
        let braking = emit(&lamps, Mat4::IDENTITY, LampState { brake: 1.0, ..running });
        assert!(braking[0].color[0] > on[0].color[0]);
        assert_eq!(emit(&lamps, Mat4::IDENTITY, LampState { broken: 8, ..running }).len(), 1);
        assert!(emit(&lamps, Mat4::IDENTITY, LampState { dark: true, brake: 1.0, ..running }).is_empty());
    }
}
