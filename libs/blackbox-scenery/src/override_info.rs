//! `SceneryOverrideInfos` (chunk `0x34108` of the track bundle): one record per scenery instance that a scenery group
//! can switch, and the flags the engine writes into that instance. Spec: `docs/formats/maps.md`
//! ("Scenery overrides and groups") and `docs/specs/race-barriers.md`.

use blackbox_chunk::ids;

use crate::{Error, Result};

/// Bytes per record.
pub const OVERRIDE_RECORD_LEN: usize = 6;

/// Override flag: the instance is drawn in no view (the visibility rule's `0x10` bit).
pub const OVERRIDE_HIDDEN: u16 = 0x0010;
/// Override flag: the instance is turned; an instance with [`INSTANCE_TURNABLE`] is mirrored along its local x axis
/// while this bit differs from its stored flags.
pub const OVERRIDE_TURNED: u16 = 0x0400;
/// Instance flag (bit 23 of the instance's flag word): the engine mirrors the instance when an override flips
/// [`OVERRIDE_TURNED`]. Instances without it keep their matrix.
pub const INSTANCE_TURNABLE: u32 = 0x0080_0000;

/// One override record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneryOverride {
    /// The streaming section number of the instance (`TrackStreamingSection::number`).
    pub section: i16,
    /// The instance's index in that section's scenery instance list.
    pub instance: i16,
    /// The low 16 bits of the instance's flag word while the override applies. The shipped records equal the stored
    /// instance flags.
    pub flags: u16,
}

impl SceneryOverride {
    /// The flags after the instance's group is switched on: hidden and turned cleared, turned set again for a
    /// starred switch.
    pub const fn enabled(flags: u16, turned: bool) -> u16 {
        let cleared = flags & !(OVERRIDE_HIDDEN | OVERRIDE_TURNED);
        match turned {
            true => cleared | OVERRIDE_TURNED,
            false => cleared,
        }
    }

    /// The flags after the instance's group is switched off: hidden set, turned kept.
    pub const fn disabled(flags: u16) -> u16 {
        flags | OVERRIDE_HIDDEN
    }
}

/// Every record of a `SceneryOverrideInfos` payload (the chunk's bytes, header excluded).
pub fn parse_overrides(payload: &[u8]) -> Result<Vec<SceneryOverride>> {
    let (records, rest) = payload.as_chunks::<OVERRIDE_RECORD_LEN>();
    if !rest.is_empty() {
        return Err(Error::Overrides {
            detail: format!("{} bytes is not a multiple of {OVERRIDE_RECORD_LEN}", payload.len()),
        });
    }
    Ok(records
        .iter()
        .map(|r| SceneryOverride {
            section: i16::from_le_bytes([r[0], r[1]]),
            instance: i16::from_le_bytes([r[2], r[3]]),
            flags: u16::from_le_bytes([r[4], r[5]]),
        })
        .collect())
}

/// Every override record in a track bundle (`TRACKS/L2RA.BUN`). A bundle without the chunk has none. The record's
/// position is its global override index, the number scenery groups and section override hooks refer to.
pub fn read_scenery_overrides(bundle: &[u8]) -> Result<Vec<SceneryOverride>> {
    let Some(chunk) = blackbox_chunk::find(bundle, ids::SCENERY_OVERRIDE_INFOS) else {
        return Ok(Vec::new());
    };
    parse_overrides(chunk.payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(id: u32, payload: &[u8]) -> Vec<u8> {
        let mut v = id.to_le_bytes().to_vec();
        v.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        v.extend_from_slice(payload);
        v
    }

    fn record(section: i16, instance: i16, flags: u16) -> Vec<u8> {
        [section.to_le_bytes(), instance.to_le_bytes(), flags.to_le_bytes()].concat()
    }

    #[test]
    fn records_are_section_instance_and_flags() {
        let payload = [record(1538, 24, 0x8000), record(743, 3, 0x8070)].concat();
        let overrides = parse_overrides(&payload).unwrap();
        assert_eq!(
            overrides,
            [
                SceneryOverride { section: 1538, instance: 24, flags: 0x8000 },
                SceneryOverride { section: 743, instance: 3, flags: 0x8070 },
            ]
        );
    }

    #[test]
    fn a_payload_that_does_not_divide_is_an_error() {
        let err = parse_overrides(&[0; 7]).unwrap_err();
        assert!(matches!(err, Error::Overrides { .. }), "{err}");
    }

    #[test]
    fn a_bundle_without_the_chunk_has_none() {
        assert!(read_scenery_overrides(&chunk(ids::SCENERY_GROUPS, &[])).unwrap().is_empty());
        let bundle = chunk(ids::SCENERY_OVERRIDE_INFOS, &record(135, 18, 0x8070));
        assert_eq!(read_scenery_overrides(&bundle).unwrap().len(), 1);
    }

    #[test]
    fn switching_on_clears_hidden_and_sets_turned_only_when_starred() {
        assert_eq!(SceneryOverride::enabled(0x8070, false), 0x8060);
        assert_eq!(SceneryOverride::enabled(0x8070, true), 0x8460);
        assert_eq!(SceneryOverride::enabled(0x8470, false), 0x8060, "a plain switch un-turns");
        assert_eq!(SceneryOverride::disabled(0x8460), 0x8470, "switching off keeps the turn");
    }
}
