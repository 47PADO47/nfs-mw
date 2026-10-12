//! Persistent visual damage of one vehicle.
//!
//! [`Damage`] keeps, per impact zone, three levels: panel deformation, paint scrape and glass crack.
//! An impact is a severity from 0 (no contact) to [`MAX_SEVERITY`] plus whether the zone is glazed.
//! Impacts only raise levels; [`Damage::repair`] is the only reset. Spec: `docs/specs/vehicle-damage.md`.

mod zone;

#[cfg(test)]
mod tests;

pub use zone::Zone;

/// The highest level of any damage aspect.
pub const MAX_LEVEL: u8 = 3;
/// The highest impact severity; severities above it count as this one.
pub const MAX_SEVERITY: u8 = 6;
/// Severity from which a glazed zone shatters at once.
pub const SHATTER_SEVERITY: u8 = 5;
/// Severity from which a panel deforms.
const DENT_SEVERITY: u8 = 2;

/// The damage of one vehicle: a fixed set of zones and one punctured flag per wheel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Damage {
    zones: Vec<Zone>,
    punctured: [bool; 4],
}

impl Damage {
    /// An undamaged vehicle with `zones` impact zones.
    pub fn new(zones: usize) -> Self {
        Self { zones: vec![Zone::default(); zones], punctured: [false; 4] }
    }

    /// Record an impact of `severity` on `zone`; returns whether any level changed.
    /// Out-of-range zones and severity 0 change nothing.
    pub fn impact(&mut self, zone: usize, severity: u8, glazed: bool) -> bool {
        let Some(zone) = self.zones.get_mut(zone) else { return false };
        let before = *zone;
        let severity = severity.min(MAX_SEVERITY);
        if severity == 0 {
            return false;
        }
        zone.paint = zone.paint.max(paint_level(severity));
        if severity >= DENT_SEVERITY {
            zone.panel = (zone.panel + 1).min(MAX_LEVEL);
        }
        if glazed && severity >= DENT_SEVERITY {
            zone.glass = (zone.glass + 1).min(MAX_LEVEL);
        }
        if glazed && severity >= SHATTER_SEVERITY {
            zone.glass = MAX_LEVEL;
        }
        *zone != before
    }

    /// The levels of `zone`; zones out of range read as undamaged.
    pub fn zone(&self, zone: usize) -> Zone {
        self.zones.get(zone).copied().unwrap_or_default()
    }

    /// The number of impact zones.
    pub fn zone_count(&self) -> usize {
        self.zones.len()
    }

    /// Puncture `wheel` (0..4 in model order); returns whether it was intact.
    pub fn puncture(&mut self, wheel: usize) -> bool {
        let Some(flag) = self.punctured.get_mut(wheel) else { return false };
        let intact = !*flag;
        *flag = true;
        intact
    }

    /// Whether `wheel` is punctured.
    pub fn is_punctured(&self, wheel: usize) -> bool {
        self.punctured.get(wheel).copied().unwrap_or(false)
    }

    /// Whether anything is damaged, including a puncture.
    pub fn is_damaged(&self) -> bool {
        self.zones.iter().any(|z| *z != Zone::default()) || self.punctured.iter().any(|&p| p)
    }

    /// Clear every zone and puncture.
    pub fn repair(&mut self) {
        self.zones.iter_mut().for_each(|z| *z = Zone::default());
        self.punctured = [false; 4];
    }
}

/// Paint scrapes from light contact and deepens with heavier contact.
fn paint_level(severity: u8) -> u8 {
    match severity {
        0 => 0,
        1..=2 => 1,
        3..=4 => 2,
        _ => MAX_LEVEL,
    }
}
