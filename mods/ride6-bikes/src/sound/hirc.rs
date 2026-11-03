//! The object hierarchy (`HIRC` chunk) of a Wwise sound bank, as far as the importer needs it: which recordings an event
//! plays, and the parameters of a source plugin object.
//!
//! The layout of a `HIRC` chunk (bank version 154, `docs/formats/ride6-audio.md`): `count u32`, then `count` objects of
//! `kind u8`, `size u32`, and a body of `size` bytes whose first four bytes are the object's id. Only these kinds are read:
//!
//! | Kind | Body (after the id) |
//! |---|---|
//! | 2 sound | `plugin u32`, `stream u8`, `source id u32`, then the node parameters (not parsed) |
//! | 3 action | `type u16` (`0x04xx` plays), `target id u32` |
//! | 4 event | `count` (a 7-bit variable length number), `count` action ids |
//! | 17 effect/source plugin | `plugin u32`, `size u32`, the parameter block, the media list, the game parameter curves |
//!
//! The node parameters of a sound or container carry its parent id before its children are listed, but their layout
//! changes from bank version to bank version and is not parsed. The parent and children are found instead by the
//! references that objects hold to each other: 32-bit ids of objects of the same bank are unique and a stray match of four
//! bytes with one of a few hundred ids is practically impossible; an object's **parent** is the first container it
//! references, and a container's **children** are the objects that reference it as their parent.

use std::collections::HashMap;

use anyhow::{Result, bail, ensure};

pub const KIND_SOUND: u8 = 2;
pub const KIND_ACTION: u8 = 3;
pub const KIND_EVENT: u8 = 4;
/// Containers: random/sequence (5), switch (6), actor-mixer (7), layer (blend) container (9).
pub const CONTAINERS: [u8; 4] = [5, 6, 7, 9];
pub const KIND_FX: u8 = 17;

/// The low nibble of a sound's plugin number says what it plays: 1 is a codec (the source is a media file), anything else a
/// source plugin (the source is a plugin object of the bank).
pub fn is_codec(plugin: u32) -> bool {
    plugin & 0xF == 1
}

/// One object: its kind and its body (the id included, as the first four bytes).
#[derive(Debug, Clone, Copy)]
pub struct Object<'a> {
    pub kind: u8,
    pub id: u32,
    pub body: &'a [u8],
}

/// A sound played by an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoundRef {
    pub id: u32,
    /// The plugin number: a codec ([`is_codec`]) or a source plugin.
    pub plugin: u32,
    /// The media id for a codec; the id of the plugin object for a source plugin.
    pub source: u32,
}

/// A game parameter curve of a plugin object: the game parameter `game_parameter` drives the plugin parameter `parameter`
/// through `points` (`x`, `y`, interpolation).
#[derive(Debug, Clone, PartialEq)]
pub struct ParameterCurve {
    pub game_parameter: u32,
    pub parameter: u8,
    pub points: Vec<(f32, f32, u32)>,
}

/// A source plugin object.
#[derive(Debug, Clone, PartialEq)]
pub struct PluginObject {
    pub plugin: u32,
    /// The plugin's own parameter block.
    pub params: Vec<u8>,
    /// The media files the plugin reads.
    pub media: Vec<u32>,
    pub curves: Vec<ParameterCurve>,
}

/// The parsed chunk.
#[derive(Debug)]
pub struct Hirc<'a> {
    objects: Vec<Object<'a>>,
    index: HashMap<u32, usize>,
}

fn word(bytes: &[u8], at: usize) -> Result<u32> {
    let Some(slice) = bytes.get(at..at + 4) else { bail!("an object ends inside a field") };
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn float(bytes: &[u8], at: usize) -> Result<f32> {
    Ok(f32::from_bits(word(bytes, at)?))
}

fn short(bytes: &[u8], at: usize) -> Result<u16> {
    let Some(slice) = bytes.get(at..at + 2) else { bail!("an object ends inside a field") };
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

/// A 7-bit variable length number: returns the value and the bytes it took.
fn varint(bytes: &[u8], at: usize) -> Result<(u32, usize)> {
    let mut value = 0u32;
    for (taken, byte) in bytes.iter().skip(at).take(5).enumerate() {
        value = (value << 7) | u32::from(byte & 0x7F);
        if byte & 0x80 == 0 {
            return Ok((value, taken + 1));
        }
    }
    bail!("a variable length number is cut short")
}

impl<'a> Hirc<'a> {
    /// Parse the body of a `HIRC` chunk.
    pub fn parse(chunk: &'a [u8]) -> Result<Self> {
        let count = word(chunk, 0)? as usize;
        let mut at = 4;
        let mut objects = Vec::with_capacity(count.min(1 << 16));
        for _ in 0..count {
            ensure!(at < chunk.len(), "the object list ends early");
            let kind = chunk[at];
            let size = word(chunk, at + 1)? as usize;
            let Some(body) = chunk.get(at + 5..at + 5 + size) else {
                bail!("an object runs past the end of the chunk")
            };
            ensure!(size >= 4, "an object is smaller than its id");
            objects.push(Object { kind, id: word(body, 0)?, body });
            at += 5 + size;
        }
        let index = objects.iter().enumerate().map(|(position, object)| (object.id, position)).collect();
        Ok(Self { objects, index })
    }

    pub fn len(&self) -> usize {
        self.objects.len()
    }

    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    pub fn get(&self, id: u32) -> Option<&Object<'a>> {
        self.index.get(&id).map(|&position| &self.objects[position])
    }

    /// Ids of other objects of the bank that `id`'s body mentions, with the byte offset, in file order.
    fn references(&self, id: u32) -> Vec<(usize, u32)> {
        let Some(object) = self.get(id) else { return Vec::new() };
        let body = object.body;
        let mut found = Vec::new();
        for at in 4..body.len().saturating_sub(3) {
            let value = u32::from_le_bytes([body[at], body[at + 1], body[at + 2], body[at + 3]]);
            if value != id && self.index.contains_key(&value) {
                found.push((at, value));
            }
        }
        found
    }

    fn is_container(&self, id: u32) -> bool {
        self.get(id).is_some_and(|object| CONTAINERS.contains(&object.kind))
    }

    /// The first container `id` references: its parent in the hierarchy.
    pub fn parent(&self, id: u32) -> Option<u32> {
        self.references(id).into_iter().map(|(_, value)| value).find(|&value| self.is_container(value))
    }

    /// The sounds and containers whose parent is `id`, in the order `id` lists them.
    pub fn children(&self, id: u32) -> Vec<u32> {
        let mut out = Vec::new();
        for (_, value) in self.references(id) {
            let kind_ok =
                self.get(value).is_some_and(|object| object.kind == KIND_SOUND || CONTAINERS.contains(&object.kind));
            if kind_ok && !out.contains(&value) && self.parent(value) == Some(id) {
                out.push(value);
            }
        }
        out
    }

    /// Every sound below `id` (or `id` itself when it is a sound), each once, in the order of the hierarchy.
    pub fn sounds_below(&self, id: u32) -> Vec<SoundRef> {
        let mut out: Vec<SoundRef> = Vec::new();
        let mut stack = vec![id];
        let mut seen = Vec::new();
        while let Some(next) = stack.pop() {
            if seen.contains(&next) {
                continue;
            }
            seen.push(next);
            let Some(object) = self.get(next) else { continue };
            if object.kind == KIND_SOUND {
                if let Ok(sound) = self.sound(next) {
                    out.push(sound);
                }
                continue;
            }
            stack.extend(self.children(next).into_iter().rev());
        }
        out
    }

    /// A sound object's plugin and source.
    pub fn sound(&self, id: u32) -> Result<SoundRef> {
        let Some(object) = self.get(id).filter(|object| object.kind == KIND_SOUND) else {
            bail!("object {id} is not a sound")
        };
        Ok(SoundRef { id, plugin: word(object.body, 4)?, source: word(object.body, 9)? })
    }

    /// The ids of the actions of an event.
    pub fn event_actions(&self, event: u32) -> Result<Vec<u32>> {
        let Some(object) = self.get(event).filter(|object| object.kind == KIND_EVENT) else {
            bail!("event {event} is not in this bank")
        };
        let (count, taken) = varint(object.body, 4)?;
        (0..count as usize).map(|n| word(object.body, 4 + taken + 4 * n)).collect()
    }

    /// The sounds the play actions of an event start (actions of other kinds, such as setting a game parameter, are
    /// skipped; a play action whose target is in another bank adds nothing).
    pub fn event_sounds(&self, event: u32) -> Result<Vec<SoundRef>> {
        let mut out: Vec<SoundRef> = Vec::new();
        for action in self.event_actions(event)? {
            let Some(object) = self.get(action).filter(|object| object.kind == KIND_ACTION) else { continue };
            if short(object.body, 4)? & 0xFF00 != 0x0400 {
                continue;
            }
            let target = word(object.body, 6)?;
            for sound in self.sounds_below(target) {
                if !out.contains(&sound) {
                    out.push(sound);
                }
            }
        }
        Ok(out)
    }

    /// Read a source plugin object (kind 17).
    pub fn plugin_object(&self, id: u32) -> Result<PluginObject> {
        let Some(object) = self.get(id).filter(|object| object.kind == KIND_FX) else {
            bail!("object {id} is not a plugin")
        };
        let body = object.body;
        let (plugin, size) = (word(body, 4)?, word(body, 8)? as usize);
        let Some(params) = body.get(12..12 + size) else {
            bail!("plugin {id}: the parameter block runs past the object")
        };
        let mut at = 12 + size;
        let media_count = *body.get(at).unwrap_or(&0) as usize;
        at += 1;
        let mut media = Vec::new();
        for _ in 0..media_count {
            media.push(word(body, at + 1)?);
            at += 5;
        }
        let mut curves = Vec::new();
        let curve_count = short(body, at)?;
        let mut cursor = at + 2;
        for _ in 0..curve_count {
            // game parameter id u32, type u8, accumulation u8, plugin parameter u8, curve id u32, scaling u8, points u16
            let (game_parameter, parameter) = (word(body, cursor)?, *body.get(cursor + 6).unwrap_or(&0));
            let points = usize::from(short(body, cursor + 12)?);
            ensure!(points <= 512, "plugin {id}: a curve of {points} points");
            let mut list = Vec::with_capacity(points);
            for point in 0..points {
                let p = cursor + 14 + 12 * point;
                list.push((float(body, p)?, float(body, p + 4)?, word(body, p + 8)?));
            }
            curves.push(ParameterCurve { game_parameter, parameter, points: list });
            cursor += 14 + 12 * points;
        }
        Ok(PluginObject { plugin, params: params.to_vec(), media, curves })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn object(kind: u8, body: &[u8]) -> Vec<u8> {
        let mut out = vec![kind];
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(body);
        out
    }

    fn le(values: &[u32]) -> Vec<u8> {
        values.iter().flat_map(|value| value.to_le_bytes()).collect()
    }

    /// event 100 -> play action 200 -> mixer 300 -> sounds 400 (codec, media 7) and 401 (source plugin 500); a set-parameter
    /// action 201 plays nothing.
    pub(crate) fn chunk() -> Vec<u8> {
        let mut plugin_body = le(&[500, 0x01A0_1052, 8]);
        plugin_body.extend_from_slice(&[9, 8, 7, 6, 5, 4, 3, 2]);
        plugin_body.push(1);
        plugin_body.push(0);
        plugin_body.extend_from_slice(&le(&[7777]));
        plugin_body.extend_from_slice(&1u16.to_le_bytes());
        plugin_body.extend_from_slice(&le(&[55]));
        plugin_body.extend_from_slice(&[0, 1, 3]);
        plugin_body.extend_from_slice(&le(&[66]));
        plugin_body.push(0);
        plugin_body.extend_from_slice(&2u16.to_le_bytes());
        for (x, y, interpolation) in [(0.0f32, 1.0f32, 9u32), (1.0, 6.0, 4)] {
            plugin_body.extend_from_slice(&x.to_le_bytes());
            plugin_body.extend_from_slice(&y.to_le_bytes());
            plugin_body.extend_from_slice(&interpolation.to_le_bytes());
        }
        let mut sound_a = le(&[400, 0x0004_0001]);
        sound_a.push(0);
        sound_a.extend_from_slice(&le(&[7, 0, 300]));
        let mut sound_b = le(&[401, 0x01A0_1052]);
        sound_b.push(0);
        sound_b.extend_from_slice(&le(&[500, 300]));
        let objects = [
            object(KIND_EVENT, &[&le(&[100])[..], &[2], &le(&[200, 201])].concat()),
            object(KIND_ACTION, &[&le(&[200])[..], &0x0403u16.to_le_bytes(), &le(&[300])].concat()),
            object(KIND_ACTION, &[&le(&[201])[..], &0x1303u16.to_le_bytes(), &le(&[300])].concat()),
            object(7, &le(&[300, 0, 400, 401])),
            object(KIND_SOUND, &sound_a),
            object(KIND_SOUND, &sound_b),
            object(KIND_FX, &plugin_body),
        ];
        let mut chunk = (objects.len() as u32).to_le_bytes().to_vec();
        chunk.extend(objects.concat());
        chunk
    }

    #[test]
    fn an_event_finds_the_sounds_below_its_play_actions() {
        let bytes = chunk();
        let hirc = Hirc::parse(&bytes).unwrap();
        assert_eq!(hirc.len(), 7);
        assert_eq!(hirc.event_actions(100).unwrap(), [200, 201]);
        let sounds = hirc.event_sounds(100).unwrap();
        assert_eq!(sounds.len(), 2);
        assert_eq!(sounds[0], SoundRef { id: 400, plugin: 0x0004_0001, source: 7 });
        assert!(is_codec(sounds[0].plugin));
        assert_eq!(sounds[1], SoundRef { id: 401, plugin: 0x01A0_1052, source: 500 });
        assert!(!is_codec(sounds[1].plugin));
        assert_eq!(hirc.parent(400), Some(300));
        assert_eq!(hirc.children(300), [400, 401]);
        assert!(hirc.event_sounds(999).is_err());
    }

    #[test]
    fn a_plugin_object_gives_its_parameters_media_and_curves() {
        let bytes = chunk();
        let hirc = Hirc::parse(&bytes).unwrap();
        let plugin = hirc.plugin_object(500).unwrap();
        assert_eq!(plugin.plugin, 0x01A0_1052);
        assert_eq!(plugin.params, [9, 8, 7, 6, 5, 4, 3, 2]);
        assert_eq!(plugin.media, [7777]);
        assert_eq!(plugin.curves.len(), 1);
        assert_eq!((plugin.curves[0].game_parameter, plugin.curves[0].parameter), (55, 3));
        assert_eq!(plugin.curves[0].points, [(0.0, 1.0, 9), (1.0, 6.0, 4)]);
        assert!(hirc.plugin_object(300).is_err());
    }

    #[test]
    fn a_broken_chunk_is_refused_not_a_panic() {
        assert!(Hirc::parse(&[1, 0, 0, 0]).is_err());
        let mut cut = chunk();
        cut.truncate(40);
        assert!(Hirc::parse(&cut).is_err());
        assert!(Hirc::parse(&[]).is_err());
        let mut tiny = vec![1, 0, 0, 0, KIND_SOUND];
        tiny.extend_from_slice(&2u32.to_le_bytes());
        tiny.extend_from_slice(&[0, 0]);
        assert!(Hirc::parse(&tiny).is_err());
    }

    #[test]
    fn variable_length_numbers_are_read() {
        assert_eq!(varint(&[0x05], 0).unwrap(), (5, 1));
        assert_eq!(varint(&[0x81, 0x00], 0).unwrap(), (128, 2));
        assert!(varint(&[0x80, 0x80], 0).is_err());
    }
}
