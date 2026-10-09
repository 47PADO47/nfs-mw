//! A small bitset over a fieldless enum, so capabilities can say "these anti-aliasing methods"
//! without a dependency.

use std::fmt;
use std::marker::PhantomData;

use crate::{Antialiasing, Tonemap, Upscaler};

use super::Setting;

/// A fieldless enum that fits in a [`EnumSet`]: at most 32 variants, `bit` is the variant's index.
pub trait SetMember: Copy + 'static {
    /// Every variant, in declaration order.
    const ALL: &'static [Self];

    /// The variant's index in `ALL`, below 32.
    fn bit(self) -> u32;
}

macro_rules! set_member {
    ($($ty:ty),* $(,)?) => {$(
        impl SetMember for $ty {
            const ALL: &'static [Self] = &<$ty>::ALL;

            fn bit(self) -> u32 {
                self as u32
            }
        }
    )*};
}

set_member!(Antialiasing, Upscaler, Tonemap, Setting);

/// A set of `T` (up to 32 distinct variants), stored in one `u32`.
pub struct EnumSet<T> {
    bits: u32,
    marker: PhantomData<T>,
}

/// The anti-aliasing methods a renderer offers.
pub type AaSet = EnumSet<Antialiasing>;
/// The upscalers a renderer offers.
pub type UpscalerSet = EnumSet<Upscaler>;
/// The tone-mapping curves a renderer offers.
pub type TonemapSet = EnumSet<Tonemap>;
/// Settings whose change needs a restart.
pub type RestartSet = EnumSet<Setting>;

impl<T> EnumSet<T> {
    pub const fn empty() -> Self {
        Self { bits: 0, marker: PhantomData }
    }

    pub const fn is_empty(&self) -> bool {
        self.bits == 0
    }

    pub const fn len(&self) -> u32 {
        self.bits.count_ones()
    }
}

impl<T: SetMember> EnumSet<T> {
    /// Every variant.
    pub fn all() -> Self {
        Self::of(T::ALL)
    }

    pub fn of(members: &[T]) -> Self {
        members.iter().fold(Self::empty(), |set, &m| set.with(m))
    }

    pub fn contains(&self, member: T) -> bool {
        self.bits & (1 << member.bit()) != 0
    }

    pub fn insert(&mut self, member: T) {
        self.bits |= 1 << member.bit();
    }

    pub fn remove(&mut self, member: T) {
        self.bits &= !(1 << member.bit());
    }

    /// This set plus `member`.
    pub fn with(mut self, member: T) -> Self {
        self.insert(member);
        self
    }

    /// This set minus `member`.
    pub fn without(mut self, member: T) -> Self {
        self.remove(member);
        self
    }

    pub fn union(self, other: Self) -> Self {
        Self { bits: self.bits | other.bits, marker: PhantomData }
    }

    pub fn intersection(self, other: Self) -> Self {
        Self { bits: self.bits & other.bits, marker: PhantomData }
    }

    /// The members in declaration order.
    pub fn iter(&self) -> impl Iterator<Item = T> + '_ {
        T::ALL.iter().copied().filter(|&m| self.contains(m))
    }
}

impl<T> Clone for EnumSet<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for EnumSet<T> {}

impl<T> Default for EnumSet<T> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<T> PartialEq for EnumSet<T> {
    fn eq(&self, other: &Self) -> bool {
        self.bits == other.bits
    }
}

impl<T> Eq for EnumSet<T> {}

impl<T: SetMember + fmt::Debug> fmt::Debug for EnumSet<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl<T: SetMember> FromIterator<T> for EnumSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        iter.into_iter().fold(Self::empty(), Self::with)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_set_contains_nothing() {
        let set = AaSet::empty();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
        assert!(Antialiasing::ALL.iter().all(|&a| !set.contains(a)));
        assert_eq!(AaSet::default(), set);
    }

    #[test]
    fn insert_remove_and_contains() {
        let mut set = UpscalerSet::empty();
        set.insert(Upscaler::Fsr1);
        set.insert(Upscaler::Dlss);
        set.insert(Upscaler::Dlss);
        assert!(set.contains(Upscaler::Fsr1) && set.contains(Upscaler::Dlss));
        assert!(!set.contains(Upscaler::Fsr3));
        assert_eq!(set.len(), 2);
        set.remove(Upscaler::Fsr1);
        set.remove(Upscaler::Fsr3);
        assert_eq!(set, UpscalerSet::of(&[Upscaler::Dlss]));
    }

    #[test]
    fn all_holds_every_variant_and_iterates_in_declaration_order() {
        let all = UpscalerSet::all();
        assert_eq!(all.len() as usize, Upscaler::ALL.len());
        assert!(all.iter().eq(Upscaler::ALL));
        let some = AaSet::of(&[Antialiasing::Taa, Antialiasing::Off]);
        assert_eq!(some.iter().collect::<Vec<_>>(), [Antialiasing::Off, Antialiasing::Taa]);
    }

    #[test]
    fn set_algebra() {
        let a = TonemapSet::of(&[Tonemap::Off]);
        let b = TonemapSet::of(&[Tonemap::Aces]);
        assert_eq!(a.union(b), TonemapSet::all());
        assert!(a.intersection(b).is_empty());
        assert_eq!(TonemapSet::all().without(Tonemap::Aces), a);
        assert_eq!(a.with(Tonemap::Aces), TonemapSet::all());
    }

    #[test]
    fn collects_from_an_iterator_and_prints_its_members() {
        let set: AaSet = [Antialiasing::Fxaa, Antialiasing::Smaa].into_iter().collect();
        assert_eq!(format!("{set:?}"), "{Fxaa, Smaa}");
    }

    #[test]
    fn every_member_has_a_distinct_bit_below_32() {
        fn check<T: SetMember>() {
            let mut seen = 0u32;
            for &m in T::ALL {
                assert!(m.bit() < 32);
                assert_eq!(seen & (1 << m.bit()), 0, "duplicate bit");
                seen |= 1 << m.bit();
            }
        }
        check::<Antialiasing>();
        check::<Upscaler>();
        check::<Tonemap>();
        check::<Setting>();
    }
}
