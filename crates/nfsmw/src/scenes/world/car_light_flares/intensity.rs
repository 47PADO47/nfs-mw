//! One ordered native marker traversal, independent of profile layering or draw cadence.
use super::emergency;
use crate::scenes::world::car_lights::{CarLights, Kind, Lamp, LampState};

pub(super) fn samples(lamps: &CarLights, state: LampState) -> impl Iterator<Item = (&Lamp, f32)> + Clone {
    let live = state.emergency & !state.broken;
    let colors = [emergency::RED, emergency::BLUE].map(|mask| f32::from(u8::from(live & mask != 0)));
    let pulse = emergency::colour_pulse(state.time);
    let head = emergency::headlight_wigwag(state.time, state.emergency);
    lamps.lamps().iter().filter(|lamp| !matches!(lamp.kind, Kind::Signal(_))).enumerate().scan(
        colors,
        move |colors, (ordinal, lamp)| {
            let intensity = match lamp.kind {
                Kind::Head => {
                    let gain = match lamp.flags {
                        1 => head[0],
                        2 => head[1],
                        _ => 1.0,
                    };
                    f32::from(u8::from(state.headlights)) * gain
                }
                Kind::Tail => match state.brake > 0.0 {
                    true => 1.0,
                    false => 0.5,
                },
                Kind::CentreBrake => f32::from(u8::from(state.brake > 0.0)),
                Kind::Reverse => f32::from(u8::from(state.reverse)),
                Kind::Siren(red) => {
                    let index = usize::from(!red);
                    colors[index] *= pulse[index];
                    colors[index]
                }
                Kind::SirenBright(red) => {
                    let index = usize::from(!red);
                    colors[index] * pulse[index]
                }
                Kind::SirenWhite => emergency::white_intensity(state.time, ordinal, state.emergency, state.broken),
                Kind::SirenOrange => 1.0,
                Kind::Signal(_) => 0.0,
            };
            let intensity = match lamp.flags & state.broken != 0 || state.dark {
                true => 0.0,
                false => intensity,
            };
            Some((lamp, intensity))
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lights() -> CarLights {
        let lamps = [Kind::Head, Kind::Tail, Kind::CentreBrake, Kind::Reverse].map(|kind| Lamp {
            direction: glam::Vec3::X,
            at: glam::Vec3::ZERO,
            kind,
            flags: 0,
        });
        CarLights::new(lamps.to_vec())
    }

    #[test]
    fn a_dark_car_shines_no_lamp_not_even_the_tail_glow() {
        let lit = LampState { headlights: true, reverse: true, ..Default::default() };
        let levels = |state| samples(&lights(), state).map(|(_, level)| level).collect::<Vec<_>>();
        assert_eq!(levels(lit), [1.0, 0.5, 0.0, 1.0], "head, the tail's resting glow, no brake, reverse");
        assert!(levels(LampState { dark: true, brake: 1.0, ..lit }).iter().all(|level| *level == 0.0));
    }
}
