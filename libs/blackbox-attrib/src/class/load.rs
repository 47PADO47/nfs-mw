// Portions ported from VaultLib (https://github.com/NFSTools/VaultLib),
// Copyright (c) 2019 NFS Tools & heyitsleo (MIT): the reading of ClassLoadData and its
// definitions (VaultLib.LegacyBase/Exports/ClassLoad.cs, VaultLib.LegacyBase/AttribDefinition.cs).
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

//! Reading `Attrib::ClassLoadData`: the class header in the `.vlt`, its definitions behind a
//! pointer (in the `.bin` in every MW vault).

use super::{Class, Field, FieldFlags};
use crate::vault::{Export, Location, Vault};
use crate::{Error, Result};

pub(crate) fn read_class(vault: &Vault, export: &Export, vault_index: usize) -> Result<Class> {
    let l = &vault.layout().class_load;
    let d = &vault.layout().definition;
    if export.size != l.len {
        return Err(Error::UnsupportedLayout {
            vault: vault.name().to_owned(),
            detail: format!(
                "ClassLoadData of {} bytes (the {} layout has {})",
                export.size,
                vault.layout().name,
                l.len
            ),
        });
    }
    let at = |field: usize| Location::vlt(export.offset + field);
    let key = vault.u32(at(l.key))?;
    let count = vault.u32(at(l.definition_count))? as usize;
    let definitions = vault.required_pointer(at(l.definitions), "class definitions")?;
    // Bounds-check the whole table before allocating for it.
    vault.bytes(definitions, count.saturating_mul(d.len))?;
    let fields = (0..count)
        .map(|i| {
            let def = |field: usize| definitions.advance(i * d.len + field);
            let alignment_log2 = vault.u8(def(d.alignment_log2))?;
            Ok(Field {
                key: vault.u32(def(d.key))?,
                type_key: vault.u32(def(d.type_key))?,
                offset: vault.u16(def(d.offset))?,
                size: vault.u16(def(d.size))?,
                max_count: vault.u16(def(d.max_count))?,
                flags: FieldFlags(vault.u8(def(d.flags))?),
                alignment: 1u32.checked_shl(u32::from(alignment_log2)).unwrap_or(0),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Class {
        key,
        fields,
        layout_size: vault.u32(at(l.layout_size))?,
        layout_count: vault.u32(at(l.layout_count))?,
        collection_reserve: vault.u32(at(l.collection_reserve))?,
        vault: vault_index,
    })
}
