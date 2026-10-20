//! The parameter values of one speech request.
//!
//! A sentence says different phrases for different circumstances (which car, which direction, how hard the hit).
//! The circumstances are the *parameters* of the request: numbered from 1 in the order the caller passes them, each a
//! set of bits (`1 << n` for the nth choice of that parameter; a parameter can also name several choices at once).
//! The sentence database tests them by number, so this crate does not know what any parameter means.

/// How many parameters a request can carry (number 0 is never used).
pub const MAX_PARAMS: usize = 16;

/// A sparse set of parameter values, by number.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Params {
    values: [Option<u32>; MAX_PARAMS],
}

impl Params {
    /// No parameter set: every test of a sentence accepts any value.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set parameter `number` (1 or more) to `value`. Numbers out of range are ignored.
    pub fn set(&mut self, number: u8, value: u32) {
        let Some(slot) = self.values.get_mut(usize::from(number)) else { return };
        *slot = Some(value);
    }

    /// `self` with parameter `number` set.
    pub fn with(mut self, number: u8, value: u32) -> Self {
        self.set(number, value);
        self
    }

    /// The value of parameter `number`, if set.
    pub fn get(&self, number: u8) -> Option<u32> {
        self.values.get(usize::from(number)).copied().flatten()
    }

    /// Whether parameter `number` allows the bit `bit`. A parameter that is not set allows every bit, so a request
    /// with fewer parameters than the sentence tests still gets a line.
    pub fn allows(&self, number: u8, bit: u8) -> bool {
        let Some(value) = self.get(number) else { return true };
        bit < 32 && value & (1 << bit) != 0
    }
}
