//! `bb.content`: a mod tells the game that the files and manifests in its folder have changed since start-up.
//!
//! A mod that generates its content on a worker thread (the NFS Most Wanted 2012 cars) finishes after the game has
//! started. The call adds the mod's `files/` as a layer of the install and registers its new cars, both of which reach
//! the scenes the game has already made (the layer is shared by every copy of the install, the car list is read from
//! the car tables each time one is built). Everything the game opens afterwards, the garage and the career among it,
//! lists the cars.

use std::rc::Rc;

use blackbox_modding::abi::{BB_ERR_HOST, BB_ERR_NOT_FOUND, BB_ERR_NOT_READY};
use blackbox_modding::host::{BackendResult, ContentBackend};

use super::bridge::Bridge;
use crate::modding::content;

/// The `bb.content` port of the game: one per mod table, sharing the bridge.
pub struct ContentPort(pub(super) Rc<Bridge>);

impl ContentBackend for ContentPort {
    fn rescan(&self, owner: &str) -> BackendResult<()> {
        let Some(install) = self.0.install() else { return Err(BB_ERR_NOT_READY) };
        match content::rescan(&install, owner) {
            Ok(added) => {
                log::info!("mod {owner}: content rescanned, {added} car(s) added");
                Ok(())
            }
            Err(content::RescanError::NotLoaded) => Err(BB_ERR_NOT_FOUND),
            Err(content::RescanError::Layer(reason)) => {
                log::warn!("mod {owner}: its files are not added: {reason}");
                Err(BB_ERR_HOST)
            }
        }
    }
}
