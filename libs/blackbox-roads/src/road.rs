//! Roads (`RNrd`): named chains of segments.

use crate::Result;
use crate::bytes::Reader;

pub(crate) const ROAD_LEN: usize = 8;

#[derive(Debug, Clone, PartialEq)]
pub struct Road {
    /// Multiplies the length of shortcut segments when path distance is summed (1.0 in the file).
    pub scale: f32,
    /// Street-name id for cop dispatch (0 when unnamed).
    pub speech_id: u16,
}

impl Road {
    pub(crate) fn parse(r: &Reader<'_>, at: usize) -> Result<Self> {
        Ok(Self { scale: f32::from(r.u16(at)?) / 256.0, speech_id: r.u16(at + 6)? })
    }
}
