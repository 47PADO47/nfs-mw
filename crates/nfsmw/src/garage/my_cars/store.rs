//! The v2 store is replaced atomically. Reading migration inputs does not modify them.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};

use super::{Selection, Slot};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Record {
    pub label: String,
    pub selection: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct Store {
    version: u32,
    next_id: u64,
    migrated: bool,
    #[serde(default = "default_limit")]
    pub slot_limit: usize,
    pub slots: BTreeMap<String, Record>,
    pub deleted: BTreeSet<String>,
}

impl Default for Store {
    fn default() -> Self {
        Self {
            version: 2,
            next_id: 1,
            migrated: false,
            slot_limit: default_limit(),
            slots: BTreeMap::new(),
            deleted: BTreeSet::new(),
        }
    }
}

fn default_limit() -> usize {
    20
}

impl Store {
    pub fn read(path: &Path, legacy: Option<&Path>) -> Result<Self> {
        let mut store = match std::fs::read_to_string(path) {
            Ok(text) => toml::from_str::<Self>(&text).context("Invalid My Cars slot file")?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(error) => return Err(error).context("Reading My Cars slots"),
        };
        ensure!(store.version == 2, "Unsupported My Cars slot schema");
        ensure!(store.slot_limit > 0, "My Cars slot capacity must be positive");
        if store.migrated {
            return Ok(store);
        }
        if let Some(legacy) = legacy {
            for name in crate::preset_store::names(legacy)? {
                let selection = crate::preset_store::load(legacy, &name)?;
                let id = format!("legacy-{name}");
                if !store.deleted.contains(&id) {
                    store.slots.entry(id).or_insert(Record { label: name, selection });
                }
            }
        }
        store.migrated = true;
        Ok(store)
    }

    pub fn metadata(&self) -> Result<Vec<Slot>> {
        self.slots
            .iter()
            .map(|(id, record)| {
                let payload: toml::Table = record.selection.parse().context("Invalid custom car selection")?;
                let car = payload
                    .get("car")
                    .and_then(toml::Value::as_str)
                    .context("Custom car slot has no vehicle identity")?;
                Ok(Slot { id: id.clone(), label: record.label.clone(), car: car.to_owned(), vehicle_key: None })
            })
            .collect()
    }

    pub fn create(&mut self, selection: &Selection, label: &str) -> Result<String> {
        ensure!(self.slots.len() < self.slot_limit, "My Cars is full ({} slots)", self.slot_limit);
        let payload = crate::garage::presets::encode(selection)?;
        let id = format!("slot-{:016x}", self.next_id);
        self.next_id = self.next_id.checked_add(1).context("Custom slot identity limit reached")?;
        ensure!(!self.slots.contains_key(&id) && !self.deleted.contains(&id), "Custom slot identity was already used");
        self.slots.insert(id.clone(), Record { label: label.to_owned(), selection: payload });
        Ok(id)
    }

    pub fn save(&mut self, id: &str, selection: &Selection, label: &str) -> Result<()> {
        ensure!(!self.deleted.contains(id), "Custom car slot was deleted");
        ensure!(self.slots.contains_key(id) || id.starts_with("native-"), "Custom car slot is missing");
        let selection = crate::garage::presets::encode(selection)?;
        self.slots.insert(id.to_owned(), Record { label: label.to_owned(), selection });
        Ok(())
    }

    pub fn write(&self, path: &Path) -> Result<()> {
        let parent = path.parent().context("Missing My Cars folder")?;
        let text = toml::to_string(self).context("Serializing My Cars slots")?;
        std::fs::create_dir_all(parent).context("Creating My Cars folder")?;
        let temporary = path.with_extension("toml.tmp");
        std::fs::write(&temporary, text).context("Staging My Cars slots")?;
        std::fs::rename(temporary, path).context("Replacing My Cars slots")
    }
}
