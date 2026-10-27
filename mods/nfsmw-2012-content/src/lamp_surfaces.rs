//! The lit and unlit looks of the lamp surfaces of a converted car (`docs/modding/car-lamps.md`, "Lamp surfaces").
//!
//! The 2012 car lights a lamp from one of four circuits (`chameleon_vehicle::LampCircuit`). The game's own cars light a
//! lamp surface through a placeholder texture that it swaps for the car's own texture of the lamp's state, so every
//! circuit has a placeholder for each side of the car and two flat textures, lit and unlit, named for the car.

use chameleon_vehicle::{LampCircuit, LampSide, LampSurface};

/// What each circuit looks like: the kind of lamp the game's placeholders and state textures are named for (the textures
/// are `<FOLDER>_KIT00_<KIND>_ON` and `_OFF`), and the unlit and lit colours, sRGB bytes.
pub const LOOKS: [(LampCircuit, &str, [u8; 3], [u8; 3]); 4] = [
    // Warm white lamps; dark grey-blue glass when off.
    (LampCircuit::Head, "HEADLIGHT", [38, 42, 48], [255, 244, 214]),
    // Bright red brake lamps over dark red.
    (LampCircuit::Brake, "BRAKELIGHT", [56, 8, 8], [255, 24, 16]),
    // The tail lamps burn dimmer than the brake lamps: the same dark red off, a deep red lit.
    (LampCircuit::Tail, "TAILLIGHT", [56, 8, 8], [176, 14, 10]),
    // White reverse lamps, a clear dark grey when off.
    (LampCircuit::Reverse, "REVERSELIGHT", [60, 64, 68], [250, 252, 255]),
];

/// The kind of lamp `circuit` lights: `HEADLIGHT`, `BRAKELIGHT`, `TAILLIGHT` or `REVERSELIGHT`.
fn kind(circuit: LampCircuit) -> &'static str {
    LOOKS.iter().find(|look| look.0 == circuit).map_or("BRAKELIGHT", |look| look.1)
}

/// The placeholder texture a lamp surface is drawn with: the game swaps it for the car's lit or unlit texture. The head
/// lamps have a left and a right placeholder only (a piece on the centre plane follows the left lamp); the others have a
/// centre one too.
pub fn placeholder(surface: LampSurface) -> String {
    let side = match (surface.circuit, surface.side) {
        (LampCircuit::Head, LampSide::Centre) | (_, LampSide::Left) => "LEFT",
        (_, LampSide::Right) => "RIGHT",
        (_, LampSide::Centre) => "CENTRE",
    };
    format!("{}_{side}", kind(surface.circuit))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chameleon_vehicle::LampEnd;

    fn surface(circuit: LampCircuit, side: LampSide) -> LampSurface {
        LampSurface { end: circuit.end(), side, circuit }
    }

    #[test]
    fn each_circuit_has_its_own_placeholders() {
        assert_eq!(placeholder(surface(LampCircuit::Head, LampSide::Left)), "HEADLIGHT_LEFT");
        assert_eq!(placeholder(surface(LampCircuit::Head, LampSide::Right)), "HEADLIGHT_RIGHT");
        assert_eq!(placeholder(surface(LampCircuit::Brake, LampSide::Centre)), "BRAKELIGHT_CENTRE");
        assert_eq!(placeholder(surface(LampCircuit::Tail, LampSide::Right)), "TAILLIGHT_RIGHT");
        assert_eq!(placeholder(surface(LampCircuit::Reverse, LampSide::Centre)), "REVERSELIGHT_CENTRE");
        assert_eq!(placeholder(surface(LampCircuit::Reverse, LampSide::Left)), "REVERSELIGHT_LEFT");
    }

    #[test]
    fn a_head_piece_on_the_centre_plane_follows_the_left_lamp() {
        let centre = LampSurface { end: LampEnd::Front, side: LampSide::Centre, circuit: LampCircuit::Head };
        assert_eq!(placeholder(centre), "HEADLIGHT_LEFT");
    }

    #[test]
    fn every_circuit_has_a_look_and_a_lit_colour_brighter_than_its_unlit_one() {
        for circuit in [LampCircuit::Head, LampCircuit::Brake, LampCircuit::Tail, LampCircuit::Reverse] {
            let look = LOOKS.iter().find(|look| look.0 == circuit).unwrap();
            assert!(
                look.3.iter().map(|&c| u32::from(c)).sum::<u32>() > look.2.iter().map(|&c| u32::from(c)).sum::<u32>()
            );
        }
    }
}
