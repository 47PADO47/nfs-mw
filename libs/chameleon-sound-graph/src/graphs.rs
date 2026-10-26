//! Reading the sound graphs of a car bundle.
//!
//! A sound graph object is a `GenesysObject` (kind 0x15) whose import table names its class (a type import at slot
//! offset `0x8000_0000`), the classes of the items inline in it (type imports at their offsets) and the resources it
//! plays or routes to (handle imports). The offsets of the members of each class were read from the type resources
//! and are written out here: field names are hashed.

use std::collections::{HashMap, HashSet};

use chameleon_bundle::{Bundle, Resource, ResourceKind};

use crate::bytes::{f32_at, string_at, u32_at};
use crate::ginsu::GinsuResource;
use crate::sequence::{GinsuItem, Item, ItemKind, MixerItem, Sequence, WaveItem};
use crate::value::{Fade, MODULATOR_SIZE, Modulator};

const TYPE_FLAG: u32 = 0x8000_0000;
const SEQUENCE: &str = "Genesys.Gen.Sequence";
const WAVE_ITEM: &str = "Genesys.Gen.WaveSequenceItem";
const GINSU_ITEM: &str = "Genesys.Gen.GinsuSequenceItem";
const MIXER_ITEM: &str = "Genesys.Gen.MixerChannelSequenceItem";
const MIXER_VALUE: &str = "MixerChannelDoubleValue";
const MIXER_CHANNEL: &str = "Genesys.Gen.MixerChannel";
/// Most items of a sequence and most values of a mixer item.
const MAX_ITEMS: usize = 256;
/// Bytes of one value of a mixer item.
const MIXER_VALUE_SIZE: usize = 36;
/// A fade of no length.
const NO_FADE: Fade = Fade { shape: 1.0, span: 0.0 };

/// The sequences and mixer channels of one car bundle.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SoundGraphs {
    pub sequences: Vec<Sequence>,
    /// The base gain of each mixer channel, by resource id.
    pub channel_gains: HashMap<u64, f32>,
}

impl SoundGraphs {
    /// Read every sequence and mixer channel of `bundle`. A resource that does not parse is left out.
    pub fn read(bundle: &Bundle) -> Self {
        let type_kind = ResourceKind::GenesysType.id();
        let object_kind = ResourceKind::GenesysObject.id();
        let type_ids: HashSet<u64> = bundle.entries().iter().filter(|e| e.kind == type_kind).map(|e| e.id).collect();
        let names = type_names(bundle, type_kind);
        let mut graphs = Self::default();
        for (index, entry) in bundle.entries().iter().enumerate() {
            if entry.kind != object_kind || entry.import_count == 0 {
                continue;
            }
            let Ok(resource) = bundle.resource(index) else { continue };
            let Some(view) = View::new(entry.id, &resource, &type_ids, &names) else { continue };
            match view.class.as_str() {
                SEQUENCE => graphs.sequences.extend(view.sequence()),
                MIXER_CHANNEL => {
                    graphs.channel_gains.extend(f32_at(view.block, 48).map(|gain| (entry.id, gain)));
                }
                _ => {}
            }
        }
        graphs
    }

    /// The sequences whose driver contains `name`.
    pub fn driven_by<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Sequence> + 'a {
        self.sequences.iter().filter(move |sequence| sequence.driven_by(name))
    }

    /// The mixer items on `channel` among the sequences driven by `name`, with the sequence they are in.
    pub fn mixer_items<'a>(
        &'a self,
        name: &'a str,
        channel: u64,
    ) -> impl Iterator<Item = (&'a Sequence, &'a MixerItem)> + 'a {
        self.driven_by(name).flat_map(move |sequence| {
            sequence.items.iter().filter_map(move |item| match &item.kind {
                ItemKind::Mixer(mixer) if mixer.channel == channel => Some((sequence, mixer)),
                _ => None,
            })
        })
    }
}

/// The Ginsu resource `id` of `bundle`, split into its parts.
pub fn ginsu_resource(bundle: &Bundle, id: u64) -> Option<GinsuResource> {
    let resource = bundle.resource_by_id(id).ok()?;
    GinsuResource::read(&resource.blocks[0])
}

/// The names of the type resources: the zero-terminated string 8 bytes past the offset held at 0x20.
fn type_names(bundle: &Bundle, kind: u32) -> HashMap<u64, String> {
    let mut names = HashMap::new();
    for (index, entry) in bundle.entries().iter().enumerate() {
        if entry.kind != kind {
            continue;
        }
        let Ok(resource) = bundle.resource(index) else { continue };
        let main = &resource.blocks[0];
        let Some(start) = u32_at(main, 0x20).and_then(|header| (header as usize).checked_add(8)) else { continue };
        let Some(tail) = main.get(start..) else { continue };
        let Some(end) = tail.iter().position(|&byte| byte == 0) else { continue };
        let Ok(name) = std::str::from_utf8(&tail[..end]) else { continue };
        if name.len() >= 5 && name.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.')) {
            names.insert(entry.id, name.to_owned());
        }
    }
    names
}

/// One object with its imports sorted into classes (by offset) and handles (by slot offset).
struct View<'a> {
    id: u64,
    block: &'a [u8],
    class: String,
    classes: HashMap<usize, String>,
    handles: HashMap<usize, u64>,
}

impl<'a> View<'a> {
    fn new(id: u64, resource: &'a Resource, type_ids: &HashSet<u64>, names: &HashMap<u64, String>) -> Option<Self> {
        let mut classes = HashMap::new();
        let mut handles = HashMap::new();
        for import in &resource.imports {
            let at = (import.offset & !TYPE_FLAG) as usize;
            if type_ids.contains(&import.id) {
                classes.insert(at, names.get(&import.id).cloned().unwrap_or_default());
                continue;
            }
            handles.insert(at, import.id);
        }
        let class = classes.get(&0)?.clone();
        Some(Self { id, block: &resource.blocks[0], class, classes, handles })
    }

    fn handle(&self, at: usize) -> u64 {
        self.handles.get(&at).copied().unwrap_or(0)
    }

    fn sequence(&self) -> Option<Sequence> {
        let block = self.block;
        let begin = u32_at(block, 32)? as usize;
        let mut items = Vec::new();
        for n in 0..MAX_ITEMS {
            let Some(entry) = u32_at(block, begin + 4 * n).map(|entry| entry as usize) else { break };
            if entry == 0 || !self.classes.contains_key(&entry) {
                break;
            }
            items.push(self.item(entry)?);
        }
        Some(Sequence {
            id: self.id,
            driver: string_at(block, 12)?,
            hi: f32_at(block, 24)?,
            lo: f32_at(block, 28)?,
            items,
        })
    }

    fn item(&self, at: usize) -> Option<Item> {
        let block = self.block;
        let class = self.classes.get(&at)?.as_str();
        let fades = |fade_in: usize, fade_out: usize| -> Option<(Fade, Fade)> {
            Some((Fade::read(block, at + fade_in)?, Fade::read(block, at + fade_out)?))
        };
        let (fades, kind) = match class {
            WAVE_ITEM => (fades(148, 168)?, ItemKind::Wave(self.wave(at)?)),
            GINSU_ITEM => (
                fades(36, 56)?,
                ItemKind::Ginsu(GinsuItem { resource: self.handle(at + 76), mixer: self.handle(at + 84) }),
            ),
            MIXER_ITEM => ((NO_FADE, NO_FADE), ItemKind::Mixer(self.mixer(at)?)),
            other => ((NO_FADE, NO_FADE), ItemKind::Other(other.to_owned())),
        };
        Some(Item {
            hi: f32_at(block, at + 24)?,
            lo: f32_at(block, at + 28)?,
            fade_in: fades.0,
            fade_out: fades.1,
            kind,
        })
    }

    fn wave(&self, at: usize) -> Option<WaveItem> {
        let block = self.block;
        let begin = u32_at(block, at + 212)? as usize;
        let count = usize::from(*block.get(at + 226)?);
        let waves = (0..count).map(|n| self.handle(begin + 8 * n)).filter(|&id| id != 0).collect();
        Some(WaveItem {
            waves,
            mixer: self.handle(at + 204),
            gain: Modulator::read(block, at + 36)?,
            pitch: Modulator::read(block, at + 92)?,
            tracks_driver: *block.get(at + 224)? != 0,
            random: *block.get(at + 225)? != 0,
        })
    }

    fn mixer(&self, at: usize) -> Option<MixerItem> {
        let block = self.block;
        let begin = u32_at(block, at + 52)? as usize;
        let mut gains = Vec::new();
        for n in 0..MAX_ITEMS {
            let value = begin + n * MIXER_VALUE_SIZE;
            let is_value = self.classes.get(&value).is_some_and(|class| class.ends_with(MIXER_VALUE));
            if !is_value {
                break;
            }
            let targets_gain = u32_at(block, value + 12)? == 0 && u32_at(block, value + 16)? == 0;
            let pointer = u32_at(block, value + 24)? as usize;
            if targets_gain && pointer != 0 && pointer + MODULATOR_SIZE <= block.len() {
                gains.extend(Modulator::read(block, pointer));
            }
        }
        Some(MixerItem { channel: self.handle(at + 44), gains })
    }
}
