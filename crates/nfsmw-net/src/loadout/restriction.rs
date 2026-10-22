//! The session's car restriction (MP-31): the `allowed_cars` field of the settings struct.

use crate::message::{DecodeError, List, Reader, Wire, Writer};

/// Most car ids in an allowed list.
pub const MAX_ALLOWED_CARS: usize = 64;

/// Which cars a session accepts.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum CarRestriction {
    /// Any known car.
    #[default]
    Any,
    /// Only these car types.
    Cars(List<u32, MAX_ALLOWED_CARS>),
    /// Only cars of this class.
    Class(u8),
    /// Only loadouts whose performance rating is at most this.
    PerformanceCap(u16),
}

impl Wire for CarRestriction {
    fn put(&self, w: &mut Writer<'_>) {
        match self {
            Self::Any => w.varint(0),
            Self::Cars(cars) => {
                w.varint(1);
                cars.put(w);
            }
            Self::Class(class) => {
                w.varint(2);
                class.put(w);
            }
            Self::PerformanceCap(cap) => {
                w.varint(3);
                cap.put(w);
            }
        }
    }

    fn take(r: &mut Reader<'_>) -> Result<Self, DecodeError> {
        match r.varint(3).map_err(|_| DecodeError::BadEnum)? {
            0 => Ok(Self::Any),
            1 => Ok(Self::Cars(List::take(r)?)),
            2 => Ok(Self::Class(u8::take(r)?)),
            _ => Ok(Self::PerformanceCap(u16::take(r)?)),
        }
    }
}
