// Portions ported from VaultLib (https://github.com/NFSTools/VaultLib),
// Copyright (c) 2019 NFS Tools & heyitsleo (MIT): the reading of DatabaseLoadData
// (VaultLib.Core/Exports/Implementations/DatabaseLoad.cs).
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

//! `Attrib::DatabaseLoadData`: the database's type table. A header, `u32 size[type_count]`, and
//! a pointer to `type_count` NUL-terminated type names in the same order.

use crate::Result;
use crate::vault::{Export, Location, Vault};

/// One type the database knows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeInfo {
    /// Hash of the name.
    pub key: u32,
    /// For example `EA::Reflection::Float`, `Attrib::RefSpec`, `EffectLinkageRecord`.
    pub name: String,
    /// Size of one value in bytes.
    pub size: u32,
}

pub(crate) fn read_types(vault: &Vault, export: &Export) -> Result<Vec<TypeInfo>> {
    let l = &vault.layout().database_load;
    let at = |field: usize| Location::vlt(export.offset + field);
    let count = vault.u32(at(l.type_count))? as usize;
    if export.size != l.header_len.saturating_add(count.saturating_mul(4)) {
        return Err(vault.error(format!("DatabaseLoadData of {} bytes for {count} types", export.size)));
    }
    let mut name_at = vault.required_pointer(at(l.type_names), "type names")?;
    (0..count)
        .map(|i| {
            let raw = vault.cstr_bytes(name_at)?;
            let key = crate::hash::lookup2(raw, crate::hash::VLT_HASH_INIT);
            name_at = name_at.advance(raw.len() + 1);
            let name = String::from_utf8_lossy(raw).into_owned();
            Ok(TypeInfo { key, name, size: vault.u32(at(l.header_len + i * 4))? })
        })
        .collect()
}
