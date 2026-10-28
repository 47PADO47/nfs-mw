//! The driver of a 2012 car. Spec: `docs/formats/chameleon-vehicle.md` ("The driver").
//!
//! Every car bundle holds a `Vehicle_DriverSetup` Genesys object: which character sits in the car (the key of a
//! `CharacterSpec` of the shared character bundle) and where its seat is. The characters are three seated figures (two
//! civilians and the police officer, which the police cars seat instead of the one their setup names, `driver_cop.rs`), a
//! skinned model at five levels of detail, modelled in the pose of their rest skeleton around the seat, so a car's driver
//! is the figure moved by the car's seat position and nothing else ([`Driver::seated`]). One bundle (the police Explorer)
//! also bakes a copy of the officer into its body; [`remove_baked_driver`] takes it out so the car does not draw two.

use chameleon_bundle::{Bundle, Pool, ResourceId, ResourceKind};

use crate::driver_cop::{COP_CHARACTER, police_flag};
use crate::error::{Error, Result};
use crate::genesys::{Object, TypeNames, find_object};
use crate::vehicle::{Surfaces, expect, model_lods};
use crate::{Lod, PartMesh, Vehicle};

/// The Genesys type of the object that sets a car's driver up.
pub(crate) const SETUP: &str = "Genesys.Gen.Vehicle_DriverSetup";
/// Where the seat (three floats), the character's key (u32) and the number of seat belt bones (u16) sit in the object.
const SEAT_AT: usize = 0x10;
const CHARACTER_AT: usize = 0x24;
const BELTS_AT: usize = 0x42;
/// Where the locator group ids of the gear lever and of the two hand grips on the wheel, and the key of the car's own grip
/// pose clip sit in the object (`docs/formats/chameleon-driver-animation.md`).
const SHIFTER_AT: usize = 0x2C;
const GRIPS_AT: usize = 0x30;
const POSE_AT: usize = 0x50;
/// A seat further than this from the car's origin, on any axis, is no seat: the one car without a driver says 19.
const MAX_SEAT: f32 = 10.0;
/// Where a `CharacterSpec` keeps the import of its model.
pub(crate) const MODEL_AT: u32 = 0x20;
/// The end of the technique name of the material of the driver baked into a body.
const BAKED_TECHNIQUE: &str = "_Driver";

/// What a car's bundle says about its driver.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DriverSetup {
    /// The key of the `CharacterSpec` of the figure that sits in the car (in the character bundle, class 1 ids): the officer
    /// ([`COP_CHARACTER`]) in a police car, else the character the setup names.
    pub character: u32,
    /// The key the setup object itself names (the same civilian character in a police car as in the others).
    pub named_character: u32,
    /// Whether the car is a police car, which seats the officer whatever the setup names.
    pub cop: bool,
    /// Where the figure's origin goes (engine space: x left, y up, z forward), metres; `None` when the car has no driver.
    pub seat: Option<[f32; 3]>,
    /// The number of seat belt bones the car sets up: 3 for a belt, 4 for a racing harness. The figure with the harness
    /// goes with the 4.
    pub belts: u16,
    /// The id of the locator group (in the car's spec) of the gear lever the driver's hand goes to; 0 for a car with none.
    pub shifter_group: u32,
    /// The id of the locator group of the two hand grips on the steering wheel (left and right hand); 0 for a car with none.
    pub grip_group: u32,
    /// The low word of the id of the clip of the character's animation list that poses the hands for this car's wheel.
    pub pose_clip: u32,
}

impl DriverSetup {
    pub(crate) fn of(object: &Object) -> Result<Self> {
        let base = object.at(SETUP, 0).ok_or_else(|| Error::Spec(format!("the object has no {SETUP}")))?;
        let seat = object.floats::<3>(base + SEAT_AT)?;
        let named_character = object.u32(base + CHARACTER_AT)?;
        Ok(Self {
            character: named_character,
            named_character,
            cop: false,
            seat: seat.iter().all(|c| c.abs() < MAX_SEAT).then_some(seat),
            belts: object.u16(base + BELTS_AT)?,
            shifter_group: object.u32(base + SHIFTER_AT)?,
            grip_group: object.u32(base + GRIPS_AT)?,
            pose_clip: object.u32(base + POSE_AT)?,
        })
    }

    /// The setup of a car that is (or is not) a police car: the officer sits in it when it is.
    pub(crate) fn policed(mut self, cop: bool) -> Self {
        self.cop = cop;
        if cop {
            self.character = COP_CHARACTER;
        }
        self
    }
}

/// The driver setup of a car bundle, with the character the game seats in it (the officer for a police car); `None` when the
/// bundle has none that reads.
pub fn driver_setup(bundle: &Bundle) -> Option<DriverSetup> {
    let types = TypeNames::read(bundle).ok()?;
    let object = find_object(bundle, &types, SETUP).ok()?;
    let setup = DriverSetup::of(&object).ok()?;
    Some(setup.policed(police_flag(bundle, &types).unwrap_or(false)))
}

/// A driver figure at every level of detail, around its own origin (the seat), with the textures its meshes draw named in
/// their appearances.
#[derive(Debug, Clone, PartialEq)]
pub struct Driver {
    pub character: u32,
    /// Level 0 is the most detailed.
    pub lods: Vec<Lod>,
}

/// The figure of the character `character` ([`DriverSetup::character`]). `pool` must hold the character bundle and the shader bundle.
pub fn load_driver(pool: &Pool, character: u32) -> Result<Driver> {
    let id = ResourceId::from_parts(character, 0, 0, 1).0;
    let spec = expect(pool, id, ResourceKind::CharacterSpec)?;
    let model = spec.imports.iter().find(|import| import.offset == MODEL_AT).map(|import| import.id);
    let model = model.ok_or_else(|| Error::Spec(format!("character {character:X} has no model")))?;
    Ok(Driver { character, lods: model_lods(pool, &Surfaces::default(), model)? })
}

impl Driver {
    /// The figure sitting at `seat`: every level moved by it.
    pub fn seated(&self, seat: [f32; 3]) -> Vec<Lod> {
        let mut lods = self.lods.clone();
        for vertex in lods.iter_mut().flatten().flat_map(|mesh| mesh.vertices.iter_mut()) {
            for (axis, offset) in vertex.position.iter_mut().zip(seat) {
                *axis += offset;
            }
        }
        lods
    }

    /// The diffuse textures the figure's meshes draw, each once.
    pub fn textures(&self) -> Vec<u64> {
        let mut ids: Vec<u64> = self.lods.iter().flatten().filter_map(|mesh| mesh.appearance.diffuse_texture).collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    }
}

/// Whether `mesh` is the driver a body bakes into itself.
pub fn is_driver(mesh: &PartMesh) -> bool {
    mesh.appearance.technique.ends_with(BAKED_TECHNIQUE)
}

/// The positions of the baked driver's meshes in `lod`.
pub fn driver_meshes(lod: &Lod) -> Vec<usize> {
    lod.iter().enumerate().filter(|(_, mesh)| is_driver(mesh)).map(|(at, _)| at).collect()
}

/// Take the baked driver out of the body of `vehicle` at every level of detail, and say how many meshes the finest level
/// lost.
pub fn remove_baked_driver(vehicle: &mut Vehicle) -> usize {
    let mut removed = 0;
    for (level, lod) in vehicle.body.iter_mut().enumerate() {
        let before = lod.len();
        lod.retain(|mesh| !is_driver(mesh));
        if level == 0 {
            removed = before - lod.len();
        }
    }
    removed
}
