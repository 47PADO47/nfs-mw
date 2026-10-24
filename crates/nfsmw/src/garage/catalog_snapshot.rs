//! Shared immutable installed part catalogs across menu snapshots.
use nfsmw_data::car::customization::Catalog;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

#[derive(Clone, Debug, Default)]
pub struct CatalogSnapshot(Arc<Catalog>);

impl From<Catalog> for CatalogSnapshot {
    fn from(catalog: Catalog) -> Self {
        Self(Arc::new(catalog))
    }
}

impl Deref for CatalogSnapshot {
    type Target = Catalog;
    fn deref(&self) -> &Catalog {
        &self.0
    }
}

impl DerefMut for CatalogSnapshot {
    fn deref_mut(&mut self) -> &mut Catalog {
        Arc::make_mut(&mut self.0)
    }
}
