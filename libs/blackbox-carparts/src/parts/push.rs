//! Adding parts to a decoded parts database (a game or a mod that ships parts the pack does not list).

use blackbox_hash::bstring_hash;

use super::{ModelTable, Part, PartsDb};
use crate::{Error, Result};

/// A part to add with [`PartsDb::push_part`].
#[derive(Debug, Clone, Copy)]
pub struct NewPart<'a> {
    /// The car type or shared group the part belongs to.
    pub type_name_hash: u32,
    /// `CAR_PART_ID`: which kind of part (body, wheel, paint, ...).
    pub part_id: u8,
    /// The authoring name. Its `bStringHash` is the part's identity, so two parts of one database need two names.
    pub name: &'a str,
    /// 0 = stock, at most 7.
    pub upgrade_level: u8,
    /// Per level of detail (0 = A): the hash of the plain solid that draws the part, [`u32::MAX`] for none.
    pub models: &'a [u32],
    /// `(name hash, raw value)` pairs.
    pub attributes: &'a [(u32, u32)],
}

/// Room for one more entry of a table whose index is a `u16`; `u16::MAX` stands for "none" there.
const INDEX_LIMIT: usize = u16::MAX as usize;
/// The attribute lists store attribute indices as `i16`.
const ATTRIBUTE_LIMIT: usize = i16::MAX as usize;
/// The upgrade level shares a byte with the group, three bits.
const MAX_LEVEL: u8 = 7;

impl PartsDb {
    /// Add all of `news`, or none of them: when one does not fit, the database is as it was before the call. Returns the
    /// parts as stored, in order.
    pub fn push_parts(&mut self, news: &[NewPart<'_>]) -> Result<Vec<Part>> {
        let mark = self.mark();
        let mut stored = Vec::with_capacity(news.len());
        for new in news {
            match self.push_part(new) {
                Ok(part) => stored.push(part),
                Err(e) => {
                    self.rewind(&mark);
                    return Err(e);
                }
            }
        }
        Ok(stored)
    }

    /// Add `new` to the database and return the part as stored. The part comes after every part the database had, so a
    /// search that takes the first match still finds the stock parts first.
    ///
    /// The packed tables are kept consistent: the name goes into the string table, the attributes into the attribute
    /// table (a pair that is already there is shared) and the list of them, the models into a model table of their own,
    /// and the type name into the type-name table if it is new. When a table has no room for the entry (its index fields are
    /// 8 or 16 bits wide) the call fails and leaves the database as it was.
    pub fn push_part(&mut self, new: &NewPart<'_>) -> Result<Part> {
        if new.upgrade_level > MAX_LEVEL {
            return Err(Error::OutOfRange {
                what: "upgrade level",
                value: u32::from(new.upgrade_level),
                max: u32::from(MAX_LEVEL),
            });
        }
        let unit = self.string_unit.max(1);
        let name_start = self.strings.len().next_multiple_of(unit);
        let name_offset = u16_index(name_start / unit, "string")?;
        let type_index = self.type_index_for(new.type_name_hash)?;
        let model_table = u16_index(self.model_tables.len(), "model")?;

        // Attribute indices, and the pairs that are new.
        let mut added: Vec<(u32, u32)> = Vec::new();
        let mut indices = Vec::with_capacity(new.attributes.len());
        for &pair in new.attributes {
            let at = self.attributes.iter().chain(&added).position(|&known| known == pair);
            let at = at.unwrap_or_else(|| {
                added.push(pair);
                self.attributes.len() + added.len() - 1
            });
            indices.push(at);
        }
        if self.attributes.len() + added.len() > ATTRIBUTE_LIMIT {
            return Err(Error::Full("attribute"));
        }
        let list_start = self.attribute_lists.len().next_multiple_of(2);
        let attribute_list = match indices.is_empty() {
            true => u16::MAX,
            false => u16_index(list_start / 2, "attribute list")?,
        };
        let list_end = list_start / 2 + 1 + indices.len();
        if !indices.is_empty() && list_end > INDEX_LIMIT {
            return Err(Error::Full("attribute list"));
        }
        let count = i16::try_from(indices.len()).map_err(|_| Error::Full("attribute list"))?;

        // Everything fits: write.
        self.strings.resize(name_start, 0);
        self.strings.extend_from_slice(new.name.as_bytes());
        self.strings.push(0);
        self.strings.resize(self.strings.len().next_multiple_of(unit), 0);
        if type_index == self.type_names.len() {
            self.type_names.push(new.type_name_hash);
        }
        self.attributes.extend(added);
        if !indices.is_empty() {
            self.attribute_lists.resize(list_start, 0);
            self.attribute_lists.extend(count.to_le_bytes());
            for index in indices {
                self.attribute_lists.extend((index as i16).to_le_bytes());
            }
        }
        self.model_tables.push(ModelTable { templated: false, middle: None, entries: new.models.to_vec() });
        let part = Part {
            name_hash: bstring_hash(new.name),
            part_id: new.part_id,
            group: 0,
            upgrade_level: new.upgrade_level,
            base_selector: 0,
            type_index: type_index as u8,
            name_offset,
            attribute_list,
            model_table,
        };
        self.parts.push(part);
        Ok(part)
    }

    fn mark(&self) -> Mark {
        Mark {
            strings: self.strings.len(),
            attribute_lists: self.attribute_lists.len(),
            attributes: self.attributes.len(),
            model_tables: self.model_tables.len(),
            type_names: self.type_names.len(),
            parts: self.parts.len(),
        }
    }

    /// Drop what was pushed since `mark`: every table only grows, so cutting them back restores the database.
    fn rewind(&mut self, mark: &Mark) {
        self.strings.truncate(mark.strings);
        self.attribute_lists.truncate(mark.attribute_lists);
        self.attributes.truncate(mark.attributes);
        self.model_tables.truncate(mark.model_tables);
        self.type_names.truncate(mark.type_names);
        self.parts.truncate(mark.parts);
    }

    /// The index of `hash` in the type-name table, or the index it would get when added.
    fn type_index_for(&self, hash: u32) -> Result<usize> {
        if let Some(at) = self.type_names.iter().position(|&known| known == hash) {
            return Ok(at);
        }
        let next = self.type_names.len();
        if next > usize::from(u8::MAX) {
            return Err(Error::Full("type-name"));
        }
        Ok(next)
    }
}

/// The lengths of the tables, to cut them back to.
struct Mark {
    strings: usize,
    attribute_lists: usize,
    attributes: usize,
    model_tables: usize,
    type_names: usize,
    parts: usize,
}

/// `index` as a `u16` table index, which must stay below the `u16::MAX` that means "none".
fn u16_index(index: usize, table: &'static str) -> Result<u16> {
    match u16::try_from(index) {
        Ok(at) if usize::from(at) < INDEX_LIMIT => Ok(at),
        _ => Err(Error::Full(table)),
    }
}
