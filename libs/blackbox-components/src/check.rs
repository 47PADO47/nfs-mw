//! Checking a vehicle definition against a catalog, for the mod author: dangling ids, impossible defaults, rules that
//! name slots that are not there. A vehicle that passes can always be reset to stock and can always be normalized.

use std::collections::BTreeSet;

use crate::effect::{ColourSource, Effect, Op};
use crate::model::{Catalog, Condition, InputDef, Slot, VehicleDef};
use crate::selection::Selection;

impl VehicleDef {
    /// Every mistake in the definition, as a sentence naming the vehicle, the slot and the component.
    pub fn check(&self, catalog: &Catalog) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        let mut seen = BTreeSet::new();
        for (index, slot) in self.slots.iter().enumerate() {
            let at = format!("{}: slot {}", self.id, slot.id);
            if slot.id.is_empty() || !seen.insert(slot.id.as_str()) {
                errors.push(format!("{at}: the slot id is empty or repeated"));
            }
            let earlier: BTreeSet<&str> = self.slots[..index].iter().map(|s| s.id.as_str()).collect();
            self.check_slot(catalog, slot, &earlier, &at, &mut errors);
        }
        for problem in self.validate(catalog, &Selection::default()) {
            errors.push(format!("{}: the stock vehicle is not valid: {problem}", self.id));
        }
        match errors.is_empty() {
            true => Ok(()),
            false => Err(errors),
        }
    }

    fn check_slot(&self, catalog: &Catalog, slot: &Slot, earlier: &BTreeSet<&str>, at: &str, errors: &mut Vec<String>) {
        if slot.accepts.is_empty() {
            errors.push(format!("{at}: accepts no component"));
        }
        for condition in &slot.when {
            if !earlier.contains(condition.slot()) {
                errors.push(format!("{at}: a rule names {}, which is not an earlier slot", condition.slot()));
            }
            check_ids(catalog, condition, at, errors);
        }
        for def in &slot.inputs {
            if let Some(problem) = check_input(def) {
                errors.push(format!("{at}: input {}: {problem}", def.key()));
            }
        }
        match &slot.default {
            None => {}
            Some(default) => match slot.offer(default) {
                None => errors.push(format!("{at}: the default {default} is not accepted")),
                Some(offer) => {
                    let component = catalog.get(default);
                    if !offer.when.is_empty() || component.is_some_and(|c| !c.when.is_empty()) {
                        errors.push(format!("{at}: the default {default} must have no rules"));
                    }
                }
            },
        }
        let mut offered = BTreeSet::new();
        for offer in &slot.accepts {
            let at = format!("{at}, {}", offer.component);
            if !offered.insert(offer.component.as_str()) {
                errors.push(format!("{at}: accepted twice"));
            }
            let Some(component) = catalog.get(&offer.component) else {
                errors.push(format!("{at}: not in the catalog"));
                continue;
            };
            for condition in offer.when.iter().chain(&component.when) {
                self.check_condition(catalog, condition, &at, errors);
            }
            for effect in &component.effects {
                check_effect(effect, slot, &at, errors);
            }
        }
    }

    fn check_condition(&self, catalog: &Catalog, condition: &Condition, at: &str, errors: &mut Vec<String>) {
        if self.slot(condition.slot()).is_none() {
            errors.push(format!("{at}: a rule names the slot {}, which the vehicle does not have", condition.slot()));
        }
        check_ids(catalog, condition, at, errors);
    }
}

fn check_ids(catalog: &Catalog, condition: &Condition, at: &str, errors: &mut Vec<String>) {
    if let Condition::Is { any_of, .. } | Condition::IsNot { any_of, .. } = condition {
        for id in any_of.iter().filter(|id| catalog.get(id).is_none()) {
            errors.push(format!("{at}: a rule names {id}, which is not in the catalog"));
        }
    }
}

fn check_input(def: &InputDef) -> Option<String> {
    match def {
        InputDef::Num { default, min, max, .. } if !(min.is_finite() && max.is_finite() && min <= max) => {
            Some(format!("the range {min} to {max} is not valid (default {default})"))
        }
        InputDef::Text { default, max_len, .. } if default.chars().count() > *max_len => {
            Some("the default is longer than max_len".into())
        }
        _ => def.refuse(&def.default_input()),
    }
}

fn check_effect(effect: &Effect, slot: &Slot, at: &str, errors: &mut Vec<String>) {
    match effect {
        Effect::Param { key, op, value } => {
            if !value.is_finite() {
                errors.push(format!("{at}: {key} is not a finite number"));
            }
            let number = matches!(value, crate::Value::Num(_));
            if *op != Op::Set && !number {
                errors.push(format!("{at}: {key}: add, mul, min and max take a number"));
            }
        }
        Effect::Surface { colours, .. } => {
            for source in colours.values() {
                let ColourSource::Input(key) = source else { continue };
                let rgb = slot.inputs.iter().any(|def| def.key() == key && matches!(def, InputDef::Rgb { .. }));
                if !rgb {
                    errors.push(format!("{at}: a colour uses the input {key}, which the slot does not have as rgb"));
                }
            }
        }
        Effect::Part { .. } | Effect::Behaviour { .. } => {}
    }
}
