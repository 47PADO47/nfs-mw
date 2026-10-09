//! Lane types: which zone types a navigator may drive on and aim for.
//! Spec: `docs/specs/ai-road-network.md` (§1.3).

use crate::zone;

/// How a car picks lanes (`WRoadNav::eLaneType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneType {
    Racing,
    Traffic,
    Drag,
    Cop,
    CopReckless,
    Reset,
    StartingGrid,
    Any,
}

const fn bit(kind: u8) -> u16 {
    1 << kind
}

const ALL: u16 = 0x3FFF;
const NO_BARRIER: u16 = ALL & !bit(zone::BARRIER) & !bit(zone::UNDRIVABLE);

impl LaneType {
    /// Zone types (bit `n` is type `n`) the car may drive on.
    pub fn drivable_mask(self) -> u16 {
        match self {
            LaneType::Racing | LaneType::Drag | LaneType::CopReckless | LaneType::StartingGrid => NO_BARRIER,
            LaneType::Traffic => bit(zone::TRAFFIC),
            LaneType::Cop => NO_BARRIER & !bit(zone::SIDEWALK),
            LaneType::Reset => bit(zone::TRAFFIC) | bit(zone::ROAD),
            LaneType::Any => ALL,
        }
    }

    /// Zone types the car may choose as the lane to aim for.
    pub fn selectable_mask(self) -> u16 {
        let plain = bit(zone::TRAFFIC) | bit(zone::ROAD);
        match self {
            LaneType::Racing | LaneType::Reset | LaneType::StartingGrid => plain,
            LaneType::Traffic => bit(zone::TRAFFIC),
            LaneType::Drag => NO_BARRIER & !bit(zone::SIDEWALK) & !bit(zone::CURB_MEDIAN),
            LaneType::Cop => {
                bit(zone::TRAFFIC)
                    | bit(zone::MEDIAN)
                    | bit(zone::CURB_MEDIAN)
                    | bit(zone::GRASS_MEDIAN)
                    | bit(zone::ROAD)
            }
            LaneType::CopReckless => NO_BARRIER,
            LaneType::Any => ALL,
        }
    }
}
