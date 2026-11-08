//! Reading a vehicle's options from its data.

use std::collections::HashMap;

use blackbox_components::{Slot as ComponentSlot, SlotView};
use nfsmw_data::car::customization::{Catalog, Category, Selection};
use nfsmw_data::car::mods::bike;
use nfsmw_data::car::mods::components::{self, Components, select};
use nfsmw_data::car::physics::{PerformanceCatalog, PerformanceCategory, PerformancePackage};

use super::names::{LEVEL_ORDER, humanize, level_key};
use super::{Choice, Paintwork, Slot, Store, Value, VehicleOptions};

/// The garage's data about the vehicle on show: what the options are read from.
pub(crate) struct Inputs<'a> {
    pub folder: &'a str,
    pub name: &'a str,
    /// The part catalogue of the vehicle (`None` while it is not read: no category is listed).
    pub catalog: Option<&'a Catalog>,
    /// What is installed: the model choices.
    pub parts: &'a Selection,
    pub color: Option<[u8; 3]>,
    pub paintable: bool,
    pub original_color: Option<[u8; 3]>,
    /// The upgrade levels the vehicle has and what is installed (`None` where the scene has no physics, the car viewer).
    pub performance: Option<(&'a PerformanceCatalog, PerformancePackage)>,
}

/// What the component slots say about each of their options now, by slot id.
type Views = HashMap<String, SlotView>;

impl VehicleOptions {
    /// The options of the vehicle in `inputs`.
    pub(crate) fn build(inputs: &Inputs<'_>) -> Self {
        let own = components::of(inputs.folder);
        let package = inputs.performance.map(|(_, package)| package).unwrap_or_default();
        let views = views(own, inputs.folder, inputs.parts, package);
        let mut slots = Vec::new();
        let mut covered_categories = Vec::new();
        let mut covered_levels = Vec::new();
        for (components, slot) in own.into_iter().flat_map(|c| c.vehicle.slots.iter().map(move |slot| (c, slot))) {
            let view = views.get(&slot.id);
            if let Some(category) = select::category_of(&slot.id) {
                covered_categories.push(category);
                let Some(catalog) = inputs.catalog else { continue };
                slots.extend(category_slot(inputs, catalog, category, Some((components, slot, view))));
                continue;
            }
            if let Some(category) = select::performance_of(&slot.id) {
                covered_levels.push(category);
                let Some((levels, package)) = inputs.performance else { continue };
                slots.extend(level_slot(levels, package, category, Some((components, slot))));
                continue;
            }
            slots.push(session_slot(inputs.parts, components, slot, view));
        }
        if let Some(catalog) = inputs.catalog {
            for category in Category::ALL.into_iter().filter(|category| !covered_categories.contains(category)) {
                slots.extend(category_slot(inputs, catalog, category, None));
            }
        }
        if let Some((levels, package)) = inputs.performance {
            for category in LEVEL_ORDER.into_iter().filter(|category| !covered_levels.contains(category)) {
                slots.extend(level_slot(levels, package, category, None));
            }
        }
        let bike = bike::is_bike(inputs.folder);
        Self {
            folder: inputs.folder.to_owned(),
            name: inputs.name.to_owned(),
            bike,
            missing: missing(bike, &slots),
            slots,
            paintwork: inputs.paintable.then_some(Paintwork { custom: inputs.color, original: inputs.original_color }),
        }
    }
}

/// The options of every component slot of the car, with the rules checked against what is installed.
fn views(components: Option<&Components>, folder: &str, parts: &Selection, package: PerformancePackage) -> Views {
    let Some(components) = components else { return Views::new() };
    let picks = select::picks(folder, components, parts, package);
    let views = components.vehicle.slot_views(&components.catalog, &picks, &());
    views.into_iter().map(|view| (view.slot.clone(), view)).collect()
}

/// A line of what the vehicle's data lacks that its owner might look for here.
fn missing(bike: bool, slots: &[Slot]) -> Vec<String> {
    let mentions = |word: &str| slots.iter().any(|slot| slot.id.contains(word) || slot.group.contains(word));
    let mut lines = Vec::new();
    if bike && !mentions("road") {
        lines.push(
            "road equipment (side mirrors, homologation plate holder, lights): this bike's data has no slot for it. \
             The exporter writes the mirrors and the plate holder as parts, but the importer keeps them in the body solid. \
             Needed: the importer gives each a solid of its own and writes an optional slot, group `road`, whose default \
             component shows them; then `vehicle group road off` takes them off (docs/devtools-vehicle.md)"
                .to_owned(),
        );
    }
    lines
}

/// The price a component asks, when the data gives one.
fn price(offer_price: Option<u32>, component_price: u32) -> Option<u32> {
    Some(offer_price.unwrap_or(component_price)).filter(|price| *price > 0)
}

/// Why the option of `component` in the slot cannot be picked with the rest of the selection.
fn blocked(view: Option<&SlotView>, component: &str) -> Option<String> {
    let option = view?.options.iter().find(|option| option.component == component)?;
    let blockers = match &option.state {
        blackbox_components::OptionState::Blocked(conditions) => conditions,
        blackbox_components::OptionState::Locked { unlock } => return Some(format!("needs {unlock}")),
        _ => return None,
    };
    let mut slots: Vec<&str> = blockers.iter().map(|condition| condition.slot()).collect();
    slots.dedup();
    Some(format!("needs another {}", slots.join(", ")))
}

/// A choice of a component slot by the component's id, as its data gives it.
fn component_choice(
    components: &Components,
    slot: &ComponentSlot,
    view: Option<&SlotView>,
    id: &str,
    value: Value,
) -> Option<Choice> {
    let component = components.catalog.get(id)?;
    let offer = slot.offer(id)?;
    Some(Choice {
        id: component.id.clone(),
        label: component.display_name().to_owned(),
        price: price(offer.price, component.price),
        tier: component.tier,
        blocked: blocked(view, id),
        value,
    })
}

fn stock_choice() -> Choice {
    Choice { id: "stock".into(), label: "Stock".into(), price: None, tier: 0, blocked: None, value: Value::Stock }
}

fn category_group(category: Category) -> &'static str {
    match category {
        Category::Body | Category::Spoiler | Category::Hood | Category::Roof | Category::Wheels => "parts",
        Category::Paint | Category::WindowTint | Category::RimPaint | Category::Vinyl => "appearance",
    }
}

fn category_label(category: Category) -> &'static str {
    match category {
        Category::Paint => "Paint",
        Category::Vinyl => "Vinyl / livery",
        Category::WindowTint => "Window tint",
        Category::RimPaint => "Rim paint",
        other => other.label(),
    }
}

/// The slot of a model category: its choices are the catalogue's, named by the component slot that maps onto it when there is one.
fn category_slot(
    inputs: &Inputs<'_>,
    catalog: &Catalog,
    category: Category,
    component: Option<(&Components, &ComponentSlot, Option<&SlotView>)>,
) -> Option<Slot> {
    let listed = &catalog.choices[category as usize];
    if listed.is_empty() {
        return None;
    }
    let held = inputs.parts.parts[category as usize];
    let mut choices = vec![stock_choice()];
    let mut current = 0;
    for choice in listed {
        let named = component.and_then(|(components, slot, view)| {
            let offer = slot
                .accepts
                .iter()
                .find(|offer| select::choice_hash(inputs.folder, &offer.component) == choice.hash)?;
            component_choice(components, slot, view, &offer.component, Value::Hash(choice.hash))
        });
        let option = named.unwrap_or_else(|| Choice {
            id: format!("{:08X}", choice.hash),
            label: choice.label.clone(),
            price: None,
            tier: 0,
            blocked: None,
            value: Value::Hash(choice.hash),
        });
        if held == Some(choice.hash) {
            current = choices.len();
        }
        choices.push(option);
    }
    if let Some((components, slot, view)) = component
        && let Some(default) = &slot.default
        && let Some(stock) = component_choice(components, slot, view, default, Value::Stock)
    {
        choices[0] = stock;
    }
    let (id, label, group) = match component {
        Some((_, slot, _)) => (slot.id.clone(), humanize(&slot.id), group_of(slot, category_group(category))),
        None => (category.key().to_owned(), category_label(category).to_owned(), category_group(category).to_owned()),
    };
    Some(Slot { id, label, group, components: component.is_some(), store: Store::Category(category), current, choices })
}

fn group_of(slot: &ComponentSlot, fallback: &str) -> String {
    match slot.group.is_empty() {
        true => fallback.to_owned(),
        false => slot.group.clone(),
    }
}

/// The slot of a performance category: a level for each stage the vehicle's upgrade catalogue has.
fn level_slot(
    levels: &PerformanceCatalog,
    package: PerformancePackage,
    category: PerformanceCategory,
    component: Option<(&Components, &ComponentSlot)>,
) -> Option<Slot> {
    let max = levels.categories[category as usize].max_level;
    if max == 0 {
        return None;
    }
    let mut choices = vec![Choice { label: "Original equipment".into(), ..stock_choice() }];
    for level in 1..=max {
        let ladder = component.and_then(|(components, slot)| {
            let pick = select::ladder_pick(slot, &components.catalog, level)?;
            component_choice(components, slot, None, pick.component.as_deref()?, Value::Level(level))
        });
        choices.push(ladder.unwrap_or_else(|| Choice {
            id: level.to_string(),
            label: format!("Stage {level}"),
            price: None,
            tier: u32::from(level),
            blocked: None,
            value: Value::Level(level),
        }));
    }
    let (id, label, group) = match component {
        Some((_, slot)) => (slot.id.clone(), humanize(&slot.id), group_of(slot, "performance")),
        None => (level_key(category).to_owned(), category.label().to_owned(), "performance".to_owned()),
    };
    let current = usize::from(package.get(category).min(max));
    Some(Slot { id, label, group, components: component.is_some(), store: Store::Level(category), current, choices })
}

/// The slot of a component slot the garage has no category for: its choices are the slot's accepted components.
fn session_slot(parts: &Selection, components: &Components, slot: &ComponentSlot, view: Option<&SlotView>) -> Slot {
    let default = slot.default.as_deref();
    let mut choices = Vec::new();
    let nothing =
        Choice { id: "none".into(), label: "None".into(), price: None, tier: 0, blocked: None, value: Value::Empty };
    match default {
        Some(id) => choices.extend(component_choice(components, slot, view, id, Value::Stock)),
        None => choices.push(nothing.clone()),
    }
    if choices.is_empty() {
        choices.push(stock_choice());
    }
    for offer in slot.accepts.iter().filter(|offer| Some(offer.component.as_str()) != default) {
        let value = Value::Component(offer.component.clone());
        choices.extend(component_choice(components, slot, view, &offer.component, value));
    }
    if slot.optional && default.is_some() {
        choices.push(nothing);
    }
    let current = match parts.slots.get(&slot.id) {
        None => 0,
        Some(id) if id.is_empty() => choices.iter().position(|choice| choice.value == Value::Empty).unwrap_or(0),
        Some(id) => choices.iter().position(|choice| choice.value == Value::Component(id.clone())).unwrap_or(0),
    };
    Slot {
        id: slot.id.clone(),
        label: humanize(&slot.id),
        group: group_of(slot, "parts"),
        components: true,
        store: Store::Session,
        current,
        choices,
    }
}
