//! Context-specific pursuit data. Spec: `docs/specs/ai-pursuit-context.md`.

use super::*;

/// The escalation table selected by whether the perpetrator is racing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PursuitTable {
    #[default]
    OpenWorld,
    Race,
}

impl PursuitTable {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::OpenWorld => "heattable",
            Self::Race => "racetable",
        }
    }

    /// Preserve heat indices: an incomplete table fails rather than shifting later rows down.
    pub fn rows(self, db: &Database) -> Option<Vec<HeatRow>> {
        (1..=HEAT_LEVELS)
            .map(|heat| match self {
                Self::OpenWorld => heat_row(db, heat),
                Self::Race => race_heat_row(db, heat),
            })
            .collect()
    }

    pub fn backups(self, db: &Database) -> Option<Vec<BackupRow>> {
        (1..=HEAT_LEVELS).map(|heat| backup_row_in(db, self.name(), heat)).collect()
    }

    pub fn growth(self, db: &Database) -> Vec<Option<HeatGrowth>> {
        (1..=HEAT_LEVELS).map(|heat| heat_growth_in(db, self.name(), heat)).collect()
    }

    pub fn rep(self, db: &Database) -> RepTable {
        let mut table = rep_table(db);
        table.rates = (1..=HEAT_LEVELS).map(|heat| rep_rates_in(db, self.name(), heat).unwrap_or_default()).collect();
        table
    }
}

/// The event and bin heat fields, read independently of presentation race parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EventHeat {
    pub bin_range: (f32, f32),
    pub use_world_heat: bool,
    pub force_heat: f32,
    pub max_heat: Option<f32>,
    /// Missing in the inspected PC class; retain absence instead of substituting a guessed value.
    pub max_race_heat: Option<f32>,
}

impl Default for EventHeat {
    fn default() -> Self {
        Self { bin_range: (0.0, 10.0), use_world_heat: false, force_heat: 0.0, max_heat: None, max_race_heat: None }
    }
}

impl EventHeat {
    pub fn read(db: &Database, event: &str) -> Self {
        let race = db.collection("gameplay", event);
        let parent = event.split('/').next().and_then(|name| db.collection("gameplay", name));
        Self {
            bin_range: (
                parent.and_then(|bin| bin.get_f32("BaseOpenWorldHeat")).unwrap_or(0.0),
                parent.and_then(|bin| bin.get_f32("MaxOpenWorldHeat")).unwrap_or(10.0),
            ),
            use_world_heat: race.and_then(|race| race.get_bool("UseWorldHeat")).unwrap_or(false),
            force_heat: race.and_then(|race| race.get_i32("ForceHeatLevel")).unwrap_or(0) as f32,
            max_heat: race.and_then(|race| race.get_f32("MaxHeatLevel")),
            max_race_heat: race.and_then(|race| race.get_f32("MaxRaceHeatLevel")),
        }
    }

    pub fn range(self) -> (f32, f32) {
        if self.use_world_heat {
            return (self.force_heat, self.max_race_heat.unwrap_or(self.bin_range.1));
        }
        (self.bin_range.0, self.max_heat.map_or(self.bin_range.1, |max| max.min(self.bin_range.1)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event() -> EventHeat {
        EventHeat {
            bin_range: (0.0, 7.0),
            use_world_heat: false,
            force_heat: 4.0,
            max_heat: Some(5.0),
            max_race_heat: None,
        }
    }

    #[test]
    fn event_maximum_can_lower_but_not_raise_the_bin_limit() {
        assert_eq!(event().range(), (0.0, 5.0));
        assert_eq!(EventHeat { max_heat: Some(9.0), ..event() }.range(), (0.0, 7.0));
    }

    #[test]
    fn world_heat_uses_force_and_race_max_with_a_documented_missing_field_fallback() {
        let world = EventHeat { use_world_heat: true, ..event() };
        assert_eq!(world.range(), (4.0, 7.0));
        assert_eq!(EventHeat { max_race_heat: Some(6.0), ..world }.range(), (4.0, 6.0));
    }
}
