//! Editing the choices of a vehicle: what a change is, and the checks that refuse a choice the vehicle does not accept.

use nfsmw_data::car::customization::{Catalog, Category, Selection};
use nfsmw_data::car::mods::components::{self, select};
use nfsmw_data::car::physics::{PerformanceCategory, PerformancePackage};

use super::{Choice, Slot, Store, Value, VehicleOptions};

/// The part of the garage's selection that holds the choices the options edit.
pub(crate) struct Edit<'a> {
    pub parts: &'a mut Selection,
    pub color: &'a mut Option<[u8; 3]>,
    pub performance: &'a mut PerformancePackage,
}

/// One edit, as the panel and the console make it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Change {
    /// `vehicle set <slot> <choice>`: the choice by number, id, `stock`, `none` or a part of its name.
    Set { slot: String, choice: String },
    /// The panel's pick: a position in the slot's choices.
    Pick { slot: String, index: usize },
    /// Every slot back to stock, and the body paint to the vehicle's own.
    Reset,
    /// The slots of one group back to stock.
    ResetGroup(String),
    /// A group of optional slots on (what the vehicle has) or off (emptied).
    Group { name: String, on: bool },
    /// The body colour (`None`: the vehicle's own paint).
    Color(Option<[u8; 3]>),
    /// A quick finish of the paint (the hash the options list): the colour stays, the finish changes.
    Finish(u32),
    /// A performance category's Junkman package on or off.
    Junkman { slot: String, on: bool },
    /// The performance of the whole vehicle at once.
    Shortcut(Shortcut),
}

/// The performance shortcuts of the garage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Shortcut {
    /// Every category at its original equipment, no Junkman.
    Stock,
    /// Every category at its last stage the vehicle has.
    Maximum,
    /// The last stages, and every Junkman package the vehicle allows.
    MaximumJunkman,
}

impl VehicleOptions {
    /// Applies `change` to `edit`. The vehicle's rules are checked on the result: when they do not hold nothing changes and the
    /// reason is the error. `catalog` is the vehicle's part catalogue, which has its own rules (rim paint needs aftermarket rims).
    pub(crate) fn apply(
        &self,
        catalog: Option<&Catalog>,
        edit: &mut Edit<'_>,
        change: &Change,
    ) -> Result<String, String> {
        let mut next = (edit.parts.clone(), *edit.color, *edit.performance);
        let message = {
            let mut scratch = Edit { parts: &mut next.0, color: &mut next.1, performance: &mut next.2 };
            self.edit(catalog, &mut scratch, change)?
        };
        next.2 = self.without_unavailable_junkman(next.2);
        self.check(catalog, &next.0, next.2)?;
        *edit.parts = next.0;
        *edit.color = next.1;
        *edit.performance = next.2;
        Ok(message)
    }

    fn edit(&self, catalog: Option<&Catalog>, edit: &mut Edit<'_>, change: &Change) -> Result<String, String> {
        match change {
            Change::Set { slot, choice } => {
                let slot = self.named(slot)?;
                let index = find_choice(slot, choice)?;
                self.put(catalog, edit, slot, index)
            }
            Change::Pick { slot, index } => {
                let slot = self.named(slot)?;
                self.put(catalog, edit, slot, *index)
            }
            Change::Reset => {
                for slot in &self.slots {
                    self.put(catalog, edit, slot, 0)?;
                }
                *edit.color = None;
                edit.performance.junkman = 0;
                Ok(format!("{} is stock again", self.name))
            }
            Change::ResetGroup(name) => {
                let slots: Vec<&Slot> = self.in_group(name).collect();
                if slots.is_empty() {
                    return Err(self.unknown_group(name));
                }
                for slot in slots {
                    self.put(catalog, edit, slot, 0)?;
                }
                Ok(format!("{name}: stock"))
            }
            Change::Group { name, on } => self.switch_group(catalog, edit, name, *on),
            Change::Finish(hash) => self.finish(catalog, edit, *hash),
            Change::Junkman { slot, on } => self.junkman(edit, self.named(slot)?, *on),
            Change::Shortcut(shortcut) => self.shortcut(edit, *shortcut),
            Change::Color(color) => {
                if self.paintwork.is_none() {
                    return Err(format!("{} has a fixed livery", self.name));
                }
                *edit.color = *color;
                Ok(match color {
                    Some([r, g, b]) => format!("body colour #{r:02X}{g:02X}{b:02X}"),
                    None => "body colour: the vehicle's own".to_owned(),
                })
            }
        }
    }

    /// A finish keeps the colour the body has now (its own, or the preset's) and changes how it is drawn.
    fn finish(&self, catalog: Option<&Catalog>, edit: &mut Edit<'_>, hash: u32) -> Result<String, String> {
        let finish =
            self.finishes.iter().find(|finish| finish.hash == hash).ok_or("this vehicle has no such finish")?;
        let preset = edit.parts.parts[Category::Paint as usize]
            .and_then(|held| catalog?.colors.get(&held).copied())
            .or(self.paintwork.and_then(|paint| paint.original));
        *edit.color = edit.color.or(preset);
        edit.parts.parts[Category::Paint as usize] = Some(hash);
        Ok(format!("paint finish: {}", finish.label))
    }

    fn junkman(&self, edit: &mut Edit<'_>, slot: &Slot, on: bool) -> Result<String, String> {
        let (Store::Level(category), Some(junkman)) = (slot.store, slot.junkman) else {
            return Err(format!("{} has no Junkman package", slot.label));
        };
        let bit = 1 << category as u8;
        if !on {
            edit.performance.junkman &= !bit;
            return Ok(format!("{}: Junkman off", slot.label));
        }
        let allowed = self.levels.as_ref().is_some_and(|levels| levels.junkman_available(category, *edit.performance));
        if !junkman.available && !allowed {
            return Err(format!("the Junkman package of {} is not available for this vehicle", slot.label));
        }
        edit.performance.junkman |= bit;
        Ok(format!("{}: Junkman installed", slot.label))
    }

    fn shortcut(&self, edit: &mut Edit<'_>, shortcut: Shortcut) -> Result<String, String> {
        let levels = self.levels.as_ref().ok_or("this vehicle has no upgrade catalogue here")?;
        let mut package = match shortcut {
            Shortcut::Stock => PerformancePackage::default(),
            Shortcut::Maximum | Shortcut::MaximumJunkman => levels.maximum(),
        };
        if shortcut == Shortcut::MaximumJunkman {
            for category in PerformanceCategory::ALL {
                if levels.junkman_available(category, package) {
                    package.junkman |= 1 << category as u8;
                }
            }
        }
        *edit.performance = package;
        Ok(match shortcut {
            Shortcut::Stock => "performance: original equipment".to_owned(),
            Shortcut::Maximum => "performance: maximum".to_owned(),
            Shortcut::MaximumJunkman => "performance: maximum with every Junkman package".to_owned(),
        })
    }

    /// A Junkman package whose category no longer allows it (the stage was taken off) goes with it.
    fn without_unavailable_junkman(&self, mut package: PerformancePackage) -> PerformancePackage {
        let Some(levels) = &self.levels else { return package };
        for category in PerformanceCategory::ALL {
            if package.has_junkman(category) && !levels.junkman_available(category, package) {
                package.junkman &= !(1 << category as u8);
            }
        }
        package
    }

    fn named(&self, word: &str) -> Result<&Slot, String> {
        self.slot(word).ok_or_else(|| {
            let known: Vec<&str> = self.slots.iter().map(|slot| slot.id.as_str()).collect();
            format!("{} has no slot {word:?} (it has: {})", self.name, known.join(", "))
        })
    }

    /// Puts the choice at `index` of `slot` in `edit`. A choice a rule of the vehicle blocks is refused.
    pub(super) fn put(
        &self,
        catalog: Option<&Catalog>,
        edit: &mut Edit<'_>,
        slot: &Slot,
        index: usize,
    ) -> Result<String, String> {
        let choice = slot
            .choices
            .get(index)
            .ok_or_else(|| format!("{} has {} choices, not {index}", slot.label, slot.choices.len() - 1))?;
        if let Some(reason) = &choice.blocked {
            return Err(format!("{} cannot be fitted: {reason}", choice.label));
        }
        write(slot, choice, edit);
        if slot.store == Store::Category(Category::Wheels)
            && let Some(catalog) = catalog
        {
            clear_ineligible_rim_paint(catalog, edit);
        }
        Ok(format!("{}: {}", slot.label, choice.label))
    }

    /// Whether the vehicle accepts the selection: its catalogue's rules and its components' rules.
    fn check(
        &self,
        catalog: Option<&Catalog>,
        parts: &Selection,
        performance: PerformancePackage,
    ) -> Result<(), String> {
        if let Some(catalog) = catalog {
            catalog.validate(parts).map_err(|error| format!("{error:#}"))?;
        }
        let Some(own) = components::of(&self.folder) else { return Ok(()) };
        let picks = select::picks(&self.folder, own, parts, performance);
        own.vehicle.resolve(&own.catalog, &picks).map(drop).map_err(|problems| {
            let lines: Vec<String> = problems.iter().map(ToString::to_string).collect();
            lines.join("; ")
        })
    }
}

fn write(slot: &Slot, choice: &Choice, edit: &mut Edit<'_>) {
    match (slot.store, &choice.value) {
        (Store::Category(category), Value::Hash(hash)) => edit.parts.parts[category as usize] = Some(*hash),
        (Store::Category(category), _) => edit.parts.parts[category as usize] = None,
        (Store::Level(category), Value::Level(level)) => edit.performance.levels[category as usize] = *level,
        (Store::Level(category), _) => edit.performance.levels[category as usize] = 0,
        (Store::Session, Value::Component(id)) => {
            edit.parts.slots.insert(slot.id.clone(), id.clone());
        }
        (Store::Session, Value::Empty) if slot.choices[0].value == Value::Stock => {
            // The slot has a default component: emptying it is a pick of its own.
            edit.parts.slots.insert(slot.id.clone(), String::new());
        }
        (Store::Session, _) => {
            edit.parts.slots.remove(&slot.id);
        }
    }
}

/// Rim paint needs aftermarket rims (or a mod car whose paint works on its own): fitting other rims takes it off.
fn clear_ineligible_rim_paint(catalog: &Catalog, edit: &mut Edit<'_>) {
    if catalog.custom_rim_paint {
        return;
    }
    if edit.parts.parts[Category::Wheels as usize].is_some_and(|hash| catalog.paintable_wheels.contains(&hash)) {
        return;
    }
    edit.parts.parts[Category::RimPaint as usize] = None;
    edit.parts.rim_color = None;
}

/// The position of the choice `word` names in `slot`: a number, an id, `stock`, `none`, or a part of the name.
pub(super) fn find_choice(slot: &Slot, word: &str) -> Result<usize, String> {
    let lower = word.trim().to_ascii_lowercase();
    if matches!(lower.as_str(), "stock" | "default" | "0") {
        return Ok(0);
    }
    if matches!(lower.as_str(), "none" | "off" | "empty") {
        return slot.empty_choice().ok_or_else(|| format!("{} cannot be emptied", slot.label));
    }
    if let Some(at) = slot.choices.iter().position(|choice| choice.id.to_ascii_lowercase() == lower) {
        return Ok(at);
    }
    if let Ok(number) = lower.parse::<usize>() {
        return match number < slot.choices.len() {
            true => Ok(number),
            false => Err(format!("{} has choices 0 to {}", slot.label, slot.choices.len() - 1)),
        };
    }
    let found: Vec<usize> =
        (1..slot.choices.len()).filter(|at| slot.choices[*at].label.to_ascii_lowercase().contains(&lower)).collect();
    match found.as_slice() {
        [one] => Ok(*one),
        [] => Err(format!("{} has no choice {word:?} (vehicle slots lists them)", slot.label)),
        many => {
            let shown: Vec<String> =
                many.iter().take(6).map(|at| format!("{at}: {}", slot.choices[*at].label)).collect();
            Err(format!("{word:?} fits several choices of {}: {}", slot.label, shown.join(", ")))
        }
    }
}
