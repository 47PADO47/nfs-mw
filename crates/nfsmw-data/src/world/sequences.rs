//! Installed world event packages and vehicle fragment event fields.
//! Spec: `docs/formats/event-sequences.md`.

use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use blackbox_attrib::vlt_hash;
use blackbox_chunk::{find_all, ids};
use blackbox_sequence::{Event, Package, Tags};
#[cfg(test)]
mod tests;

pub const TAGS: Tags = Tags { start: vlt_hash("start"), end: vlt_hash("end"), stop: vlt_hash("stop") };

#[derive(Debug, Clone)]
pub struct Sequence {
    pub package: Arc<Package>,
    pub engine: usize,
}

#[derive(Debug, Clone, Default)]
pub struct SequenceCatalog {
    engines: BTreeMap<u32, Sequence>,
}

impl SequenceCatalog {
    pub fn read(world: &[u8]) -> Result<Self> {
        let mut engines = BTreeMap::new();
        for chunk in find_all(world, ids::EVENT_SEQUENCES) {
            let bytes = chunk.aligned_payload(16).get(16..).context("event package has no runtime header")?;
            let package = Arc::new(Package::parse(bytes).context("reading world event sequence")?);
            for (engine, definition) in package.engines.iter().enumerate() {
                let key = definition.name_hash;
                if engines.insert(key, Sequence { package: package.clone(), engine }).is_some() {
                    bail!("duplicate event engine {key:08X}");
                }
            }
        }
        Ok(Self { engines })
    }

    pub fn get(&self, hash: u32) -> Option<&Sequence> {
        self.engines.get(&hash)
    }

    pub fn len(&self) -> usize {
        self.engines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.engines.is_empty()
    }
}

/// Spawn instructions, including flags that a host must not silently discard.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fragment {
    pub part: u32,
    pub collision: u32,
    pub attributes: u32,
    pub joint: u32,
    pub allow_duplicates: bool,
    pub inherit_velocity: f32,
    pub hide_parent: bool,
    pub hide_part: bool,
    pub allow_hidden_spawn: bool,
    pub no_physics: bool,
    pub simple_physics: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Explosion {
    pub position: u32,
    pub speed: f32,
    pub radius: f32,
    pub start_radius: f32,
    pub affect_this: bool,
    pub damage: bool,
    pub include_players: bool,
    pub exclude_vehicles: bool,
    pub exclude_smackables: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObjectEffect {
    pub name_tag: u32,
    /// Resolved effects collection key; zero when the package has no matching string.
    pub effect: u32,
    pub position: u32,
    pub intensity: f32,
    pub instance: u32,
    pub tracking: bool,
    pub allow_hidden_spawn: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WorldEvent {
    ObjectEffect(ObjectEffect),
    StopObjectEffect(u32),
    StopObjectEffects,
    FireList {
        list: u32,
        delayed: bool,
    },
    Fragment(Fragment),
    Stimulus {
        system: u32,
        stimulus: u32,
        queue_mode: u32,
    },
    Explosion(Explosion),
    AreaStimulus {
        system: u32,
        stimulus: u32,
        queue_mode: u32,
        radius: f32,
    },
    HideObject,
    KillObject,
    WakeObject,
    HidePart(u32),
    StopCops {
        radius: f32,
        duration: f32,
    },
    PursuitBreaker {
        begin: bool,
    },
    /// The host can report unsupported effects explicitly.
    Other(u32),
}

impl WorldEvent {
    pub fn resolve_strings(mut self, package: &Package) -> Self {
        if let Self::ObjectEffect(effect) = &mut self {
            effect.effect =
                package.strings.get(&effect.name_tag).filter(|name| !name.is_empty()).map_or(0, |name| vlt_hash(name));
        }
        self
    }
    pub fn decode(event: &Event) -> Result<Self> {
        let word = |at| -> Result<u32> {
            let bytes = event.data.get(at..at + 4).context("truncated world event")?;
            Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
        };
        let number = |at| -> Result<f32> {
            let value = f32::from_bits(word(at)?);
            if !value.is_finite() {
                bail!("non-finite world event parameter");
            }
            Ok(value)
        };
        match event.id {
            0xC1AF_4FDD => Ok(Self::ObjectEffect(ObjectEffect {
                name_tag: word(4)?,
                effect: 0,
                position: word(8)?,
                intensity: number(12)?.clamp(0.0, 1.0),
                instance: word(16)?,
                tracking: word(20)? != 0,
                allow_hidden_spawn: word(24)? != 0,
            })),
            0x3338_EFB8 => Ok(Self::StopObjectEffect(word(4)?)),
            0xC738_9F39 => Ok(Self::StopObjectEffects),
            0x7624_42F3 => Ok(Self::AreaStimulus {
                system: word(4)?,
                stimulus: word(8)?,
                queue_mode: word(12)?,
                radius: number(16)?,
            }),
            0x0713_D2FA => Ok(Self::HideObject),
            0xBD2F_6FF1 => Ok(Self::KillObject),
            0x8E60_A642 => Ok(Self::WakeObject),
            0xEFFE_CDD6 => Ok(Self::HidePart(word(4)?)),
            0xDDA7_829B => Ok(Self::StopCops { radius: number(4)?, duration: number(8)? }),
            0x47C1_D36D => Ok(Self::PursuitBreaker { begin: word(4)? != 0 }),
            0xA3CF_BFEC => {
                let _verbose = word(12)?;
                Ok(Self::FireList { list: word(4)?, delayed: word(8)? != 0 })
            }
            0x2554_C48A => Ok(Self::Fragment(Fragment {
                part: word(4)?,
                collision: word(8)?,
                attributes: word(12)?,
                joint: word(16)?,
                allow_duplicates: word(20)? != 0,
                inherit_velocity: number(24)?,
                hide_parent: word(28)? != 0,
                hide_part: word(32)? != 0,
                allow_hidden_spawn: word(36)? != 0,
                no_physics: word(40)? != 0,
                simple_physics: word(44)? != 0,
            })),
            0xE3C8_6F4C => Ok(Self::Stimulus { system: word(4)?, stimulus: word(8)?, queue_mode: word(12)? }),
            0x31BB_BAE2 => Ok(Self::Explosion(Explosion {
                position: word(4)?,
                speed: number(8)?,
                radius: number(12)?,
                start_radius: number(16)?,
                affect_this: word(20)? != 0,
                damage: word(24)? != 0,
                include_players: word(28)? != 0,
                exclude_vehicles: word(32)? != 0,
                exclude_smackables: word(36)? != 0,
            })),
            id => Ok(Self::Other(id)),
        }
    }
}
