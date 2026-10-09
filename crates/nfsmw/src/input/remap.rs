//! Config overrides and reversible live reassignment. I/O happens only on load or explicit save.

use anyhow::{Context, Result};
use std::path::Path;
use toml::{Table, Value};

use super::{
    Action, Bindings,
    bindings::Binding,
    source::{self, Family},
};
use crate::settings::Settings;

impl Bindings {
    pub fn load(settings: &Settings) -> Self {
        let mut bindings = Self::with_paddles(settings.paddle_up, settings.paddle_down);
        let Some(path) = Settings::config_path() else { return bindings };
        match std::fs::read_to_string(&path) {
            Ok(text) => bindings.apply_config(&text, &path.display().to_string()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => log::warn!("cannot read input bindings from {}: {e}", path.display()),
        }
        bindings
    }

    pub fn apply_config(&mut self, text: &str, origin: &str) {
        let table = match text.parse::<Table>() {
            Ok(table) => table,
            Err(e) => {
                log::warn!("ignoring input bindings in {origin}: {e}");
                return;
            }
        };
        let Some(value) = table.get("bindings") else { return };
        let Some(overrides) = value.as_table() else {
            log::warn!("{origin}: bindings must be a table");
            return;
        };
        for (name, inputs) in overrides {
            let parsed = parse_override(name, inputs);
            match parsed {
                Ok((action, replacements)) => {
                    self.0.retain(|b| b.action != action);
                    self.0.extend(replacements);
                }
                Err(error) => log::warn!("{origin}: ignoring bindings.{name}: {error}"),
            }
        }
    }

    /// Replace only this action's sources from the same device family, or append one.
    pub fn bind(&mut self, action: Action, input: &str, append: bool) -> Result<(), String> {
        let replacement = source::parse(action, input)?;
        if append && self.0.contains(&replacement) {
            return Ok(());
        }
        let family = source::family(replacement.source);
        let retained =
            self.0.iter().filter(|b| b.action == action && (append || source::family(b.source) != family)).count();
        if retained >= 32 {
            return Err("at most 32 inputs per action".into());
        }
        if !append {
            self.0.retain(|b| b.action != action || source::family(b.source) != family);
        }
        if !self.0.contains(&replacement) {
            self.0.push(replacement);
        }
        Ok(())
    }

    pub fn unbind(&mut self, action: Action, family: Option<Family>) {
        self.0.retain(|b| b.action != action || family.is_some_and(|f| source::family(b.source) != f));
    }

    pub fn reset(&mut self, action: Option<Action>, settings: &Settings) {
        let defaults = Self::with_paddles(settings.paddle_up, settings.paddle_down);
        let Some(action) = action else {
            *self = defaults;
            return;
        };
        self.0.retain(|b| b.action != action);
        self.0.extend(defaults.0.into_iter().filter(|b| b.action == action));
    }

    pub fn merge_config(&self, existing: &str) -> Result<String> {
        let mut table = existing.parse::<Table>().context("the config file is not valid TOML")?;
        let mut overrides = match table.remove("bindings") {
            Some(Value::Table(table)) => table,
            Some(_) => anyhow::bail!("bindings must be a table; existing config left unchanged"),
            None => Table::new(),
        };
        for action in Action::ALL {
            let inputs =
                self.0.iter().filter(|b| b.action == action).map(|b| Value::String(source::encode(b))).collect();
            overrides.insert(action.name().into(), Value::Array(inputs));
        }
        table.insert("bindings".into(), Value::Table(overrides));
        toml::to_string(&table).context("serializing input bindings")
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        let existing = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
        };
        let text = self.merge_config(&existing)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, text).with_context(|| format!("saving {}", path.display()))
    }
}

fn parse_override(name: &str, value: &Value) -> Result<(Action, Vec<Binding>), String> {
    let action = Action::parse(name)?;
    let inputs = value.as_array().ok_or("expected an array of input strings")?;
    if inputs.len() > 32 {
        return Err("at most 32 inputs per action".into());
    }
    let bindings = inputs
        .iter()
        .map(|input| {
            let text = input.as_str().ok_or("expected an input string")?;
            source::parse(action, text)
        })
        .collect::<Result<_, _>>()?;
    Ok((action, bindings))
}
