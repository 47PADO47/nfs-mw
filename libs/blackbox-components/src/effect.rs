//! What a component does: the values it carries and the four kinds of effect. Nothing here names a game: a part, a
//! surface, a parameter key or a behaviour domain is an opaque string that the game maps to its own things.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A model part of a vehicle (a solid, a mesh group): the vehicle's `parts` lists the ones it is built with.
pub type PartId = String;

/// A parameter value. A number is what the modifier operations act on; the others can only be set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Flag(bool),
    Num(f64),
    Text(String),
    List(Vec<f64>),
}

impl Value {
    pub(crate) fn is_finite(&self) -> bool {
        match self {
            Self::Num(n) => n.is_finite(),
            Self::List(list) => list.iter().all(|n| n.is_finite()),
            Self::Flag(_) | Self::Text(_) => true,
        }
    }
}

/// A value the player gives a slot when picking a component (a colour, an amount, a text).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Input {
    Rgb([u8; 3]),
    Num(f64),
    Text(String),
}

/// Where an effect takes a colour from: a fixed sRGB colour, or (a string) the name of an `Rgb` input of the slot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ColourSource {
    Fixed([u8; 3]),
    Input(String),
}

/// How a `param` effect changes its key. Composition order is `set`, `add`, `mul`, then `min` and `max` ([`crate::ParamTable`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Op {
    /// Replace the base value.
    Set,
    /// Add to it (summed over components).
    Add,
    /// Multiply it (multiplied over components).
    Mul,
    /// An upper limit (the lowest limit wins).
    Min,
    /// A lower limit (the highest limit wins).
    Max,
}

/// One effect of a component. A component lists any number of them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Effect {
    /// Show and hide model parts. A swap is one effect that does both. A part hidden by any component is hidden.
    Part {
        #[serde(default)]
        show: Vec<PartId>,
        #[serde(default)]
        hide: Vec<PartId>,
    },
    /// Change how a surface (a material, a paint area) looks. Each field it gives replaces the field set before it.
    Surface {
        target: String,
        #[serde(default)]
        texture: Option<String>,
        #[serde(default)]
        finish: Option<String>,
        /// Named colour channels (`base`, `stripe`...).
        #[serde(default)]
        colours: BTreeMap<String, ColourSource>,
    },
    /// Change a number the game reads (mass, power, grip...) or set a flag, a text or a list.
    Param { key: String, op: Op, value: Value },
    /// Pick one of the named behaviours of a domain (`tyre.compound = "slick"`); the game owns what each name does.
    Behaviour { domain: String, select: String },
}
