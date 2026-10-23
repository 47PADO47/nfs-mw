//! The best time and the best speed of each event, shown on the track list (`docs/specs/custom-race.md` section 3). The original
//! keeps them in the profile's race scores (`GRaceSaveInfo::mHighScores`); this build keeps them in `records.toml` next to the
//! config file, whatever profile is played, as the challenge series keeps its wins.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

const FILE: &str = "records.toml";

/// What the player did best on one event.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Record {
    /// The fastest finish, seconds.
    pub time: Option<f32>,
    /// The fastest lap, seconds.
    pub lap: Option<f32>,
    /// The highest speed through a speed trap, km/h.
    pub speed: Option<f32>,
    /// The highest average speed of a finished run and the highest speed reached in one, km/h (the race events sheet).
    pub average: Option<f32>,
    pub top: Option<f32>,
}

#[derive(Debug, Default, Clone, PartialEq)]
struct Records(BTreeMap<String, Record>);

impl Records {
    fn note_time(&mut self, event: &str, seconds: f32) -> bool {
        let record = self.0.entry(event.to_owned()).or_default();
        let better = seconds > 0.0 && record.time.is_none_or(|best| seconds < best);
        if better {
            record.time = Some(seconds);
        }
        better
    }

    fn note_lap(&mut self, event: &str, seconds: f32) -> bool {
        let record = self.0.entry(event.to_owned()).or_default();
        let better = seconds > 0.0 && record.lap.is_none_or(|best| seconds < best);
        if better {
            record.lap = Some(seconds);
        }
        better
    }

    fn note_speed(&mut self, event: &str, kmh: f32) -> bool {
        let record = self.0.entry(event.to_owned()).or_default();
        let better = kmh > 0.0 && record.speed.is_none_or(|best| kmh > best);
        if better {
            record.speed = Some(kmh);
        }
        better
    }

    /// Keeps the average and the top speed of a finished run when they beat the record's.
    fn note_run(&mut self, event: &str, average: f32, top: f32) -> bool {
        let record = self.0.entry(event.to_owned()).or_default();
        let faster = |best: Option<f32>, now: f32| now > 0.0 && best.is_none_or(|best| now > best);
        let (new_average, new_top) = (faster(record.average, average), faster(record.top, top));
        if new_average {
            record.average = Some(average);
        }
        if new_top {
            record.top = Some(top);
        }
        new_average || new_top
    }

    fn to_toml(&self) -> String {
        let mut events = toml::Table::new();
        for (event, record) in &self.0 {
            let mut entry = toml::Table::new();
            if let Some(time) = record.time {
                entry.insert("time".into(), toml::Value::Float(f64::from(time)));
            }
            if let Some(lap) = record.lap {
                entry.insert("lap".into(), toml::Value::Float(f64::from(lap)));
            }
            if let Some(speed) = record.speed {
                entry.insert("speed".into(), toml::Value::Float(f64::from(speed)));
            }
            if let Some(average) = record.average {
                entry.insert("average".into(), toml::Value::Float(f64::from(average)));
            }
            if let Some(top) = record.top {
                entry.insert("top".into(), toml::Value::Float(f64::from(top)));
            }
            events.insert(event.clone(), toml::Value::Table(entry));
        }
        let mut table = toml::Table::new();
        table.insert("events".into(), toml::Value::Table(events));
        toml::to_string(&table).unwrap_or_default()
    }

    /// The records a file's text holds; text that does not parse holds none.
    fn from_toml(text: &str) -> Self {
        let Ok(table) = text.parse::<toml::Table>() else { return Self::default() };
        let Some(events) = table.get("events").and_then(toml::Value::as_table) else { return Self::default() };
        let number = |entry: &toml::Table, key: &str| entry.get(key).and_then(toml::Value::as_float).map(|n| n as f32);
        let read = |(event, value): (&String, &toml::Value)| {
            let entry = value.as_table()?;
            Some((
                event.clone(),
                Record {
                    time: number(entry, "time"),
                    lap: number(entry, "lap"),
                    speed: number(entry, "speed"),
                    average: number(entry, "average"),
                    top: number(entry, "top"),
                },
            ))
        };
        Self(events.iter().filter_map(read).collect())
    }
}

fn path() -> Option<PathBuf> {
    Some(crate::settings::Settings::config_path()?.parent()?.join(FILE))
}

static SHARED: Mutex<Option<Records>> = Mutex::new(None);

fn with_shared<T>(f: impl FnOnce(&mut Records) -> T) -> T {
    let mut guard = SHARED.lock().unwrap_or_else(|e| e.into_inner());
    let records = guard.get_or_insert_with(|| {
        Records::from_toml(&path().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default())
    });
    f(records)
}

/// The best of an event (nothing for an event never raced).
pub fn best(event: &str) -> Record {
    with_shared(|records| records.0.get(event).copied().unwrap_or_default())
}

/// Notes a finish time and writes the file when it is the best.
pub fn note_time(event: &str, seconds: f32) {
    write(with_shared(|records| records.note_time(event, seconds).then(|| records.to_toml())));
}

/// Notes the time of a lap and writes the file when it is the best.
pub fn note_lap(event: &str, seconds: f32) {
    write(with_shared(|records| records.note_lap(event, seconds).then(|| records.to_toml())));
}

/// Notes the average and the top speed (km/h) of a finished run and writes the file when either is the best.
pub fn note_run(event: &str, average: f32, top: f32) {
    write(with_shared(|records| records.note_run(event, average, top).then(|| records.to_toml())));
}

/// Notes the speed of a pass through a speed trap and writes the file when it is the best.
pub fn note_speed(event: &str, kmh: f32) {
    write(with_shared(|records| records.note_speed(event, kmh).then(|| records.to_toml())));
}

fn write(text: Option<String>) {
    let (Some(text), Some(path)) = (text, path()) else { return };
    if let Some(folder) = path.parent()
        && let Err(e) = std::fs::create_dir_all(folder)
    {
        return log::warn!("records: cannot make {}: {e}", folder.display());
    }
    if let Err(e) = std::fs::write(&path, text) {
        log::warn!("records: cannot write {}: {e}", path.display());
    }
}

/// A time as the track list shows it: `0.00` for none, else minutes, seconds and hundredths (`1:23.45`) **[ours]**.
pub fn time_text(seconds: Option<f32>) -> String {
    let Some(seconds) = seconds.filter(|s| *s > 0.0) else { return "0.00".to_owned() };
    let hundredths = (seconds * 100.0).round() as u32;
    format!("{}:{:02}.{:02}", hundredths / 6000, hundredths / 100 % 60, hundredths % 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_better_time_or_speed_is_kept() {
        let mut records = Records::default();
        assert!(records.note_time("10.2.1", 90.0));
        assert!(!records.note_time("10.2.1", 95.0));
        assert!(records.note_time("10.2.1", 80.5));
        assert!(records.note_speed("10.2.1", 200.0));
        assert!(!records.note_speed("10.2.1", 150.0));
        assert!(!records.note_time("10.2.1", 0.0));
        assert!(records.note_run("10.2.1", 120.0, 190.0));
        assert!(records.note_run("10.2.1", 110.0, 200.0), "a better top speed counts alone");
        assert!(!records.note_run("10.2.1", 100.0, 150.0));
        assert_eq!(records.0["10.2.1"].average, Some(120.0));
        assert_eq!(records.0["10.2.1"].top, Some(200.0));
        assert!(records.note_lap("10.2.1", 40.0));
        assert!(!records.note_lap("10.2.1", 41.0));
        assert_eq!(
            records.0["10.2.1"],
            Record { time: Some(80.5), lap: Some(40.0), speed: Some(200.0), average: Some(120.0), top: Some(200.0) }
        );
    }

    #[test]
    fn the_file_text_round_trips_and_bad_text_holds_nothing() {
        let mut records = Records::default();
        records.note_time("10.2.1", 83.25);
        records.note_speed("4.1.2", 212.0);
        assert_eq!(Records::from_toml(&records.to_toml()), records);
        assert_eq!(Records::from_toml("not toml ["), Records::default());
    }

    #[test]
    fn a_time_reads_as_minutes_seconds_and_hundredths() {
        assert_eq!(time_text(None), "0.00");
        assert_eq!(time_text(Some(83.25)), "1:23.25");
        assert_eq!(time_text(Some(5.0)), "0:05.00");
    }
}
