//! The siren sound's class words and the voice-slot rules. Spec: `docs/specs/cop-siren-audio.md` sections 3 and 4.

/// The cop sound states the original creates: at most this many sirens sound at once. [decomp]
pub const SLOTS: usize = 4;
/// A cop car gets a sound state within this many metres of the camera. [decomp]
pub const RADIUS: f32 = 135.0;

/// The `SIREN` class's nine words, in the order of its structure: volume, pitch offset, pan, distance, type, low-pass,
/// high-pass, dry and wet level. The clamps are the class setters'. [decomp]
///
/// `volume` is the mixer's output (0 to 32767), `pitch` the mixer's pitch in Q12 (4096 is unchanged), `azimuth` the
/// mixer's 16-bit angle, `metres` the distance to the view and `kind` the voice's type word.
pub fn class_words(volume: i32, pitch: i32, azimuth: u16, metres: f32, kind: i32) -> [i32; 9] {
    let distance = if metres.is_finite() { (metres * 12.8) as i32 } else { 0 };
    [
        volume.clamp(0, 0x7FFF),
        (pitch - 0x1000).clamp(-0x4000, 0x4000),
        i32::from(azimuth),
        distance.clamp(0, 0x400),
        kind.clamp(0, 7),
        25000,
        0,
        0x7FFF,
        0,
    ]
}

/// What a frame did to the voice slots.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Changes {
    /// Slots that took a car this frame (their voices start from scratch).
    pub attached: Vec<usize>,
    /// Slots that lost their car this frame.
    pub detached: Vec<usize>,
}

/// Gives the free slots to the cars in range and takes them from the cars out of range or gone. `cars` lists every
/// police car with its distance to the camera, in the order the sound system would meet them: a car keeps its slot
/// while it stays in range, and a car in range with every slot taken waits. [decomp]
pub fn assign(slots: &mut [Option<u32>; SLOTS], cars: &[(u32, f32)]) -> Changes {
    let mut changes = Changes::default();
    for (index, slot) in slots.iter_mut().enumerate() {
        let Some(id) = *slot else { continue };
        let in_range = cars.iter().any(|&(car, distance)| car == id && distance < RADIUS);
        if in_range {
            continue;
        }
        *slot = None;
        changes.detached.push(index);
    }
    for &(id, distance) in cars {
        if distance >= RADIUS || slots.contains(&Some(id)) {
            continue;
        }
        let Some(index) = slots.iter().position(Option::is_none) else { break };
        slots[index] = Some(id);
        changes.attached.push(index);
    }
    changes
}
