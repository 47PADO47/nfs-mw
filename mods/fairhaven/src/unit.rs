//! One streaming unit, opened: its own bundle first and the install's shared bundles after it, so a resource is found by its id
//! wherever it lives.

use anyhow::{Context, Result};
use chameleon_bundle::{Bundle, Pool, Resource};

use crate::install::Install;

/// An opened `TRK_UNIT<n>.BNDL`.
pub struct Unit<'a> {
    install: &'a Install,
    number: u32,
    own: Pool<'static>,
    /// The type and id of each resource of the unit's own bundle, in file order.
    kinds: Vec<(u32, u64)>,
}

impl<'a> Unit<'a> {
    pub(crate) fn new(install: &'a Install, number: u32, bundle: Bundle<'static>) -> Self {
        let kinds = bundle.entries().iter().map(|entry| (entry.kind, entry.id)).collect();
        let mut own = Pool::new();
        own.add(bundle);
        Self { install, number, own, kinds }
    }

    pub fn number(&self) -> u32 {
        self.number
    }

    pub fn install(&self) -> &'a Install {
        self.install
    }

    /// The ids of the unit's own resources of type `kind`, in file order.
    pub fn ids_of(&self, kind: u32) -> Vec<u64> {
        self.kinds.iter().filter(|&&(k, _)| k == kind).map(|&(_, id)| id).collect()
    }

    /// The resource `id`, from the unit's own bundle or else the shared ones.
    pub fn resource(&self, id: u64) -> Result<Resource> {
        if self.own.contains(id) {
            return self
                .own
                .resource(id)
                .with_context(|| format!("reading resource {id:016X} of unit {}", self.number));
        }
        self.install
            .shared_resource(id)
            .with_context(|| format!("resource {id:016X} of unit {} is nowhere", self.number))
    }

    /// The type of resource `id`, wherever it is.
    pub fn kind(&self, id: u64) -> Option<u32> {
        self.own.kind(id).or_else(|| self.install.shared_kind(id))
    }
}
