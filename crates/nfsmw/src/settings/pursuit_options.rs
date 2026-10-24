//! Optional pursuit limit changes. Reference event rules remain the default.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PursuitOptions {
    pub enhanced: bool,
    pub max_active_cops: u32,
    /// Zero preserves the event/career maximum; 1..10 explicitly replaces it.
    pub heat_cap: u32,
}

impl Default for PursuitOptions {
    fn default() -> Self {
        Self { enhanced: false, max_active_cops: 16, heat_cap: 0 }
    }
}

impl PursuitOptions {
    pub fn sanitized(self) -> Self {
        Self { max_active_cops: self.max_active_cops.clamp(8, 64), heat_cap: self.heat_cap.min(10), ..self }
    }

    pub fn active_limit(self, reference: u32) -> u32 {
        match self.enhanced {
            true => self.sanitized().max_active_cops,
            false => reference,
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
    }

    #[test]
    fn file_values_are_bounded_without_changing_the_disabled_state() {
        let options = PursuitOptions { enhanced: false, max_active_cops: 500, heat_cap: 99 }.sanitized();
        assert_eq!(options.max_active_cops, 64);
        assert_eq!(options.heat_cap, 10);
        assert_eq!(options.active_limit(8), 8);
        assert_eq!(options.maximum_heat(), None);
    }
}
