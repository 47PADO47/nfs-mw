//! The image of a module instance: little-endian reads and writes that never panic. A bad offset reads 0, drops a
//! write and leaves a mark, which [`Instance::update`](crate::Instance::update) turns into an error.

use std::cell::Cell;

#[derive(Debug, Clone)]
pub(crate) struct Mem {
    bytes: Vec<u8>,
    fault: Cell<Option<usize>>,
}

impl Mem {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { bytes, fault: Cell::new(None) }
    }

    /// The first bad offset since the last [`take_fault`](Self::take_fault).
    pub fn take_fault(&self) -> Option<usize> {
        self.fault.take()
    }

    fn mark(&self, at: usize) {
        if self.fault.get().is_none() {
            self.fault.set(Some(at));
        }
    }

    fn read<const N: usize>(&self, at: usize) -> [u8; N] {
        match at.checked_add(N).and_then(|end| self.bytes.get(at..end)) {
            Some(slice) => {
                let mut out = [0; N];
                out.copy_from_slice(slice);
                out
            }
            None => {
                self.mark(at);
                [0; N]
            }
        }
    }

    fn write<const N: usize>(&mut self, at: usize, value: [u8; N]) {
        match at.checked_add(N).and_then(|end| self.bytes.get_mut(at..end)) {
            Some(slice) => slice.copy_from_slice(&value),
            None => self.mark(at),
        }
    }

    pub fn offset(&self, base: usize, delta: i64) -> usize {
        let at = base as i64 + delta;
        if at < 0 {
            self.mark(0);
            return usize::MAX;
        }
        at as usize
    }

    pub fn u8(&self, at: usize) -> u8 {
        self.read::<1>(at)[0]
    }
    pub fn i8(&self, at: usize) -> i8 {
        self.u8(at) as i8
    }
    pub fn u16(&self, at: usize) -> u16 {
        u16::from_le_bytes(self.read(at))
    }
    pub fn i16(&self, at: usize) -> i16 {
        i16::from_le_bytes(self.read(at))
    }
    pub fn u32(&self, at: usize) -> u32 {
        u32::from_le_bytes(self.read(at))
    }
    pub fn i32(&self, at: usize) -> i32 {
        i32::from_le_bytes(self.read(at))
    }
    pub fn f32(&self, at: usize) -> f32 {
        f32::from_le_bytes(self.read(at))
    }

    pub fn set_u8(&mut self, at: usize, v: u8) {
        self.write(at, [v]);
    }
    pub fn set_i8(&mut self, at: usize, v: i8) {
        self.write(at, [v as u8]);
    }
    pub fn set_u16(&mut self, at: usize, v: u16) {
        self.write(at, v.to_le_bytes());
    }
    pub fn set_i16(&mut self, at: usize, v: i16) {
        self.write(at, v.to_le_bytes());
    }
    pub fn set_i32(&mut self, at: usize, v: i32) {
        self.write(at, v.to_le_bytes());
    }
    pub fn set_f32(&mut self, at: usize, v: f32) {
        self.write(at, v.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_writes_round_trip() {
        let mut m = Mem::new(vec![0; 16]);
        m.set_i32(4, -5);
        m.set_f32(8, 1.5);
        m.set_i16(12, -2);
        assert_eq!((m.i32(4), m.f32(8), m.i16(12)), (-5, 1.5, -2));
        assert_eq!(m.take_fault(), None);
    }

    #[test]
    fn bad_offsets_read_zero_and_leave_a_mark() {
        let mut m = Mem::new(vec![1; 8]);
        assert_eq!(m.i32(6), 0);
        assert_eq!(m.take_fault(), Some(6));
        m.set_i32(100, 7);
        assert_eq!(m.take_fault(), Some(100));
        assert_eq!(m.i32(usize::MAX), 0);
        assert!(m.take_fault().is_some());
    }
}
