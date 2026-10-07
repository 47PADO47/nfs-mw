//! Adding packs and vaults to a [`Database`].
//!
//! Loading is all or nothing: everything is parsed and decoded first, then committed, so a
//! failed load leaves the database as it was.

use std::collections::HashMap;

use super::{Database, TypeInfo, types::read_types};
use crate::class::{Class, read_class};
use crate::collection::{Collection, read_collection};
use crate::hash::vlt_hash;
use crate::pack::read_pack;
use crate::value::Value;
use crate::vault::Vault;
use crate::{Error, Result};

const CLASS_LOAD: u32 = vlt_hash("Attrib::ClassLoadData");
const COLLECTION_LOAD: u32 = vlt_hash("Attrib::CollectionLoadData");
const DATABASE_LOAD: u32 = vlt_hash("Attrib::DatabaseLoadData");

impl Database {
    /// Adds every vault of a `VPAK` pack. The pack may be wrapped in `RAWW`, `JDLZ` or `HUFF`
    /// (`gameplay.lzc`). Returns the number of vaults added.
    pub fn load(&mut self, data: &[u8]) -> Result<usize> {
        let data = ea_compress::unwrap(data)?;
        let vaults = read_pack(&data)?
            .iter()
            .map(|entry| Vault::parse(entry.name, entry.vlt, entry.bin))
            .collect::<Result<Vec<_>>>()?;
        let count = vaults.len();
        self.add_vaults(vaults)?;
        Ok(count)
    }

    /// Adds one vault given as a bare `.vlt` + `.bin` pair (outside a pack).
    pub fn load_vault(&mut self, name: &str, vlt: &[u8], bin: &[u8]) -> Result<()> {
        self.add_vaults(vec![Vault::parse(name, vlt, bin)?])
    }

    fn add_vaults(&mut self, vaults: Vec<Vault>) -> Result<()> {
        let first = self.vaults.len();

        // Classes and types first: a vault's collections may use classes of a later vault.
        let mut classes: Vec<Class> = Vec::new();
        let mut class_index = HashMap::new();
        let mut types: Vec<TypeInfo> = Vec::new();
        for (i, vault) in vaults.iter().enumerate() {
            for export in vault.exports() {
                match export.kind {
                    CLASS_LOAD => {
                        let class = read_class(vault, export, first + i)?;
                        if self.class_index.contains_key(&class.key) || class_index.contains_key(&class.key) {
                            return Err(duplicate(vault, "class", class.key));
                        }
                        class_index.insert(class.key, classes.len());
                        classes.push(class);
                    }
                    DATABASE_LOAD => types.extend(read_types(vault, export)?),
                    _ => {}
                }
            }
        }

        let class_of = |key: u32| class_index.get(&key).map(|&i| &classes[i]).or_else(|| self.class_by_key(key));
        let mut collections: Vec<Collection> = Vec::new();
        let mut collection_index = HashMap::new();
        for (i, vault) in vaults.iter().enumerate() {
            for export in vault.exports().iter().filter(|e| e.kind == COLLECTION_LOAD) {
                let collection = read_collection(vault, export, first + i, class_of)?;
                let id = (collection.class, collection.key);
                if self.collection_index.contains_key(&id) || collection_index.contains_key(&id) {
                    return Err(duplicate(vault, "collection", collection.key));
                }
                collection_index.insert(id, collections.len());
                collections.push(collection);
            }
        }

        // Commit.
        for vault in &vaults {
            self.names.insert(vault.name());
            self.names.extend(vault.dependencies().iter().map(|d| &d.name));
            self.names.extend(vault.strings());
        }
        for collection in &collections {
            for attribute in &collection.attributes {
                add_value_names(&mut self.names, &attribute.value);
            }
        }
        for info in types {
            self.names.insert(&info.name);
            self.type_index.entry(info.key).or_insert(self.types.len());
            self.types.push(info);
        }
        let class_base = self.classes.len();
        self.class_index.extend(class_index.into_iter().map(|(key, i)| (key, class_base + i)));
        self.classes.extend(classes);
        let collection_base = self.collections.len();
        self.collection_index.extend(collection_index.into_iter().map(|(id, i)| (id, collection_base + i)));
        self.collections.extend(collections);
        self.vaults.extend(vaults);
        Ok(())
    }
}

fn duplicate(vault: &Vault, what: &'static str, key: u32) -> Error {
    Error::Duplicate { vault: vault.name().to_owned(), what, key }
}

fn add_value_names(names: &mut crate::Names, value: &Value) {
    match value {
        Value::Text(Some(s)) => {
            names.insert(s);
        }
        Value::StringKey(k) => {
            if let Some(s) = &k.string {
                names.insert(s);
            }
        }
        Value::Array(items) => items.iter().for_each(|v| add_value_names(names, v)),
        _ => {}
    }
}
