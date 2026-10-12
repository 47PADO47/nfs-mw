//! The light bars of a cop car: the particle groups its `ecar` record links (`fxcar_coplight*`, one per colour) and
//! the `COPLIGHT*` markers of the model where each colour's emitters run. The emitters' on and off cycles give the
//! flashing (`emitterdata.h` in the decompilation, read by `exhaust::emitter_spec`).

use std::collections::HashMap;

use blackbox_attrib::{Database, Value};
use blackbox_hash::bstring_hash;
use blackbox_solid::Solid;
use glam::Mat4;

use super::CarModel;
use super::assemble::Placement;
use super::exhaust::{GroupEmitter, group_emitters};

/// A light bar's colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Colour {
    Red,
    Blue,
    White,
}

impl Colour {
    /// The markers of this colour: the normal lamp and the bright one.
    fn markers(self) -> [&'static str; 2] {
        match self {
            Colour::Red => ["COPLIGHTRED", "COPLIGHTBRIGHTRED"],
            Colour::Blue => ["COPLIGHTBLUE", "COPLIGHTBRIGHTBLUE"],
            Colour::White => ["COPLIGHTWHITE", "COPLIGHTWHITE"],
        }
    }
}

/// The colour a light group is for, from its name (`fxcar_coplightred`); `None` for other groups.
pub fn colour_of_group(name: &str) -> Option<Colour> {
    match name.to_ascii_lowercase().strip_prefix("fxcar_coplight")? {
        "red" => Some(Colour::Red),
        "blue" => Some(Colour::Blue),
        "white" => Some(Colour::White),
        _ => None,
    }
}

/// One colour of light bar: where its lamps are in the car model's space, and the emitters run at each of them.
#[derive(Debug, Clone, PartialEq)]
pub struct CopLane {
    pub colour: Colour,
    pub frames: Vec<Mat4>,
    pub emitters: Vec<GroupEmitter>,
}

/// The light bars of a car, when it has any.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CopLightFx {
    pub lanes: Vec<CopLane>,
}

impl CopLightFx {
    /// Read the light bars of `model` from the gameplay database (`attributes.bin`).
    pub fn read(db: &Database, model: &CarModel) -> Option<Self> {
        let key = model.car_type.as_deref().unwrap_or(&model.name).to_ascii_lowercase();
        let ecar = db.collection("ecar", &key)?;
        let mut lanes = Vec::new();
        for attribute in ecar.attributes() {
            let Value::RefSpec(reference) = &attribute.value else { continue };
            let Some(group) = db.resolve(*reference) else { continue };
            let Some(colour) = group.name().and_then(colour_of_group) else { continue };
            let emitters = group_emitters(db, group);
            let frames = light_frames(&model.placements, &model.solids, colour);
            if emitters.is_empty() || frames.is_empty() {
                continue;
            }
            lanes.push(CopLane { colour, frames, emitters });
        }
        (!lanes.is_empty()).then_some(Self { lanes })
    }
}

/// The frame of every marker of `colour` among the placed solids, in the car model's space.
fn light_frames(placements: &[Placement], solids: &HashMap<u32, Solid>, colour: Colour) -> Vec<Mat4> {
    let names = colour.markers().map(bstring_hash);
    let mut frames = Vec::new();
    for placement in placements {
        let Some(solid) = solids.get(&placement.solid) else { continue };
        for marker in solid.markers.iter().filter(|m| names.contains(&m.name_hash)) {
            // Stored row-major for row vectors, as the pipe markers are.
            frames.push(placement.transform * Mat4::from_cols_array(&marker.matrix));
        }
    }
    frames
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_light_groups_name_their_colour() {
        assert_eq!(colour_of_group("fxcar_coplightred"), Some(Colour::Red));
        assert_eq!(colour_of_group("fxcar_coplightBLUE"), Some(Colour::Blue));
        assert_eq!(colour_of_group("fxcar_coplightwhite"), Some(Colour::White));
        assert_eq!(colour_of_group("fxcar_nos"), None);
        assert_eq!(colour_of_group("fxcar_coplightgreen"), None);
    }
}
