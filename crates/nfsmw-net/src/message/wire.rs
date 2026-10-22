//! The [`Wire`] trait and its implementations for numbers, arrays, options and bounded lists and byte strings.
//!
//! The layouts are postcard's: `u8` one raw byte, `u16`/`u32`/`u64` LEB128 varints, `i32` a zigzag varint, `bool` one
//! byte 0 or 1, `f32` 4 bytes little-endian (finite only), an array its elements back to back, an `Option` a tag byte
//! 0 or 1 then the value, a list or byte string a varint count then the elements.

use std::ops::Deref;

use super::codec::{DecodeError, LimitError, Reader, Writer};

/// A value with a canonical wire form. `take` never panics and refuses every value `put` cannot write.
pub trait Wire: Sized {
    /// Appends the value.
    fn put(&self, w: &mut Writer<'_>);
    /// Reads one value.
    fn take(r: &mut Reader<'_>) -> Result<Self, DecodeError>;
}

impl Wire for u8 {
    fn put(&self, w: &mut Writer<'_>) {
        w.u8(*self);
    }
    fn take(r: &mut Reader<'_>) -> Result<Self, DecodeError> {
        r.u8()
    }
}

impl Wire for u16 {
    fn put(&self, w: &mut Writer<'_>) {
        w.varint(u64::from(*self));
    }
    fn take(r: &mut Reader<'_>) -> Result<Self, DecodeError> {
        let value = r.varint(u64::from(u16::MAX))?;
        u16::try_from(value).map_err(|_| DecodeError::BadValue)
    }
}

impl Wire for u32 {
    fn put(&self, w: &mut Writer<'_>) {
        w.varint(u64::from(*self));
    }
    fn take(r: &mut Reader<'_>) -> Result<Self, DecodeError> {
        let value = r.varint(u64::from(u32::MAX))?;
        u32::try_from(value).map_err(|_| DecodeError::BadValue)
    }
}

impl Wire for u64 {
    fn put(&self, w: &mut Writer<'_>) {
        w.varint(*self);
    }
    fn take(r: &mut Reader<'_>) -> Result<Self, DecodeError> {
        r.varint(u64::MAX)
    }
}

impl Wire for i32 {
    fn put(&self, w: &mut Writer<'_>) {
        w.zigzag(i64::from(*self));
    }
    fn take(r: &mut Reader<'_>) -> Result<Self, DecodeError> {
        let value = r.zigzag(i64::from(i32::MIN), i64::from(i32::MAX))?;
        i32::try_from(value).map_err(|_| DecodeError::BadValue)
    }
}

impl Wire for bool {
    fn put(&self, w: &mut Writer<'_>) {
        w.u8(u8::from(*self));
    }
    fn take(r: &mut Reader<'_>) -> Result<Self, DecodeError> {
        match r.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(DecodeError::BadValue),
        }
    }
}

impl Wire for f32 {
    fn put(&self, w: &mut Writer<'_>) {
        w.bytes(&self.to_le_bytes());
    }
    fn take(r: &mut Reader<'_>) -> Result<Self, DecodeError> {
        let bytes: [u8; 4] = r.take(4)?.try_into().map_err(|_| DecodeError::Truncated)?;
        let value = f32::from_le_bytes(bytes);
        if !value.is_finite() {
            return Err(DecodeError::BadValue);
        }
        Ok(value)
    }
}

impl<T: Wire + Copy + Default, const N: usize> Wire for [T; N] {
    fn put(&self, w: &mut Writer<'_>) {
        for item in self {
            item.put(w);
        }
    }
    fn take(r: &mut Reader<'_>) -> Result<Self, DecodeError> {
        let mut out = [T::default(); N];
        for slot in out.iter_mut() {
            *slot = T::take(r)?;
        }
        Ok(out)
    }
}

impl<T: Wire> Wire for Option<T> {
    fn put(&self, w: &mut Writer<'_>) {
        match self {
            None => w.u8(0),
            Some(value) => {
                w.u8(1);
                value.put(w);
            }
        }
    }
    fn take(r: &mut Reader<'_>) -> Result<Self, DecodeError> {
        match r.u8()? {
            0 => Ok(None),
            1 => Ok(Some(T::take(r)?)),
            _ => Err(DecodeError::BadValue),
        }
    }
}

/// A list of at most `MAX` values. The bound is part of the type, so a decoded list never holds more.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct List<T, const MAX: usize>(Vec<T>);

impl<T, const MAX: usize> List<T, MAX> {
    /// The list of `items`, or [`LimitError`] when there are more than `MAX`.
    pub fn new(items: Vec<T>) -> Result<Self, LimitError> {
        if items.len() > MAX {
            return Err(LimitError);
        }
        Ok(Self(items))
    }

    /// An empty list.
    pub fn empty() -> Self {
        Self(Vec::new())
    }

    /// Appends `item`, or [`LimitError`] when the list is full.
    pub fn push(&mut self, item: T) -> Result<(), LimitError> {
        if self.0.len() >= MAX {
            return Err(LimitError);
        }
        self.0.push(item);
        Ok(())
    }

    /// The items.
    pub fn into_inner(self) -> Vec<T> {
        self.0
    }
}

impl<T, const MAX: usize> Default for List<T, MAX> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<T, const MAX: usize> Deref for List<T, MAX> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        &self.0
    }
}

impl<T: Wire, const MAX: usize> Wire for List<T, MAX> {
    fn put(&self, w: &mut Writer<'_>) {
        w.varint(self.0.len() as u64);
        for item in &self.0 {
            item.put(w);
        }
    }
    fn take(r: &mut Reader<'_>) -> Result<Self, DecodeError> {
        let count = r.count(MAX)?;
        let mut items = Vec::with_capacity(count);
        for _ in 0..count {
            items.push(T::take(r)?);
        }
        Ok(Self(items))
    }
}

/// Opaque bytes, at most `MAX` of them (PAKE messages, tokens, bulk chunks).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Bytes<const MAX: usize>(Vec<u8>);

impl<const MAX: usize> Bytes<MAX> {
    /// The byte string `bytes`, or [`LimitError`] when it is longer than `MAX`.
    pub fn new(bytes: Vec<u8>) -> Result<Self, LimitError> {
        if bytes.len() > MAX {
            return Err(LimitError);
        }
        Ok(Self(bytes))
    }

    /// The bytes.
    pub fn into_inner(self) -> Vec<u8> {
        self.0
    }
}

impl<const MAX: usize> Deref for Bytes<MAX> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.0
    }
}

impl<const MAX: usize> Wire for Bytes<MAX> {
    fn put(&self, w: &mut Writer<'_>) {
        w.varint(self.0.len() as u64);
        w.bytes(&self.0);
    }
    fn take(r: &mut Reader<'_>) -> Result<Self, DecodeError> {
        let count = r.count(MAX)?;
        Ok(Self(r.take(count)?.to_vec()))
    }
}

/// The encoded size of `value` in bytes.
pub fn wire_len<T: Wire>(value: &T) -> usize {
    let mut out = Vec::new();
    value.put(&mut Writer::new(&mut out));
    out.len()
}
