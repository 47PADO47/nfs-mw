//! The database: every loaded vault's classes, collections and types, plus a name dictionary.

mod load;
mod types;

use std::collections::HashMap;

pub use types::TypeInfo;

use crate::Result;
use crate::class::Class;
use crate::collection::{Collection, CollectionRef};
use crate::hash::vlt_hash;
use crate::names::Names;
use crate::value::RefSpec;
use crate::vault::Vault;

/// An AttribSys database built from one or more packs.
///
/// Collections can only be decoded once their class is loaded, so load the pack holding the
/// class definitions first (in NFS: Most Wanted, `attributes.bin` before `gameplay.bin` and
/// `FE_ATTRIB.bin`). Values are decoded while loading; lookups afterwards cannot fail.
#[derive(Debug, Clone, Default)]
pub struct Database {
    vaults: Vec<Vault>,
    classes: Vec<Class>,
    class_index: HashMap<u32, usize>,
    collections: Vec<Collection>,
    collection_index: HashMap<(u32, u32), usize>,
    types: Vec<TypeInfo>,
    type_index: HashMap<u32, usize>,
    names: Names,
}

impl Database {
    pub fn new() -> Self {
        Self::default()
    }

    /// A database holding one pack; see [`Self::load`].
    pub fn open(data: &[u8]) -> Result<Self> {
        let mut db = Self::new();
        db.load(data)?;
        Ok(db)
    }

    pub fn vaults(&self) -> &[Vault] {
        &self.vaults
    }

    pub fn vault(&self, name: &str) -> Option<&Vault> {
        self.vaults.iter().find(|v| v.name() == name)
    }

    pub fn classes(&self) -> &[Class] {
        &self.classes
    }

    pub fn class(&self, name: &str) -> Option<&Class> {
        self.class_by_key(vlt_hash(name))
    }

    pub fn class_by_key(&self, key: u32) -> Option<&Class> {
        self.class_index.get(&key).map(|&i| &self.classes[i])
    }

    pub fn collection_count(&self) -> usize {
        self.collections.len()
    }

    /// Every collection, in load order.
    pub fn collections(&self) -> impl Iterator<Item = CollectionRef<'_>> {
        self.collections.iter().map(|collection| CollectionRef { db: self, collection })
    }

    /// The collections of one class, in load order.
    pub fn collections_of(&self, class: &str) -> impl Iterator<Item = CollectionRef<'_>> {
        self.collections_of_key(vlt_hash(class))
    }

    pub fn collections_of_key(&self, class: u32) -> impl Iterator<Item = CollectionRef<'_>> {
        self.collections().filter(move |c| c.collection.class == class)
    }

    /// `db.collection("pvehicle", "bmwm3gtr")`.
    pub fn collection(&self, class: &str, name: &str) -> Option<CollectionRef<'_>> {
        self.collection_by_key(vlt_hash(class), vlt_hash(name))
    }

    pub fn collection_by_key(&self, class: u32, key: u32) -> Option<CollectionRef<'_>> {
        let &i = self.collection_index.get(&(class, key))?;
        Some(CollectionRef { db: self, collection: &self.collections[i] })
    }

    /// The collection a `RefSpec` points to, if it is loaded.
    pub fn resolve(&self, reference: RefSpec) -> Option<CollectionRef<'_>> {
        self.collection_by_key(reference.class, reference.collection)
    }

    /// The type table (`Attrib::DatabaseLoadData`): every type name the database knows, with its size.
    pub fn types(&self) -> &[TypeInfo] {
        &self.types
    }

    pub fn type_info(&self, type_key: u32) -> Option<&TypeInfo> {
        self.type_index.get(&type_key).map(|&i| &self.types[i])
    }

    /// Names for hashes: every string found while loading, plus whatever the caller adds.
    pub fn names(&self) -> &Names {
        &self.names
    }

    pub fn names_mut(&mut self) -> &mut Names {
        &mut self.names
    }
}
