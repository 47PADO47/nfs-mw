use super::{BitError, BitReader, BitWriter};

/// Writes strictly increasing `u16` ids as gaps. The first id is a 7-bit varint; each later id is the gap to the
/// previous one minus one, as a 3-bit varint. Consecutive ids cost 4 bits.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct IdGapEncoder {
    prev: Option<u16>,
}

impl IdGapEncoder {
    /// An encoder with no previous id.
    pub const fn new() -> Self {
        Self { prev: None }
    }

    /// Writes `id`. An id not above the previous one is [`BitError::BadValue`] and writes nothing.
    pub fn write(&mut self, w: &mut BitWriter<'_>, id: u16) -> Result<(), BitError> {
        match self.prev {
            None => w.write_varint_n(u64::from(id), 7)?,
            Some(prev) => {
                if id <= prev {
                    return Err(BitError::BadValue);
                }
                w.write_varint_n(u64::from(id - prev - 1), 3)?;
            }
        }
        self.prev = Some(id);
        Ok(())
    }
}

/// Reads ids written by [`IdGapEncoder`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct IdGapDecoder {
    prev: Option<u16>,
}

impl IdGapDecoder {
    /// A decoder with no previous id.
    pub const fn new() -> Self {
        Self { prev: None }
    }

    /// Reads the next id. A sum past `u16::MAX` is [`BitError::BadValue`].
    pub fn read(&mut self, r: &mut BitReader<'_>) -> Result<u16, BitError> {
        let wide = match self.prev {
            None => r.read_varint_n(7)?,
            Some(prev) => {
                let gap = r.read_varint_n(3)?;
                u64::from(prev).checked_add(1).and_then(|base| base.checked_add(gap)).ok_or(BitError::BadValue)?
            }
        };
        let id = u16::try_from(wide).map_err(|_| BitError::BadValue)?;
        self.prev = Some(id);
        Ok(id)
    }
}
