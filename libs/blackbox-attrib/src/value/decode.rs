// Portions ported from VaultLib (https://github.com/NFSTools/VaultLib),
// Copyright (c) 2019 NFS Tools & heyitsleo (MIT): the reading of Attrib::Array, StringKey,
// RefSpec and Blob values (VaultLib.Core/Types/VLTArrayType.cs, VaultLib.LegacyBase/StringKey64.cs,
// VaultLib.Core/Types/Attrib/RefSpec.cs, VaultLib.Core/Types/Attrib/BaseBlob.cs).
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

//! Decoding a field's value at a location in a vault.

use super::{Blob, RefSpec, StringKey, TypeKind, Value};
use crate::Result;
use crate::class::Field;
use crate::vault::{Location, Vault};

/// The value of `field` stored at `at` (an `Attrib::Array` header for array fields).
pub(crate) fn decode_field(vault: &Vault, field: &Field, at: Location) -> Result<Value> {
    if field.is_array() {
        decode_array(vault, field, at)
    } else {
        decode_scalar(vault, field.type_key, usize::from(field.size), at)
    }
}

/// `{u16 capacity, u16 count, u16 item size, u16 flags}`, then `count` items `item size` apart,
/// starting right after the header, or one header length later when the 16-byte-alignment flag
/// is set.
fn decode_array(vault: &Vault, field: &Field, at: Location) -> Result<Value> {
    let a = &vault.layout().array;
    let capacity = vault.u16(at.advance(a.capacity))?;
    let count = vault.u16(at.advance(a.count))?;
    let item_size = usize::from(vault.u16(at.advance(a.item_size))?);
    let flags = vault.u16(at.advance(a.flags))?;
    if count > capacity || item_size != usize::from(field.size) {
        return Err(vault.error(format!(
            "array at {at} (field 0x{:08X}): {count} of {capacity} items of {item_size} bytes, field size {}",
            field.key, field.size
        )));
    }
    let pad = if flags & a.aligned16 != 0 { a.header_len } else { 0 };
    let first = at.advance(a.header_len + pad);
    vault.bytes(first, usize::from(count) * item_size)?;
    (0..usize::from(count))
        .map(|i| decode_scalar(vault, field.type_key, item_size, first.advance(i * item_size)))
        .collect::<Result<_>>()
        .map(Value::Array)
}

fn decode_scalar(vault: &Vault, type_key: u32, size: usize, at: Location) -> Result<Value> {
    let layout = vault.layout();
    let b = vault.bytes(at, size)?;
    let Some(kind) = TypeKind::from_key(type_key).filter(|k| k.size(layout) == size) else {
        return Ok(Value::Raw { type_key, bytes: b.to_vec() });
    };
    let u16_at = |o: usize| u16::from_le_bytes([b[o], b[o + 1]]);
    let u32_at = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().expect("4 bytes"));
    let u64_at = |o: usize| u64::from_le_bytes(b[o..o + 8].try_into().expect("8 bytes"));
    let f32_at = |o: usize| f32::from_bits(u32_at(o));
    let text_at = |o: usize| vault.pointer(at.advance(o))?.map(|p| vault.cstr(p)).transpose();
    Ok(match kind {
        TypeKind::Bool => Value::Bool(b[0] != 0),
        TypeKind::Int8 => Value::Int8(b[0] as i8),
        TypeKind::UInt8 => Value::UInt8(b[0]),
        TypeKind::Int16 => Value::Int16(u16_at(0) as i16),
        TypeKind::UInt16 => Value::UInt16(u16_at(0)),
        TypeKind::Int32 => Value::Int32(u32_at(0) as i32),
        TypeKind::UInt32 => Value::UInt32(u32_at(0)),
        TypeKind::Int64 => Value::Int64(u64_at(0) as i64),
        TypeKind::UInt64 => Value::UInt64(u64_at(0)),
        TypeKind::Float => Value::Float(f32_at(0)),
        TypeKind::Double => Value::Double(f64::from_bits(u64_at(0))),
        TypeKind::Key => Value::Key(u32_at(0)),
        TypeKind::Text => Value::Text(text_at(0)?),
        TypeKind::Vector2 => Value::Vector2([f32_at(0), f32_at(4)]),
        TypeKind::Vector3 => Value::Vector3([f32_at(0), f32_at(4), f32_at(8)]),
        TypeKind::Vector4 => Value::Vector4([f32_at(0), f32_at(4), f32_at(8), f32_at(12)]),
        TypeKind::StringKey => {
            let l = &layout.string_key;
            Value::StringKey(StringKey {
                hash64: l.hash64.map(u64_at),
                hash32: u32_at(l.hash32),
                string: text_at(l.string)?,
            })
        }
        TypeKind::RefSpec => {
            let l = &layout.ref_spec;
            Value::RefSpec(RefSpec { class: u32_at(l.class), collection: u32_at(l.collection) })
        }
        TypeKind::Blob => {
            let l = &layout.blob;
            let len = u32_at(l.size) as usize;
            let data = match vault.pointer(at.advance(l.data))? {
                Some(p) => vault.bytes(p, len)?.to_vec(),
                None if len == 0 => Vec::new(),
                None => return Err(vault.error(format!("blob at {at} has {len} bytes but a null pointer"))),
            };
            Value::Blob(Blob { data })
        }
    })
}
