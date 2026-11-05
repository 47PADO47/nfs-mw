//! Optional pursuit limit changes. Reference event rules remain the default.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PursuitOptions {
    pub enhanced: bool,
    pub max_active_cops: u32,
    /// Zero preserves the event/career maximum; 1..10 explicitly replaces it.
    pub heat_cap: u32,
    /// The heat table's cop counts a chase uses, percent: the wave, the cops to evade before backup and the cool-down
    /// search. 100 keeps the reference counts.
    pub wave_percent: u32,
    /// The bounty bug fixes of a takedown (`docs/specs/ai-pursuit-heat.md` section 5.5). On by default, independent of
    /// the limit enhancement; off restores the original's bugs.
    pub bounty_fixes: bool,
}

impl Default for PursuitOptions {
    fn default() -> Self {
        Self { enhanced: false, max_active_cops: 16, heat_cap: 0, wave_percent: 100, bounty_fixes: true }
    }
}

impl PursuitOptions {
    pub fn sanitized(self) -> Self {
        Self {
            max_active_cops: self.max_active_cops.clamp(8, 64),
            heat_cap: self.heat_cap.min(10),
            wave_percent: self.wave_percent.clamp(100, 400),
            ..self
        }
    }

    pub fn active_limit(self, reference: u32) -> u32 {
        match self.enhanced {
            true => self.sanitized().max_active_cops,
            false => reference,
        }
    }

    /// How much the heat table's cop counts grow: 1 unless the enhancement is on.
    pub fn wave_scale(self) -> f32 {
        match self.enhanced {
            true => self.sanitized().wave_percent as f32 / 100.0,
            false => 1.0,
        }
    }

    pub fn maximum_heat(self) -> Option<f32> {
        (self.enhanced && self.heat_cap > 0).then_some(self.heat_cap.min(10) as f32)
    }

    pub(crate) fn read(value: &toml::Value, origin: &str) -> Option<Self> {
        match value.clone().try_into::<Self>() {
            Ok(value) => Some(value.sanitized()),
            Err(error) => {
                log::warn!("{origin}: invalid pursuit options: {error}");
                None
            }
        }
    }

    pub(crate) fn merge(self, old: Option<&toml::Value>) -> anyhow::Result<toml::Value> {
        let mut table = old.and_then(toml::Value::as_table).cloned().unwrap_or_default();
        let toml::Value::Table(update) = toml::Value::try_from(self.sanitized())? else { unreachable!() };
        table.extend(update);
        Ok(toml::Value::Table(table))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enhancements_do_not_change_reference_limits_until_enabled() {
        let reference = PursuitOptions::default();
        assert_eq!(reference.active_limit(8), 8);
        assert_eq!(reference.maximum_heat(), None);
        let enhanced = PursuitOptions { enhanced: true, ..reference };
        assert_eq!(enhanced.active_limit(8), 16);
        assert_eq!(enhanced.maximum_heat(), None, "lifting the active cap alone keeps event heat rules");
        assert_eq!(PursuitOptions { heat_cap: 7, ..enhanced }.maximum_heat(), Some(7.0));
        assert_eq!(PursuitOptions { wave_percent: 200, ..reference }.wave_scale(), 1.0, "off keeps the waves");
        assert_eq!(PursuitOptions { wave_percent: 200, ..enhanced }.wave_scale(), 2.0);
    }

    #[test]
    fn file_values_are_bounded_without_changing_the_disabled_state() {
        let options = PursuitOptions {
            enhanced: false,
            max_active_cops: 500,
            heat_cap: 99,
            wave_percent: 900,
            bounty_fixes: false,
        }
        .sanitized();
        assert_eq!(options.max_active_cops, 64);
        assert_eq!(options.wave_percent, 400);
        assert_eq!(options.heat_cap, 10);
        assert_eq!(options.active_limit(8), 8);
        assert_eq!(options.maximum_heat(), None);
        assert!(!options.bounty_fixes, "turning the fixes off is kept");
    }

    #[test]
    fn the_bounty_fixes_are_on_by_default_and_for_an_older_file() {
        assert!(PursuitOptions::default().bounty_fixes);
        let old: toml::Value = toml::from_str(
            "enhanced = true
max_active_cops = 20",
        )
        .unwrap();
        assert!(PursuitOptions::read(&old, "test").unwrap().bounty_fixes);
    }
}
