//! The compact byte form of a selection, for the network and for saves, and the digest that tells two peers they hold
//! the same definition. The bytes are canonical (slots in order, fixed widths) so equal selections encode equally.
//!
//! Layout: `version u8`, `count u16`, then per pick in slot order `slot str`, `has component u8`, `component str`
//! (when there is one), `inputs u8`, and per input `key str`, `tag u8` (0 rgb: 3 bytes, 1 number: `f64` bits, 2 text:
//! `str`). A `str` is `len u16` and UTF-8 bytes. All integers are little-endian.

use std::collections::BTreeMap;

use crate::effect::Input;
use crate::model::{Catalog, VehicleDef};
use crate::selection::{Pick, Selection};

pub const WIRE_VERSION: u8 = 1;
/// Most picks a selection may hold: a vehicle with more slots is refused by [`VehicleDef::check`]'s callers.
pub const MAX_PICKS: usize = 256;
const MAX_INPUTS: usize = 8;
const MAX_STR: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireError {
    Truncated,
    Version(u8),
    TooLarge,
    Utf8,
    /// Slots must be strictly increasing, so the same selection has one encoding.
    Order,
    Tag(u8),
    NotFinite,
    Trailing,
}

impl Selection {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = vec![WIRE_VERSION];
        out.extend((self.picks.len() as u16).to_le_bytes());
        for (slot, pick) in &self.picks {
            put_str(&mut out, slot);
            match &pick.component {
                None => out.push(0),
                Some(component) => {
                    out.push(1);
                    put_str(&mut out, component);
                }
            }
            out.push(pick.inputs.len() as u8);
            for (key, input) in &pick.inputs {
                put_str(&mut out, key);
                match input {
                    Input::Rgb(rgb) => {
                        out.push(0);
                        out.extend(rgb);
                    }
                    Input::Num(n) => {
                        out.push(1);
                        out.extend(n.to_bits().to_le_bytes());
                    }
                    Input::Text(text) => {
                        out.push(2);
                        put_str(&mut out, text);
                    }
                }
            }
        }
        out
    }

    /// Read bytes from another machine: bounded, and strict about everything the encoder does. A decoded selection
    /// still has to pass [`VehicleDef::validate`] for the vehicle it is meant for.
    pub fn decode(bytes: &[u8]) -> Result<Self, WireError> {
        let mut reader = Reader { bytes };
        let version = reader.u8()?;
        if version != WIRE_VERSION {
            return Err(WireError::Version(version));
        }
        let count = usize::from(reader.u16()?);
        if count > MAX_PICKS {
            return Err(WireError::TooLarge);
        }
        let mut picks = BTreeMap::new();
        let mut last: Option<String> = None;
        for _ in 0..count {
            let slot = reader.str()?;
            if last.as_ref().is_some_and(|last| *last >= slot) {
                return Err(WireError::Order);
            }
            let component = match reader.u8()? {
                0 => None,
                1 => Some(reader.str()?),
                tag => return Err(WireError::Tag(tag)),
            };
            let inputs = reader.inputs()?;
            last = Some(slot.clone());
            picks.insert(slot, Pick { component, inputs });
        }
        match reader.bytes.is_empty() {
            true => Ok(Self { picks }),
            false => Err(WireError::Trailing),
        }
    }
}

fn put_str(out: &mut Vec<u8>, text: &str) {
    out.extend((text.len() as u16).to_le_bytes());
    out.extend(text.as_bytes());
}

struct Reader<'a> {
    bytes: &'a [u8],
}

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], WireError> {
        let (head, tail) = self.bytes.split_at_checked(n).ok_or(WireError::Truncated)?;
        self.bytes = tail;
        Ok(head)
    }

    fn u8(&mut self) -> Result<u8, WireError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, WireError> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }

    fn str(&mut self) -> Result<String, WireError> {
        let len = usize::from(self.u16()?);
        if len > MAX_STR {
            return Err(WireError::TooLarge);
        }
        String::from_utf8(self.take(len)?.to_vec()).map_err(|_| WireError::Utf8)
    }

    fn inputs(&mut self) -> Result<BTreeMap<String, Input>, WireError> {
        let count = usize::from(self.u8()?);
        if count > MAX_INPUTS {
            return Err(WireError::TooLarge);
        }
        let mut inputs = BTreeMap::new();
        for _ in 0..count {
            let key = self.str()?;
            let input = match self.u8()? {
                0 => Input::Rgb(self.take(3)?.try_into().unwrap()),
                1 => {
                    let n = f64::from_bits(u64::from_le_bytes(self.take(8)?.try_into().unwrap()));
                    match n.is_finite() {
                        true => Input::Num(n),
                        false => return Err(WireError::NotFinite),
                    }
                }
                2 => Input::Text(self.str()?),
                tag => return Err(WireError::Tag(tag)),
            };
            if inputs.insert(key, input).is_some() {
                return Err(WireError::Order);
            }
        }
        Ok(inputs)
    }
}

impl VehicleDef {
    /// A 64-bit FNV-1a digest of the definition and of every component it accepts. Peers compare it before they
    /// compare selections: equal digests mean the same slots, rules and effects, so the same resolution.
    pub fn digest(&self, catalog: &Catalog) -> u64 {
        let accepted: Vec<_> = self
            .slots
            .iter()
            .flat_map(|slot| &slot.accepts)
            .filter_map(|offer| catalog.get(&offer.component))
            .collect();
        let bytes = serde_json::to_vec(&(self, accepted)).unwrap_or_default();
        bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3))
    }
}
