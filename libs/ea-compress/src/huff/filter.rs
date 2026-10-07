//! Post-filters selected by the stream type (spec §8).

#[derive(Debug, Clone, Copy)]
pub(super) enum Filter {
    /// `30FB`: bytes are stored as is.
    Plain,
    /// `32FB`: bytes are stored as differences; output is their running sum.
    Delta,
    /// `34FB`: differences of differences; output is the running sum of the running sum.
    DeltaDelta,
}

impl Filter {
    pub(super) fn apply(self, out: &mut [u8]) {
        match self {
            Self::Plain => {}
            Self::Delta => {
                let mut sum = 0u8;
                for b in out {
                    sum = sum.wrapping_add(*b);
                    *b = sum;
                }
            }
            Self::DeltaDelta => {
                let (mut delta, mut sum) = (0u8, 0u8);
                for b in out {
                    delta = delta.wrapping_add(*b);
                    sum = sum.wrapping_add(delta);
                    *b = sum;
                }
            }
        }
    }
}
