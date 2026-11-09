//! `bb.content`: a mod tells the game that the files and manifests in its folder have changed since start-up, and gives it
//! the function that makes its lazy files.
//!
//! A mod that generates its content on a worker thread (the NFS Most Wanted 2012 cars) finishes after the game has
//! started. The call adds the mod's `files/` as a layer of the install and registers its new cars, both of which reach
//! the scenes the game has already made (the layer is shared by every copy of the install, the car list is read from
//! the car tables each time one is built). Everything the game opens afterwards, the garage and the career among it,
//! lists the cars.
//!
//! A mod that makes some of its files only when they are read (the same cars, converted at first use) writes a stub for each in
//! its layer and gives the game the maker: `GameDir::read` asks it whenever it reads a stub (`game_install::LazySource`).

use std::io;
use std::rc::Rc;
use std::sync::Arc;

use blackbox_modding::abi::{BB_ERR_HOST, BB_ERR_NOT_FOUND, BB_ERR_NOT_READY};
use blackbox_modding::host::{BackendResult, ContentBackend, LazyFiles};
use game_install::LazySource;

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

    fn provide_lazy_files(&self, owner: &str, files: Arc<dyn LazyFiles>) -> BackendResult<()> {
        self.0.add_lazy_source(owner, Arc::new(ModFiles { owner: owner.to_owned(), files }));
        log::info!("mod {owner}: it makes some of its files when they are read");
        Ok(())
    }

    fn capabilities(&self) -> u32 {
        use blackbox_modding::abi::{BB_CONTENT_CAP_LAZY_FILES, BB_CONTENT_CAP_REPLACE_SETS, BB_CONTENT_CAP_TEXTURES};
        // World patches and alternative maps are parsed and merged by the library; the game applies them when the world opens
        // (`worlds.rs`): their bits are set with that work.
        BB_CONTENT_CAP_LAZY_FILES | BB_CONTENT_CAP_REPLACE_SETS | BB_CONTENT_CAP_TEXTURES | content::WORLD_CAPABILITIES
    }

    fn report(&self, owner: &str) -> BackendResult<String> {
        Ok(self.0.replacement_report(owner))
    }
}

/// A mod's maker of lazy files, as the install's reads see it.
struct ModFiles {
    owner: String,
    files: Arc<dyn LazyFiles>,
}

impl LazySource for ModFiles {
    fn read(&self, key: &str) -> io::Result<Vec<u8>> {
        self.files.read(key).map_err(|status| {
            io::Error::other(format!("mod {}: its file {key} could not be made (status {status})", self.owner))
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use blackbox_modding::abi::BB_ERR_STALE;

    use super::*;

    /// A maker that repeats the key, or says the mod is gone.
    struct Repeat(Mutex<bool>);

    impl LazyFiles for Repeat {
        fn read(&self, key: &str) -> BackendResult<Vec<u8>> {
            if *self.0.lock().unwrap() {
                return Err(BB_ERR_STALE);
            }
            Ok(key.repeat(2).into_bytes())
        }
    }

    fn install_with_stub(name: &str) -> (std::path::PathBuf, game_install::GameDir) {
        let root = std::env::temp_dir().join(format!("nfsmw-lazy-port-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("install/GLOBAL")).unwrap();
        std::fs::write(root.join("install/GLOBAL/X.BIN"), b"x").unwrap();
        std::fs::create_dir_all(root.join("mod/CARS/NEW")).unwrap();
        std::fs::write(root.join("mod/CARS/NEW/GEOMETRY.BIN"), game_install::lazy_stub("example.cars", "ab")).unwrap();
        let mut dir = game_install::GameDir::open(root.join("install")).unwrap();
        dir.add_overlay("example.cars", root.join("mod"), 0).unwrap();
        (root, dir)
    }

    #[test]
    fn a_maker_given_before_the_install_serves_it_once_it_is_known() {
        let (root, dir) = install_with_stub("pending");
        let bridge = Rc::new(Bridge::default());
        let port = ContentPort(bridge.clone());
        port.provide_lazy_files("example.cars", Arc::new(Repeat(Mutex::new(false)))).unwrap();
        assert!(dir.read("CARS/NEW/GEOMETRY.BIN").is_err(), "no maker is on the install yet");
        bridge.set_install(dir.clone());
        assert_eq!(dir.read("CARS/NEW/GEOMETRY.BIN").unwrap(), b"abab");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_maker_given_after_the_install_goes_straight_to_it_and_its_failure_is_an_error() {
        let (root, dir) = install_with_stub("late");
        let bridge = Rc::new(Bridge::default());
        bridge.set_install(dir.clone());
        let port = ContentPort(bridge);
        port.provide_lazy_files("example.cars", Arc::new(Repeat(Mutex::new(true)))).unwrap();
        let error = dir.read("CARS/NEW/GEOMETRY.BIN").unwrap_err();
        assert!(error.to_string().contains("example.cars"), "{error}");
        std::fs::remove_dir_all(root).unwrap();
    }
}
