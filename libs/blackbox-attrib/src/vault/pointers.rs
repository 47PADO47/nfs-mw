// Portions ported from VaultLib (https://github.com/NFSTools/VaultLib),
// Copyright (c) 2019 NFS Tools & heyitsleo (MIT): the meaning of the PtrN records
// (VaultLib.Core/Chunks/VLTPointersChunk.cs, VaultLib.Core/DataInterfaces/IPtrRef.cs).
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! `PtrN`: pointer fix-ups.
//!
//! Pointers are stored as 0; the loader patches them. Each 12-byte record is
//! `{u32 fix-up offset, u16 kind, u16 index, u32 destination}`:
//!
//! | kind | name | meaning |
//! |---|---|---|
//! | 0 | `PtrEnd` | end of the table |
//! | 1 | `PtrNull` | the pointer at the offset stays null |
//! | 2 | `PtrSetFixupTarget` | the following fix-up offsets are in dependency `index` |
//! | 3 | `PtrDepRelative` | the pointer at the offset points to `destination` in dependency `index` |
//! | 4 | `PtrExport` | not seen in any supported game; refused |
//!
//! Dependencies are the vault's `DepN` entries; their names say which blob they are
//! (`<vault>.vlt`, `<vault>.bin`). Without a `DepN`, index 0 is the `.vlt` and 1 the `.bin`.

use std::collections::HashMap;

use super::{Dependency, Location, Stream};
use crate::bytes::{u16_at, u32_at};

const RECORD_LEN: usize = 12;
const PTR_END: u16 = 0;
const PTR_NULL: u16 = 1;
const PTR_SET_FIXUP_TARGET: u16 = 2;
const PTR_DEP_RELATIVE: u16 = 3;

/// Every fix-up of one vault, by the location of the pointer.
#[derive(Debug, Clone, Default)]
pub struct Pointers {
    map: HashMap<Location, Option<Location>>,
}

impl Pointers {
    /// `None`: no fix-up at `at`. `Some(None)`: a `PtrNull` fix-up.
    pub fn get(&self, at: Location) -> Option<Option<Location>> {
        self.map.get(&at).copied()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }
}

fn stream_of(dependencies: &[Dependency], index: u16) -> Result<Stream, String> {
    let by_name = dependencies.get(usize::from(index)).map(|d| d.name.to_ascii_lowercase());
    match (by_name.as_deref(), index) {
        (Some(name), _) if name.ends_with(".vlt") => Ok(Stream::Vlt),
        (Some(name), _) if name.ends_with(".bin") => Ok(Stream::Bin),
        (None, 0) if dependencies.is_empty() => Ok(Stream::Vlt),
        (None, 1) if dependencies.is_empty() => Ok(Stream::Bin),
        _ => Err(format!("PtrN refers to dependency {index}, which is neither the vault's .vlt nor its .bin")),
    }
}

pub fn read_pointers(table: &[u8], dependencies: &[Dependency]) -> Result<Pointers, String> {
    let mut pointers = Pointers::default();
    let mut target = None;
    for (i, record) in table.as_chunks::<RECORD_LEN>().0.iter().enumerate() {
        let fixup = u32_at(record, 0).unwrap_or(0) as usize;
        let (kind, index) = (u16_at(record, 4).unwrap_or(0), u16_at(record, 6).unwrap_or(0));
        let destination = u32_at(record, 8).unwrap_or(0) as usize;
        let at = |target: Option<Stream>| {
            target
                .map(|stream| Location { stream, offset: fixup })
                .ok_or_else(|| format!("PtrN record {i} comes before any PtrSetFixupTarget"))
        };
        match kind {
            PTR_END => break,
            PTR_SET_FIXUP_TARGET => target = Some(stream_of(dependencies, index)?),
            PTR_NULL => {
                pointers.map.insert(at(target)?, None);
            }
            PTR_DEP_RELATIVE => {
                let to = Location { stream: stream_of(dependencies, index)?, offset: destination };
                pointers.map.insert(at(target)?, Some(to));
            }
            _ => return Err(format!("PtrN record {i} has unsupported kind {kind}")),
        }
    }
    Ok(pointers)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(fixup: u32, kind: u16, index: u16, destination: u32) -> Vec<u8> {
        [&fixup.to_le_bytes()[..], &kind.to_le_bytes(), &index.to_le_bytes(), &destination.to_le_bytes()].concat()
    }

    #[test]
    fn targets_and_kinds() {
        let table = [
            record(0, PTR_SET_FIXUP_TARGET, 1, 0),
            record(0x10, PTR_DEP_RELATIVE, 1, 0x40),
            record(0, PTR_SET_FIXUP_TARGET, 0, 0),
            record(0x20, PTR_DEP_RELATIVE, 1, 0x80),
            record(0x24, PTR_NULL, 0, 0),
            record(0, PTR_END, 0, 0),
            record(0x99, PTR_DEP_RELATIVE, 1, 0x99), // after the end: ignored
        ]
        .concat();
        let p = read_pointers(&table, &[]).unwrap();
        assert_eq!(p.len(), 3);
        assert_eq!(p.get(Location::bin(0x10)), Some(Some(Location::bin(0x40))));
        assert_eq!(p.get(Location::vlt(0x20)), Some(Some(Location::bin(0x80))));
        assert_eq!(p.get(Location::vlt(0x24)), Some(None));
        assert_eq!(p.get(Location::vlt(0x10)), None);
    }

    #[test]
    fn refuses_unknown_records() {
        assert!(read_pointers(&record(0x10, PTR_DEP_RELATIVE, 1, 0), &[]).is_err());
        let export = [record(0, PTR_SET_FIXUP_TARGET, 1, 0), record(0x10, 4, 1, 0)].concat();
        assert!(read_pointers(&export, &[]).is_err());
        let deps = [Dependency { key: 0, name: "x.vlt".into() }];
        assert!(read_pointers(&record(0, PTR_SET_FIXUP_TARGET, 1, 0), &deps).is_err());
    }
}
