//! `ABKC` sound banks: module / player / sample tables plus an embedded `BNKl` that holds the sounds.
//!
//! [`Bank::sounds`] lists every sound; [`Bank::decode`] turns one into PCM. The tables group the sounds by the
//! player that picks them at run time. Layout: `docs/specs/audio-containers.md`.

mod bnk;
mod tables;

pub use bnk::BankSound;
pub use tables::{Module, Player, SampleEntry, SampleTable, SoundKind};

use crate::bytes::u32_le;
use crate::codec::xa::{Revision, XaState};
use crate::error::{Error, Result};
use crate::pcm::Pcm;
use crate::schl::header::Codec;

/// A parsed bank borrowing its bytes.
pub struct Bank<'a> {
    data: &'a [u8],
    bnk_offset: usize,
    modules: Vec<Module>,
    tables: Vec<SampleTable>,
    sounds: Vec<BankSound>,
}

impl<'a> Bank<'a> {
    /// Parse a bank file.
    pub fn parse(data: &'a [u8]) -> Result<Self> {
        if data.get(..4) != Some(b"ABKC") {
            return Err(Error::BadMagic { expected: "ABKC" });
        }
        let (module_count, first_module) = tables::module_info(data)?;
        let bnk_offset = u32_le(data, 0x20)? as usize;
        let (modules, tables) = tables::parse(data, module_count, first_module)?;
        let sounds = match bnk_offset {
            0 => Vec::new(),
            base => bnk::parse_sounds(data, base)?,
        };
        Ok(Self { data, bnk_offset, modules, tables, sounds })
    }

    /// Every sound of the embedded `BNKl` (dummy entries left out), in table order.
    pub fn sounds(&self) -> &[BankSound] {
        &self.sounds
    }

    /// The sound at `BNKl` index `index` (the value sample-table entries carry).
    pub fn sound(&self, index: usize) -> Option<&BankSound> {
        self.sounds.iter().find(|s| s.index == index)
    }

    /// The modules and their players.
    pub fn modules(&self) -> &[Module] {
        &self.modules
    }

    /// The distinct sample tables, in order of first use.
    pub fn sample_tables(&self) -> &[SampleTable] {
        &self.tables
    }

    /// Decode the sound at `BNKl` index `index` to PCM.
    pub fn decode(&self, index: usize) -> Result<Pcm> {
        let sound = self.sound(index).ok_or(Error::NoSuchEntry(index))?;
        let h = &sound.header;
        if !matches!(h.codec, Codec::EaXa) {
            return Err(match h.codec {
                Codec::Other(id) => Error::UnsupportedCodec(id),
                _ => Error::Unsupported("bank sounds other than mono-split EA-XA"),
            });
        }
        let revision = if h.pcm_blocks { Revision::V2 } else { Revision::V1 };
        let count = h.sample_count as usize;
        if count > self.data.len().saturating_mul(2) {
            return Err(Error::Corrupt("sound claims more samples than the bank can hold"));
        }
        let channels: Vec<Vec<i16>> = h.channel_offsets[..h.channels as usize]
            .iter()
            .map(|offset| {
                let start = offset.ok_or(Error::BadHeader("bank sound without a data offset"))?;
                let start = self.bnk_offset + start as usize;
                let data = self.data.get(start..).ok_or(Error::Corrupt("sound data outside the bank"))?;
                let mut samples = Vec::with_capacity(count);
                XaState::new().decode_run(data, revision, count, &mut samples)?;
                Ok(samples)
            })
            .collect::<Result<_>>()?;
        let mut samples = Vec::with_capacity(count * channels.len());
        for i in 0..count {
            samples.extend(channels.iter().map(|c| c[i]));
        }
        Ok(Pcm { sample_rate: h.sample_rate, channels: h.channels, samples, loop_range: h.loop_range() })
    }
}
