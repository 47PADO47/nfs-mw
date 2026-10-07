// Portions ported from VaultLib (https://github.com/NFSTools/VaultLib),
// Copyright (c) 2019 NFS Tools & heyitsleo (MIT): the reading of CollectionLoadData, its layout
// block and its attribute entries (VaultLib.LegacyBase/Exports/CollectionLoad.cs,
// VaultLib.LegacyBase/Exports/AttribEntry.cs).
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

//! Reading `Attrib::CollectionLoadData`.
//!
//! The record (in the `.vlt`) is a header, the list of type keys its entries use, and one entry
//! per optional attribute. The in-layout fields live in a separate layout block behind a pointer
//! (in the `.bin`), at the offsets the class definition gives. An entry's value is stored in the
//! entry itself when it fits (`inline_max` bytes and not an array), else behind a pointer.

use super::{Attribute, Collection};
use crate::class::Class;
use crate::value::decode_field;
use crate::vault::{Export, Location, Vault};
use crate::{Error, Result};

pub(crate) fn read_collection<'c>(
    vault: &Vault,
    export: &Export,
    vault_index: usize,
    class_of: impl Fn(u32) -> Option<&'c Class>,
) -> Result<Collection> {
    let layout = vault.layout();
    let (l, e) = (&layout.collection_load, &layout.entry);
    let at = |field: usize| Location::vlt(export.offset + field);
    let key = vault.u32(at(l.key))?;
    let class_key = vault.u32(at(l.class))?;
    let parent = vault.u32(at(l.parent))?;
    let entry_count = vault.u32(at(l.entry_count))? as usize;
    let type_count = vault.u32(at(l.type_count))? as usize;

    let entries_at = l.header_len.saturating_add(type_count.saturating_mul(4));
    let expected = entries_at.saturating_add(entry_count.saturating_mul(e.len));
    if export.size != expected {
        return Err(Error::UnsupportedLayout {
            vault: vault.name().to_owned(),
            detail: format!(
                "CollectionLoadData 0x{key:08X} is {} bytes; the {} layout needs {expected} for {type_count} types \
                 and {entry_count} entries",
                export.size, layout.name
            ),
        });
    }
    let class = class_of(class_key).ok_or_else(|| Error::UnknownClass {
        vault: vault.name().to_owned(),
        collection: key,
        class: class_key,
    })?;
    let context = |detail: String| vault.error(format!("collection 0x{key:08X} of class 0x{class_key:08X}: {detail}"));

    let mut attributes = Vec::with_capacity(entry_count + class.layout_count as usize);
    if let Some(block) = vault.pointer(at(l.layout))? {
        for field in class.layout_fields() {
            let value = decode_field(vault, field, block.advance(usize::from(field.offset)))?;
            attributes.push(Attribute { key: field.key, value, in_layout: true });
        }
    }

    let types = (0..type_count).map(|i| vault.u32(at(l.header_len + i * 4))).collect::<Result<Vec<_>>>()?;
    for i in 0..entry_count {
        let entry = |field: usize| at(entries_at + i * e.len + field);
        let field_key = vault.u32(entry(e.key))?;
        let field = class
            .field_by_key(field_key)
            .ok_or_else(|| context(format!("entry {i} is field 0x{field_key:08X}, which the class lacks")))?;
        let type_index = usize::from(vault.u16(entry(e.type_index))?);
        if types.get(type_index) != Some(&field.type_key) {
            return Err(context(format!(
                "entry {i} (field 0x{field_key:08X}) has type index {type_index}, not its type"
            )));
        }
        let inline = !field.is_array() && usize::from(field.size) <= layout.inline_max;
        let value_at = if inline { entry(e.data) } else { vault.required_pointer(entry(e.data), "attribute value")? };
        let value = decode_field(vault, field, value_at)?;
        attributes.push(Attribute { key: field_key, value, in_layout: false });
    }

    Ok(Collection { key, class: class_key, parent: (parent != 0).then_some(parent), vault: vault_index, attributes })
}
