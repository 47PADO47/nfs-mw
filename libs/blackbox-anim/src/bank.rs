//! An animation bank: the named animations of one `EAGLAnimations` chunk.
//!
//! The bank symbol `__AnimationBank:::bank` points at a 0x1C-byte header in `.data`. The header holds two parallel
//! pointer arrays, one entry per animation: the animation records and their NUL-terminated names. The names are
//! sorted, so [`Bank::find`] binary-searches them. Layout: `docs/specs/eagl-animation-bank.md` sections 2 to 4.

use crate::{Error, Object, Result};

/// The symbol that names a bank's header in `.data`.
const BANK_SYMBOL: &str = "__AnimationBank:::bank";
/// The size the header holds in its first word (bank spec section 3).
const HEADER_LEN: u32 = 0x1C;
const SIZE: usize = 0x00;
const COUNT: usize = 0x04;
const ANIMATIONS: usize = 0x10;
const NAMES: usize = 0x14;
/// The width of one entry in the two pointer arrays.
const POINTER_LEN: usize = 4;
/// The offset of the target checksum in an animation record (the type id is at offset 0).
const CHECKSUM: usize = 2;

/// The animations of one bank, in the order the bank stores them (sorted by name).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bank {
    /// Every animation of the bank.
    pub animations: Vec<Animation>,
}

/// One entry of a bank: the name and the record's head. The record itself is read with its own type (for a
/// compound, [`crate::Compound::read`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Animation {
    /// The animation name, for example `ArrestF02_Cuffs_s`.
    pub name: String,
    /// The type id at offset 0 of the record (15 for every animation in the install).
    pub type_id: u16,
    /// The target checksum at offset 2 of the record: the skeleton the animation drives.
    pub checksum: u16,
    /// The offset in `.data` of the animation record.
    pub offset: u32,
}

impl Bank {
    /// Reads the bank named by the object's `__AnimationBank:::bank` symbol.
    ///
    /// A bank with no animations is valid; its pointer arrays may be null. A missing bank symbol, a header of the
    /// wrong size, or a null entry in a non-empty array is an error.
    pub fn read(object: &Object) -> Result<Self> {
        let symbol =
            object.symbol(BANK_SYMBOL).ok_or_else(|| Error::Invalid(format!("object has no {BANK_SYMBOL} symbol")))?;
        let header = symbol.value as usize;
        let size = object.u32_at(header + SIZE)?;
        if size != HEADER_LEN {
            return Err(Error::Invalid(format!("bank header at {header:#x} has size {size:#x}, not {HEADER_LEN:#x}")));
        }
        let count = object.u32_at(header + COUNT)?;
        if count == 0 {
            return Ok(Self { animations: Vec::new() });
        }
        let records = required_pointer(object, header + ANIMATIONS, "animation array")?;
        let names = required_pointer(object, header + NAMES, "name array")?;
        let mut animations = Vec::new();
        for index in 0..count as usize {
            animations.push(read_animation(object, records, names, index)?);
        }
        Ok(Self { animations })
    }

    /// The animation called `name`, or `None`.
    ///
    /// The names are sorted, so the lookup first binary-searches them. A name that the binary search misses is
    /// then looked for by a linear scan, so a bank whose names are not in order still answers correctly.
    pub fn find(&self, name: &str) -> Option<&Animation> {
        match self.animations.binary_search_by(|animation| animation.name.as_str().cmp(name)) {
            Ok(index) => self.animations.get(index),
            Err(_) => self.animations.iter().find(|animation| animation.name == name),
        }
    }

    /// The animations in bank order.
    pub fn iter(&self) -> std::slice::Iter<'_, Animation> {
        self.animations.iter()
    }
}

/// The offset that the pointer at `at` holds. A null pointer is an error here.
fn required_pointer(object: &Object, at: usize, what: &str) -> Result<usize> {
    let target = object.ptr_at(at)?;
    target
        .map(|target| target as usize)
        .ok_or_else(|| Error::Invalid(format!("bank {what} pointer at {at:#x} is null")))
}

/// Reads entry `index` of both pointer arrays and the record it names.
fn read_animation(object: &Object, records: usize, names: usize, index: usize) -> Result<Animation> {
    let slot = index * POINTER_LEN;
    let record = object
        .ptr_at(records + slot)?
        .ok_or_else(|| Error::Invalid(format!("animation {index} has a null record pointer")))?;
    let name = object
        .ptr_at(names + slot)?
        .ok_or_else(|| Error::Invalid(format!("animation {index} has a null name pointer")))?;
    let name = object.cstr_at(name as usize)?.to_string();
    let head = record as usize;
    let type_id = object.u16_at(head)?;
    let checksum = object.u16_at(head + CHECKSUM)?;
    Ok(Animation { name, type_id, checksum, offset: record })
}
