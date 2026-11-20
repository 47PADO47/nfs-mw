//! The zone list of the world (`HAWAII/PVS.BNDL`, a `ZoneList`, `docs/formats/chameleon-world.md#units`): which streaming unit
//! covers which rectangle of ground.

use anyhow::{Result, bail, ensure};

use crate::space;

/// Resource type id of a `ZoneList`.
pub const ZONE_LIST: u32 = 0x90;
/// Where the entry count is, and where the entries start and how long one is.
const COUNT_AT: usize = 0x10;
const FIRST_ENTRY: usize = 0x20;
const ENTRY_LEN: usize = 0x50;
/// An entry's rectangle: minimum and maximum corner as four floats each (x, unbounded height, z, 0).
const RECT_MIN: usize = 0x10;
const RECT_MAX: usize = 0x20;
/// An entry's unit number and district id.
const UNIT_AT: usize = 0x34;
const DISTRICT_AT: usize = 0x38;

/// One streaming unit's zone: its number, its district and its rectangle of ground, in engine space (x left, z forward).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Zone {
    pub unit: u32,
    pub district: u32,
    /// Minimum and maximum of x and z, metres.
    pub min: [f32; 2],
    pub max: [f32; 2],
}

impl Zone {
    /// The rectangle's centre in the game's render space (x forward, y left): `(z, x)`.
    pub fn centre_render(&self) -> [f32; 2] {
        let centre = [(self.min[0] + self.max[0]) / 2.0, 0.0, (self.min[1] + self.max[1]) / 2.0];
        let [x, y, _] = space::render(centre);
        [x, y]
    }

    /// Half the length of the rectangle's diagonal: the radius of the circle that holds it.
    pub fn radius(&self) -> f32 {
        let (width, depth) = (self.max[0] - self.min[0], self.max[1] - self.min[1]);
        (width * width + depth * depth).sqrt() / 2.0
    }

    /// Whether the engine-space point (`x`, `z`) is within the rectangle grown by `margin` metres on every side.
    pub fn contains(&self, x: f32, z: f32, margin: f32) -> bool {
        x >= self.min[0] - margin && x <= self.max[0] + margin && z >= self.min[1] - margin && z <= self.max[1] + margin
    }
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn f32_at(bytes: &[u8], at: usize) -> Option<f32> {
    Some(f32::from_bits(u32_at(bytes, at)?))
}

/// The zones of a `ZoneList`'s main block, in the order of the list.
pub fn parse(main: &[u8]) -> Result<Vec<Zone>> {
    let Some(count) = u32_at(main, COUNT_AT) else { bail!("the zone list is too short for its header") };
    let count = count as usize;
    ensure!(
        FIRST_ENTRY + count * ENTRY_LEN <= main.len(),
        "the zone list says {count} zones and holds {} bytes",
        main.len()
    );
    let mut zones = Vec::with_capacity(count);
    for n in 0..count {
        let at = FIRST_ENTRY + n * ENTRY_LEN;
        let read = |offset: usize| f32_at(main, at + offset);
        let (Some(x0), Some(z0), Some(x1), Some(z1)) =
            (read(RECT_MIN), read(RECT_MIN + 8), read(RECT_MAX), read(RECT_MAX + 8))
        else {
            bail!("zone {n} is cut short");
        };
        let (Some(unit), Some(district)) = (u32_at(main, at + UNIT_AT), u32_at(main, at + DISTRICT_AT)) else {
            bail!("zone {n} is cut short");
        };
        ensure!(x0 <= x1 && z0 <= z1, "zone {n} (unit {unit}) has an upside-down rectangle");
        zones.push(Zone { unit, district, min: [x0, z0], max: [x1, z1] });
    }
    Ok(zones)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A zone list of the given `(unit, district, x0, z0, x1, z1)` zones.
    pub(crate) fn zone_list(zones: &[(u32, u32, f32, f32, f32, f32)]) -> Vec<u8> {
        let mut out = vec![0u8; FIRST_ENTRY + zones.len() * ENTRY_LEN];
        out[COUNT_AT..COUNT_AT + 4].copy_from_slice(&(zones.len() as u32).to_le_bytes());
        for (n, &(unit, district, x0, z0, x1, z1)) in zones.iter().enumerate() {
            let at = FIRST_ENTRY + n * ENTRY_LEN;
            for (offset, value) in [
                (RECT_MIN, x0),
                (RECT_MIN + 4, -f32::MAX),
                (RECT_MIN + 8, z0),
                (RECT_MAX, x1),
                (RECT_MAX + 4, f32::MAX),
                (RECT_MAX + 8, z1),
            ] {
                out[at + offset..at + offset + 4].copy_from_slice(&value.to_le_bytes());
            }
            out[at + UNIT_AT..at + UNIT_AT + 4].copy_from_slice(&unit.to_le_bytes());
            out[at + DISTRICT_AT..at + DISTRICT_AT + 4].copy_from_slice(&district.to_le_bytes());
        }
        out
    }

    #[test]
    fn the_zones_of_a_list_are_read_with_their_rectangles() {
        let zones =
            parse(&zone_list(&[(161, 164408, 1436.0, 1869.0, 2026.0, 2329.0), (1, 164408, -10.0, -20.0, 10.0, 20.0)]))
                .unwrap();
        assert_eq!(zones.len(), 2);
        assert_eq!((zones[0].unit, zones[0].district), (161, 164408));
        assert_eq!(zones[0].min, [1436.0, 1869.0]);
        assert_eq!(zones[0].max, [2026.0, 2329.0]);
    }

    #[test]
    fn the_centre_is_in_render_space_and_the_radius_holds_the_corners() {
        let zone = Zone { unit: 1, district: 2, min: [0.0, 100.0], max: [300.0, 500.0] };
        // Engine x is render y and engine z is render x.
        assert_eq!(zone.centre_render(), [300.0, 150.0]);
        assert_eq!(zone.radius(), 250.0);
        assert!(zone.contains(310.0, 505.0, 25.0));
        assert!(!zone.contains(330.0, 300.0, 25.0));
    }

    #[test]
    fn a_short_list_and_an_upside_down_rectangle_are_refused() {
        assert!(parse(&[0; 8]).is_err());
        let mut bytes = zone_list(&[(1, 1, 0.0, 0.0, 1.0, 1.0)]);
        bytes.truncate(FIRST_ENTRY + 10);
        assert!(parse(&bytes).is_err());
        assert!(parse(&zone_list(&[(1, 1, 5.0, 0.0, 1.0, 1.0)])).is_err());
    }
}
