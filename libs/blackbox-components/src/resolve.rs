//! Turning a vehicle and a selection into what the game builds: the parts to draw, the surfaces, the parameter
//! modifiers and the behaviours. Pure and deterministic: slots are folded in the vehicle's order, components in the
//! order of their effects, and every map is ordered.

use std::collections::{BTreeMap, BTreeSet};

use crate::effect::{ColourSource, Effect, Input, Op, PartId, Value};
use crate::model::{Catalog, ComponentId, InputDef, SlotId, VehicleDef};
use crate::selection::Selection;
use crate::state::State;
use crate::validate::Problem;

/// The component a slot holds once resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct Installed {
    pub component: ComponentId,
    pub tier: u32,
}

/// How a surface looks after every `surface` effect.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SurfaceState {
    pub texture: Option<String>,
    pub finish: Option<String>,
    pub colours: BTreeMap<String, [u8; 3]>,
}

/// Everything the components do to one parameter key. The game applies it to its own base value with
/// [`ParamTable::apply`]; the library never sees the base.
#[derive(Debug, Clone, PartialEq)]
pub struct Modifier {
    /// The last `set`, in slot order.
    pub set: Option<Value>,
    pub add: f64,
    pub mul: f64,
    pub min: Option<f64>,
    pub max: Option<f64>,
    /// The slot that made the last `set`.
    set_by: Option<SlotId>,
}

impl Default for Modifier {
    fn default() -> Self {
        Self { set: None, add: 0.0, mul: 1.0, min: None, max: None, set_by: None }
    }
}

impl Modifier {
    fn limit(&self, value: f64) -> f64 {
        let value = self.max.map_or(value, |floor| value.max(floor));
        self.min.map_or(value, |cap| value.min(cap))
    }
}

/// The parameter modifiers of a resolved vehicle, by key.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParamTable {
    pub(crate) keys: BTreeMap<String, Modifier>,
}

impl ParamTable {
    pub fn get(&self, key: &str) -> Option<&Modifier> {
        self.keys.get(key)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Modifier)> {
        self.keys.iter()
    }

    /// `base` after the components: `set` replaces it (when a number), `add` is added, `mul` multiplies, then the
    /// floor (`max`) and the cap (`min`). A key nothing touches returns `base`.
    pub fn apply(&self, key: &str, base: f64) -> f64 {
        let Some(modifier) = self.keys.get(key) else { return base };
        let start = match &modifier.set {
            Some(Value::Num(n)) => *n,
            _ => base,
        };
        modifier.limit((start + modifier.add) * modifier.mul)
    }

    /// The same for every element of a list (a torque curve, gear ratios). A `set` list replaces `base`.
    pub fn apply_list(&self, key: &str, base: &[f64]) -> Vec<f64> {
        let start: &[f64] = match self.keys.get(key).and_then(|m| m.set.as_ref()) {
            Some(Value::List(list)) => list,
            _ => base,
        };
        let Some(modifier) = self.keys.get(key) else { return start.to_vec() };
        start.iter().map(|n| modifier.limit((n + modifier.add) * modifier.mul)).collect()
    }

    /// The value a key was set to, if any (a flag, a text, a list or a number).
    pub fn set_value(&self, key: &str) -> Option<&Value> {
        self.keys.get(key)?.set.as_ref()
    }

    /// A flag set on; false when nothing set it.
    pub fn flag(&self, key: &str) -> bool {
        matches!(self.set_value(key), Some(Value::Flag(true)))
    }

    fn push(&mut self, key: &str, op: Op, value: &Value, slot: &str, notes: &mut Vec<Note>) {
        let modifier = self.keys.entry(key.into()).or_default();
        match (op, value) {
            (Op::Set, value) => {
                if let (Some(old), Some(other)) = (&modifier.set, &modifier.set_by)
                    && old != value
                {
                    notes.push(Note::SetConflict { key: key.into(), slots: vec![other.clone(), slot.into()] });
                }
                modifier.set = Some(value.clone());
                modifier.set_by = Some(slot.into());
            }
            (Op::Add, Value::Num(n)) => modifier.add += n,
            (Op::Mul, Value::Num(n)) => modifier.mul *= n,
            (Op::Min, Value::Num(n)) => modifier.min = Some(modifier.min.map_or(*n, |m| m.min(*n))),
            (Op::Max, Value::Num(n)) => modifier.max = Some(modifier.max.map_or(*n, |m| m.max(*n))),
            _ => {}
        }
    }
}

/// Something the resolution settled by a rule rather than by the data, which a mod author may want to know.
#[derive(Debug, Clone, PartialEq)]
pub enum Note {
    /// Two components set one key to different values; the later slot won.
    SetConflict { key: String, slots: Vec<SlotId> },
    /// One component shows a part another hides; hidden wins.
    PartHidden { part: PartId, hidden_by: SlotId },
    /// Two components pick different behaviours of one domain; the later slot won.
    BehaviourConflict { domain: String, slots: Vec<SlotId> },
}

/// A vehicle with its components installed.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResolvedVehicle {
    pub vehicle: String,
    pub installed: BTreeMap<SlotId, Installed>,
    /// The model parts to draw.
    pub parts: BTreeSet<PartId>,
    pub surfaces: BTreeMap<String, SurfaceState>,
    pub params: ParamTable,
    /// The behaviour picked in each domain.
    pub behaviours: BTreeMap<String, String>,
    pub notes: Vec<Note>,
}

impl VehicleDef {
    /// The vehicle with the selection installed; the problems when the selection is not valid ([`VehicleDef::validate`]).
    pub fn resolve(&self, catalog: &Catalog, selection: &Selection) -> Result<ResolvedVehicle, Vec<Problem>> {
        let problems = self.validate(catalog, selection);
        if !problems.is_empty() {
            return Err(problems);
        }
        let state = State::new(self, catalog, selection);
        let mut out = ResolvedVehicle { vehicle: self.id.clone(), ..ResolvedVehicle::default() };
        let (mut shown, mut hidden): (BTreeSet<PartId>, BTreeMap<PartId, SlotId>) = Default::default();
        let mut selectors: BTreeMap<String, SlotId> = BTreeMap::new();
        for item in state.items.iter().filter(|item| item.active) {
            let Some(component) = item.pick.component.as_deref().and_then(|id| catalog.get(id)) else { continue };
            let slot = item.slot.id.as_str();
            out.installed.insert(slot.into(), Installed { component: component.id.clone(), tier: component.tier });
            for effect in &component.effects {
                match effect {
                    Effect::Part { show, hide } => {
                        shown.extend(show.iter().cloned());
                        hidden.extend(hide.iter().map(|part| (part.clone(), slot.to_string())));
                    }
                    Effect::Surface { target, texture, finish, colours } => {
                        let surface = out.surfaces.entry(target.clone()).or_default();
                        surface.texture = texture.clone().or(surface.texture.take());
                        surface.finish = finish.clone().or(surface.finish.take());
                        for (channel, source) in colours {
                            let rgb = colour(source, &item.pick.inputs, &item.slot.inputs);
                            surface.colours.insert(channel.clone(), rgb);
                        }
                    }
                    Effect::Param { key, op, value } => out.params.push(key, *op, value, slot, &mut out.notes),
                    Effect::Behaviour { domain, select } => {
                        let old = out.behaviours.insert(domain.clone(), select.clone());
                        let other = selectors.insert(domain.clone(), slot.to_string());
                        if let (Some(old), Some(other)) = (old, other)
                            && old != *select
                        {
                            let slots = vec![other, slot.into()];
                            out.notes.push(Note::BehaviourConflict { domain: domain.clone(), slots });
                        }
                    }
                }
            }
        }
        for (part, slot) in &hidden {
            if shown.contains(part) {
                out.notes.push(Note::PartHidden { part: part.clone(), hidden_by: slot.clone() });
            }
        }
        out.parts = self.parts.iter().chain(&shown).filter(|part| !hidden.contains_key(*part)).cloned().collect();
        Ok(out)
    }
}

/// An `Rgb` colour: fixed, or the pick's input, or the input's default. Validated definitions make the last cases total.
fn colour(source: &ColourSource, given: &BTreeMap<String, Input>, defs: &[InputDef]) -> [u8; 3] {
    let key = match source {
        ColourSource::Fixed(rgb) => return *rgb,
        ColourSource::Input(key) => key,
    };
    let input =
        given.get(key).cloned().or_else(|| defs.iter().find(|def| def.key() == key).map(InputDef::default_input));
    match input {
        Some(Input::Rgb(rgb)) => rgb,
        _ => [0, 0, 0],
    }
}
