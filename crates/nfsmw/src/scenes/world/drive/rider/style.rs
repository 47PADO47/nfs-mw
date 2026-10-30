//! `styles.json`: which blend tree each kind of bike has, and the numbers of its reverse walk.
//!
//! A mod ships one rider rig for all its bikes, and one blend tree per kind of bike (a sport bike's rider sits and leans
//! differently from a scooter's). The rig's `styles.json` lists the trees by name; a bike's `contacts.json` names the kind it is
//! (`"style": {"name": ...}`). A bike whose kind is not listed, or a rig without the file, plays the rig's `graph.json`
//! (`docs/formats/ride6-rider.md`, "styles.json").

use anyhow::{Context, Result, ensure};
use game_install::GameDir;
use serde::Deserialize;

/// The only version of the file this reads.
pub const VERSION: u32 = 1;

/// The reverse walk's numbers, in the units of the animation blueprint (speeds in km/h, negative backwards).
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
pub struct Walk {
    /// The part of the start clip that plays, from and to (seconds of the clip).
    pub start_seconds: [f32; 2],
    /// The walk begins when the speed is below this.
    pub start_speed_kmh: f32,
    /// The speeds at which the loop plays at the first and at the second play rate.
    pub rate_speed_kmh: [f32; 2],
    pub rate: [f32; 2],
    /// The length of the loop clip (seconds): one cycle of it is the walk's phase wrapping from 0 to 1.
    pub loop_seconds: f32,
}

/// One kind of bike.
#[derive(Debug, Clone, Deserialize)]
pub struct Entry {
    /// The blend tree's file in the rider folder.
    pub graph: String,
    /// Its reverse walk; a kind without one never walks the bike back.
    #[serde(default)]
    pub reverse: Option<Walk>,
}

/// The file.
#[derive(Debug, Clone, Deserialize)]
pub struct Styles {
    pub version: u32,
    pub styles: std::collections::BTreeMap<String, Entry>,
}

fn finite(values: &[f32]) -> bool {
    values.iter().all(|v| v.is_finite())
}

impl Walk {
    /// Refuses numbers a walk cannot run on: not finite, a start window that runs backwards, no loop length.
    fn check(&self) -> Result<()> {
        ensure!(
            finite(&self.start_seconds)
                && finite(&self.rate_speed_kmh)
                && finite(&self.rate)
                && finite(&[self.start_speed_kmh, self.loop_seconds]),
            "the reverse walk has a number that is not finite"
        );
        ensure!(
            self.start_seconds[0] >= 0.0 && self.start_seconds[0] <= self.start_seconds[1],
            "the start window runs backwards"
        );
        ensure!(self.loop_seconds > 0.0, "the loop clip has no length");
        ensure!(self.rate_speed_kmh[0] != self.rate_speed_kmh[1], "the play rate does not depend on the speed");
        ensure!(self.rate.iter().all(|r| *r > 0.0), "a play rate is not positive");
        Ok(())
    }
}

impl Styles {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let styles: Self = serde_json::from_slice(bytes).context("styles.json")?;
        ensure!(styles.version == VERSION, "styles.json is version {}, this reads version {VERSION}", styles.version);
        for (name, entry) in &styles.styles {
            ensure!(
                !entry.graph.is_empty()
                    && !entry.graph.contains(['/', '\\'])
                    && !entry.graph.contains("..")
                    && entry.graph.ends_with(".json"),
                "style {name}: `{}` is not a file of the rider folder",
                entry.graph
            );
            if let Some(walk) = &entry.reverse {
                walk.check().with_context(|| format!("style {name}"))?;
            }
        }
        Ok(styles)
    }

    /// The entry of the kind `name`.
    pub fn get(&self, name: &str) -> Option<&Entry> {
        self.styles.get(name)
    }
}

/// The graph file a rig without `styles.json` (or a bike whose kind it does not list) plays.
pub const DEFAULT_GRAPH: &str = "graph.json";

/// The blend tree a bike plays and the walk it has.
#[derive(Debug, Clone, PartialEq)]
pub struct Selection {
    pub graph: String,
    pub walk: Option<Walk>,
}

impl Selection {
    fn default_graph() -> Self {
        Self { graph: DEFAULT_GRAPH.to_owned(), walk: None }
    }
}

/// The graph and the walk of the kind `kind` of bike in the rider folder `folder`. A bike with no kind, a rig with no (or a
/// damaged) `styles.json`, a kind it does not list or a graph file that is missing play `graph.json` with no walk.
pub fn select(dir: &GameDir, folder: &str, kind: Option<&str>) -> Selection {
    let Some(kind) = kind else { return Selection::default_graph() };
    let path = format!("{folder}/styles.json");
    if !dir.exists(&path) {
        log::info!("rider: {folder} has no styles.json; the {kind} bike plays {DEFAULT_GRAPH}");
        return Selection::default_graph();
    }
    let table = dir.read(&path).with_context(|| format!("reading {path}")).and_then(|bytes| Styles::parse(&bytes));
    let table = match table {
        Ok(table) => table,
        Err(error) => {
            log::warn!("rider: {path} is not used: {error:#}");
            return Selection::default_graph();
        }
    };
    let Some(entry) = table.get(kind) else {
        log::info!("rider: {path} has no {kind} style; the bike plays {DEFAULT_GRAPH}");
        return Selection::default_graph();
    };
    if !dir.exists(&format!("{folder}/{}", entry.graph)) {
        log::warn!("rider: the {kind} style's graph {} is missing from {folder}", entry.graph);
        return Selection::default_graph();
    }
    Selection { graph: entry.graph.clone(), walk: entry.reverse }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = r#"{
        "version": 1,
        "styles": {
            "race": {"bike_type": 0, "graph": "graph_race.json", "riding_style": 0,
                     "reverse": {"start_seconds": [0.8, 1.567], "start_speed_kmh": -1.0,
                                 "rate_speed_kmh": [-1.0, -20.0], "rate": [1.0, 4.0], "loop_seconds": 2.2}},
            "naked": {"graph": "graph_naked.json"}
        }
    }"#;

    #[test]
    fn the_table_gives_each_kind_its_graph_and_walk() {
        let styles = Styles::parse(FILE.as_bytes()).unwrap();
        let race = styles.get("race").unwrap();
        assert_eq!(race.graph, "graph_race.json");
        assert_eq!(race.reverse.unwrap().loop_seconds, 2.2);
        assert!(styles.get("naked").unwrap().reverse.is_none());
        assert!(styles.get("cruiser").is_none());
    }

    #[test]
    fn a_table_that_would_read_outside_the_rider_folder_or_cannot_walk_is_refused() {
        let bad_graph = FILE.replace("graph_naked.json", "../graph.json");
        assert!(Styles::parse(bad_graph.as_bytes()).is_err());
        let bad_version = FILE.replace("\"version\": 1", "\"version\": 2");
        assert!(Styles::parse(bad_version.as_bytes()).is_err());
        let no_loop = FILE.replace("\"loop_seconds\": 2.2", "\"loop_seconds\": 0.0");
        assert!(Styles::parse(no_loop.as_bytes()).is_err());
        assert!(Styles::parse(b"{}").is_err());
    }
}
