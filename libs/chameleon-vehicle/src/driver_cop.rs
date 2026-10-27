//! The police officer: which cars seat the cop instead of the character their driver setup names. Spec:
//! `docs/formats/chameleon-vehicle.md` ("The driver").
//!
//! A car's `Vehicle_DriverSetup` names the character that sits in it, and in the five police cars it names the same
//! character as in every other car. The game does not use it for them: the object that references the setup (a
//! `VehicleComponent` structure, the setup's handle at +0x2C) has a byte at +0x8E that is 1 in the police cars and 0 in all
//! the others, and the game's character choice reads it first: set, the character is the officer (key
//! [`COP_CHARACTER`]); clear, the key the setup names, or the first character when that is 0.

use chameleon_bundle::{Bundle, ResourceKind};

use crate::driver::SETUP;
use crate::genesys::TypeNames;

/// The key of the `CharacterSpec` of the police officer (the third character of the character bundle; no car's setup names it).
pub const COP_CHARACTER: u32 = 0x3D8D;
/// Where the object that references the driver setup keeps the setup's handle, and the police flag.
const SETUP_REFERENCE_AT: u32 = 0x2C;
const COP_FLAG_AT: usize = 0x8E;
/// The type import bit of an import's slot offset.
const FLAG_BIT: u32 = 0x8000_0000;
/// Biggest main block of a Genesys object that is read, in bytes.
const MAX_OBJECT: usize = 65_536;

/// Whether the vehicle whose bundle is `bundle` is a police car, by the flag the game reads to pick its driver. `None` when
/// the bundle has no object that references the driver setup.
pub(crate) fn police_flag(bundle: &Bundle, types: &TypeNames) -> Option<bool> {
    let kind = ResourceKind::GenesysObject.id();
    let mut setup = None;
    for (index, entry) in bundle.entries().iter().enumerate() {
        if entry.kind != kind || entry.import_count == 0 || entry.block_size(0) > MAX_OBJECT {
            continue;
        }
        let resource = bundle.resource(index).ok()?;
        let is_setup =
            resource.imports.iter().any(|import| import.offset & FLAG_BIT != 0 && types.name(import.id) == Some(SETUP));
        if is_setup {
            setup = Some(entry.id);
            break;
        }
    }
    let setup = setup?;
    for (index, entry) in bundle.entries().iter().enumerate() {
        if entry.kind != kind || entry.import_count == 0 || entry.id == setup || entry.block_size(0) > MAX_OBJECT {
            continue;
        }
        let resource = bundle.resource(index).ok()?;
        let references =
            resource.imports.iter().any(|import| import.id == setup && import.slot() == SETUP_REFERENCE_AT);
        if references {
            return resource.blocks[0].get(COP_FLAG_AT).map(|&byte| byte != 0);
        }
    }
    None
}
