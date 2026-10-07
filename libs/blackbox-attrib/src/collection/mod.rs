//! Collections: the instances of a class, with inheritance from a parent collection.

mod load;

pub(crate) use load::read_collection;

use crate::Database;
use crate::class::Class;
use crate::hash::vlt_hash;
use crate::value::{RefSpec, StringKey, Value};
use crate::vault::Vault;

/// Parent chains longer than this are treated as broken (they would be a cycle).
const MAX_DEPTH: usize = 64;

/// One value a collection stores itself.
#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    /// The field key.
    pub key: u32,
    pub value: Value,
    /// From the layout block (every collection of the class has it) rather than an entry.
    pub in_layout: bool,
}

/// A collection as stored: its own values only. Use [`CollectionRef`] (from
/// [`Database::collection`]) for lookups that follow the parent chain.
#[derive(Debug, Clone)]
pub struct Collection {
    /// Hash of the collection name (`vlt_hash("bmwm3gtr")`).
    pub key: u32,
    /// Class key.
    pub class: u32,
    /// Key of the parent collection (same class) that supplies values this one doesn't set.
    pub parent: Option<u32>,
    /// Index of the vault it came from in [`Database::vaults`].
    pub vault: usize,
    /// Layout fields first (by offset), then entries in file order.
    pub attributes: Vec<Attribute>,
}

impl Collection {
    pub fn own(&self, field_key: u32) -> Option<&Value> {
        self.attributes.iter().find(|a| a.key == field_key).map(|a| &a.value)
    }
}

/// A collection in its database: name lookups and inherited values.
#[derive(Debug, Clone, Copy)]
pub struct CollectionRef<'a> {
    pub(crate) db: &'a Database,
    pub(crate) collection: &'a Collection,
}

impl<'a> CollectionRef<'a> {
    pub fn data(&self) -> &'a Collection {
        self.collection
    }

    pub fn key(&self) -> u32 {
        self.collection.key
    }

    /// The collection's name, if the database's [`crate::Names`] know it.
    pub fn name(&self) -> Option<&'a str> {
        self.db.names().get(self.collection.key)
    }

    pub fn class(&self) -> &'a Class {
        self.db.class_by_key(self.collection.class).expect("collections are only loaded with their class")
    }

    pub fn vault(&self) -> &'a Vault {
        &self.db.vaults()[self.collection.vault]
    }

    pub fn parent(&self) -> Option<CollectionRef<'a>> {
        self.db.collection_by_key(self.collection.class, self.collection.parent?)
    }

    /// This collection, then its parent, grandparent, ... (stops at a missing parent).
    pub fn lineage(&self) -> impl Iterator<Item = CollectionRef<'a>> + use<'a> {
        std::iter::successors(Some(*self), CollectionRef::parent).take(MAX_DEPTH)
    }

    /// The values this collection stores itself.
    pub fn attributes(&self) -> &'a [Attribute] {
        &self.collection.attributes
    }

    /// The value of `field`, from this collection or the nearest ancestor that sets it.
    pub fn get(&self, field: &str) -> Option<&'a Value> {
        self.get_by_key(vlt_hash(field))
    }

    pub fn get_by_key(&self, field_key: u32) -> Option<&'a Value> {
        self.lineage().find_map(|c| c.collection.own(field_key))
    }

    /// Like [`Self::get`], without inheritance.
    pub fn get_own(&self, field: &str) -> Option<&'a Value> {
        self.collection.own(vlt_hash(field))
    }

    /// Item `index` of an array field (a scalar field is its own item 0).
    pub fn get_at(&self, field: &str, index: usize) -> Option<&'a Value> {
        self.get(field)?.item(index)
    }

    // Typed getters. They read item 0 of array fields, like AttribSys accessors without an index,
    // and return `None` when the field is unset or has another type.

    pub fn get_f32(&self, field: &str) -> Option<f32> {
        self.get_at(field, 0)?.as_f32()
    }

    pub fn get_i32(&self, field: &str) -> Option<i32> {
        self.get_at(field, 0)?.as_i32()
    }

    pub fn get_u32(&self, field: &str) -> Option<u32> {
        self.get_at(field, 0)?.as_u32()
    }

    pub fn get_bool(&self, field: &str) -> Option<bool> {
        self.get_at(field, 0)?.as_bool()
    }

    /// A `Text` or `StringKey` field's string.
    pub fn get_str(&self, field: &str) -> Option<&'a str> {
        self.get_at(field, 0)?.as_str()
    }

    pub fn get_string_key(&self, field: &str) -> Option<&'a StringKey> {
        self.get_at(field, 0)?.as_string_key()
    }

    pub fn get_ref(&self, field: &str) -> Option<RefSpec> {
        self.get_at(field, 0)?.as_ref_spec()
    }

    pub fn get_vector2(&self, field: &str) -> Option<[f32; 2]> {
        self.get_at(field, 0)?.as_vector2()
    }

    pub fn get_vector3(&self, field: &str) -> Option<[f32; 3]> {
        self.get_at(field, 0)?.as_vector3()
    }

    pub fn get_vector4(&self, field: &str) -> Option<[f32; 4]> {
        self.get_at(field, 0)?.as_vector4()
    }

    /// The collection a `RefSpec` field (item 0) refers to.
    pub fn follow(&self, field: &str) -> Option<CollectionRef<'a>> {
        self.db.resolve(self.get_ref(field)?)
    }
}
