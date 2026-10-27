use std::collections::HashMap;

use crate::bundle::{Bundle, Resource};
use crate::error::{Error, Result};

/// Several bundles searched together: resources import each other across bundle files, so an id is looked up over the
/// whole set. The first bundle added that holds an id wins, so add the preferred bundle of a duplicated id first.
#[derive(Debug, Default)]
pub struct Pool<'a> {
    bundles: Vec<Bundle<'a>>,
    index: HashMap<u64, (usize, usize)>,
}

impl<'a> Pool<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, bundle: Bundle<'a>) {
        let at = self.bundles.len();
        for (position, entry) in bundle.entries().iter().enumerate() {
            self.index.entry(entry.id).or_insert((at, position));
        }
        self.bundles.push(bundle);
    }

    pub fn contains(&self, id: u64) -> bool {
        self.index.contains_key(&id)
    }

    /// The resource type id of `id`.
    pub fn kind(&self, id: u64) -> Option<u32> {
        let &(bundle, position) = self.index.get(&id)?;
        Some(self.bundles[bundle].entries()[position].kind)
    }

    /// The resource with id `id`, inflated.
    pub fn resource(&self, id: u64) -> Result<Resource> {
        let &(bundle, position) = self.index.get(&id).ok_or(Error::NoSuchResource { id })?;
        self.bundles[bundle].resource(position)
    }
}
