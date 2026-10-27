//! The hinged body parts of a car: bonnet, bumpers, mirrors, doors. Spec: `docs/formats/chameleon-hinged-parts.md`.
//!
//! The `VehicleDamageBehaviour` object of a car bundle lists a `Bodypart` per part that can come loose. Each names the
//! skeleton bone its mesh is skinned to (`BonnetHinge`), gives the part's centre and mass, the angle range it may swing
//! through on each axis, the damage zone that drives it and the ladder of damage fractions at which it changes state.
//! Field names are hashed, so fields are read by offset; what each offset means is tagged in the doc.

use chameleon_bundle::Bundle;

use crate::error::{Error, Result};
use crate::genesys::{Object, TypeNames, find_object};
use crate::skeleton::Skeleton;

const BEHAVIOUR: &str = "Genesys.Gen.VehicleDamageBehaviour";
const BODYPART: &str = "Genesys.Gen.VehicleDamageBehaviour.Bodypart";
/// Longest part name read, in bytes.
const MAX_NAME: usize = 64;
/// Bytes of one `DamageThreshold` and the offset of its fraction.
const THRESHOLD_SIZE: usize = 24;
const THRESHOLD_VALUE: usize = 0x10;
const THRESHOLD_TARGET: usize = 0x14;

/// One hinged part.
#[derive(Debug, Clone, PartialEq)]
pub struct HingedPart {
    /// The part's name, which is also the name of its skeleton bone (`BonnetHinge`).
    pub name: String,
    /// The index of that bone in the car's skeleton.
    pub bone: usize,
    /// The damage zone that drives the part: 1 front, 2 rear, 3 left, 4 right **[unconfirmed for other values]**.
    pub zone: u16,
    /// The part's centre in the car's frame, metres (x left, y up, z forward).
    pub centre: [f32; 3],
    /// The range the part may swing through about the car's x, y and z axes, `[min, max]` each, in degrees
    /// **[unconfirmed unit]**.
    pub limits: [[f32; 2]; 3],
    pub mass_kg: f32,
    /// The zone damage fractions (0 to 1) at which the part changes state, in the order the file gives them.
    pub thresholds: Vec<f32>,
    /// The fraction of the zone's damage clip each step plays to (0 to 1), one per threshold **[unconfirmed meaning]**.
    pub targets: Vec<f32>,
}

/// The hinged parts of a car bundle whose bone the car's skeleton has. A car with no damage behaviour, or no skeleton,
/// has none.
pub fn hinged_parts(bundle: &Bundle) -> Result<Vec<HingedPart>> {
    let types = TypeNames::read(bundle)?;
    let Ok(object) = find_object(bundle, &types, BEHAVIOUR) else { return Ok(Vec::new()) };
    let Some(skeleton) = Skeleton::read(bundle) else { return Ok(Vec::new()) };
    let mut parts = Vec::new();
    for at in object.all(BODYPART) {
        let name = name(&object, at)?;
        let Some(bone) = skeleton.find(&name) else { continue };
        parts.push(HingedPart {
            bone,
            zone: object.u16(at + 0xB0)?,
            centre: object.floats::<3>(at + 0x40)?,
            limits: [object.floats::<2>(at + 0x10)?, object.floats::<2>(at + 0x20)?, object.floats::<2>(at + 0x30)?],
            mass_kg: object.f32(at + 0xA8)?,
            thresholds: thresholds(&object, at, THRESHOLD_VALUE)?,
            targets: thresholds(&object, at, THRESHOLD_TARGET)?,
            name,
        });
    }
    Ok(parts)
}

/// The name behind the string reference at `+0x90`: an offset from the field itself and a length.
fn name(object: &Object, at: usize) -> Result<String> {
    let field = at + 0x90;
    let start = field + object.u32(field)? as usize;
    let bytes = object.main.get(start..).ok_or_else(|| Error::Spec(format!("a part name starts past 0x{start:X}")))?;
    let end = bytes.iter().take(MAX_NAME).position(|&byte| byte == 0).unwrap_or(0);
    String::from_utf8(bytes[..end].to_vec()).map_err(|_| Error::Spec("a part name is not text".into()))
}

/// One float of each step: an absolute offset at `+0xAC` and a count at `+0xB2`; `field` is the float's offset in a step.
fn thresholds(object: &Object, at: usize, field: usize) -> Result<Vec<f32>> {
    let table = object.u32(at + 0xAC)? as usize;
    let count = usize::from(object.u16(at + 0xB2)?);
    (0..count).map(|n| object.f32(table + n * THRESHOLD_SIZE + field)).collect()
}
