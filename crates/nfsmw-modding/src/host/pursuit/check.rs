//! The checks a mod's pursuit values pass before a backend sees them: level, finite floats, known enum numbers, names and
//! chances of a cop wave.

use blackbox_modding::abi::{BB_ERR_ARG, BbStatus};

use crate::abi::{BbCopRecord, BbHeatRow, CopTable, MW_HEAT_LEVELS};

/// A heat level as a mod passes it: `1..=MW_HEAT_LEVELS`.
pub(super) fn level(raw: u32) -> Result<u32, BbStatus> {
    if raw == 0 || raw > MW_HEAT_LEVELS {
        return Err(BB_ERR_ARG);
    }
    Ok(raw)
}

/// A cop table number.
pub(super) fn table(raw: u32) -> Result<CopTable, BbStatus> {
    CopTable::from_raw(raw).ok_or(BB_ERR_ARG)
}

/// A value a mod sends that must be a finite number.
pub(super) fn finite(value: f32) -> Result<f32, BbStatus> {
    if !value.is_finite() {
        return Err(BB_ERR_ARG);
    }
    Ok(value)
}

/// A finite, non-negative value (an interval in seconds).
pub(super) fn interval(value: f32) -> Result<f32, BbStatus> {
    let value = finite(value)?;
    if value < 0.0 {
        return Err(BB_ERR_ARG);
    }
    Ok(value)
}

/// A whole wave a mod sends: `cop_count` must match the records, the intervals must be valid, and each record needs a
/// name (UTF-8, not empty) and a chance of at most 100 percent.
pub(super) fn wave(row: &BbHeatRow, cops: &[BbCopRecord]) -> Result<(), BbStatus> {
    if row.cop_count as usize != cops.len() {
        return Err(BB_ERR_ARG);
    }
    interval(row.first_four_interval)?;
    interval(row.spawn_interval)?;
    for cop in cops {
        let name = cop.name.text().ok_or(BB_ERR_ARG)?;
        if name.is_empty() || cop.chance > 100 {
            return Err(BB_ERR_ARG);
        }
    }
    Ok(())
}
