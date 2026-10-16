//! The mod runtime: discovers, orders and loads mods at start-up, dispatches events in load order, quarantines a mod
//! after five faults, reloads one mod in a developer run, and unloads everything in reverse order at exit.

use std::mem::size_of;
use std::path::{Path, PathBuf};
use std::ptr;
use std::rc::Rc;

use crate::abi::{BB_EVENT_CANCEL, BB_OK, BbEvent};

use super::discovery::{Candidate, discover};
use super::fault::FaultCounter;
use super::loader::{LoadError, Module};
use super::offer::HostOffer;
use super::order::resolve;
use super::policy::Policy;
use super::registry::InterfaceRegistry;
use super::reject::{RejectReason, Rejected};
use super::reload::{purge, shadow_copy};

/// How the runtime runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunMode {
    /// Mods load at start and unload at exit. Nothing is reloaded.
    Normal,
    /// A developer run (`--mod-dev`). Libraries load from shadow copies under `cache_dir`, so a rebuild can replace the
    /// original, and [`ModRuntime::reload`] is allowed.
    Dev {
        /// Folder for the shadow copies. It is purged when the runtime starts.
        cache_dir: PathBuf,
    },
}

/// One mod the runtime has loaded. A content-only mod has no module.
struct Loaded {
    candidate: Candidate,
    module: Option<Module>,
    faults: FaultCounter,
}

/// A loaded mod, for the startup log and the `mods` console command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModSummary {
    /// Mod id.
    pub id: String,
    /// Mod version.
    pub version: String,
    /// The library in the mod folder, if the mod has one.
    pub library: Option<PathBuf>,
    /// SHA-256 of the library bytes that were loaded, if the mod has a library.
    pub sha256: Option<String>,
    /// Faults counted so far.
    pub faults: u32,
    /// Whether the mod is quarantined.
    pub quarantined: bool,
}

/// What one event delivery did.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DispatchReport {
    /// Mods that were called.
    pub delivered: usize,
    /// Calls that returned a negative status.
    pub faults: usize,
    /// The first mod that asked to cancel the event, if any. Every mod still receives the event.
    pub cancelled_by: Option<String>,
}

/// Why a reload did not happen.
#[derive(Debug, thiserror::Error)]
pub enum ReloadError {
    /// The runtime was not started in a developer run.
    #[error("reload is only allowed in a developer run")]
    NotDevMode,
    /// No mod with this id is loaded.
    #[error("no mod {0} is loaded")]
    NotLoaded(String),
    /// The mod has no library, so there is nothing to reload.
    #[error("mod {0} has no library to reload")]
    ContentOnly(String),
    /// The shadow copy could not be made.
    #[error("cannot copy the library: {0}")]
    Copy(String),
    /// The new library did not load. The old one is already unloaded, so the mod stays rejected until it is fixed.
    #[error("the new library did not load: {0}")]
    Load(LoadError),
}

/// The loaded mods, the rejected ones and, in a developer run, the shadow-copy cache.
pub struct ModRuntime {
    offer: HostOffer,
    registry: Rc<InterfaceRegistry>,
    loaded: Vec<Loaded>,
    rejected: Vec<Rejected>,
    dev: Option<DevCache>,
}

struct DevCache {
    dir: PathBuf,
    serial: u32,
}

impl ModRuntime {
    /// Discover, order and load the mods in `roots`. Nothing here stops start-up: a mod that fails is recorded and
    /// logged, and the others load. With [`Policy::Off`] nothing is scanned.
    pub fn start(offer: HostOffer, policy: &Policy, roots: &[PathBuf], mode: RunMode) -> ModRuntime {
        Self::start_with(offer, InterfaceRegistry::new(), policy, roots, mode)
    }

    /// Like [`ModRuntime::start`], with the interfaces of `registry` offered to the mods. The registry's interfaces are
    /// added to the offer, so a manifest that requires one of them can load.
    pub fn start_with(
        mut offer: HostOffer,
        registry: InterfaceRegistry,
        policy: &Policy,
        roots: &[PathBuf],
        mode: RunMode,
    ) -> ModRuntime {
        offer.interfaces.extend(registry.offers());
        let dev = dev_cache(mode);
        let mut runtime =
            ModRuntime { offer, registry: Rc::new(registry), loaded: Vec::new(), rejected: Vec::new(), dev };
        if *policy == Policy::Off {
            return runtime;
        }
        let resolution = resolve(discover(roots, policy, &runtime.offer));
        for rejected in &resolution.rejected {
            log::warn!("{}", rejected.summary());
        }
        runtime.rejected = resolution.rejected;
        for candidate in resolution.ordered {
            runtime.admit(candidate);
        }
        log::info!("mods: {} loaded, {} rejected", runtime.loaded.len(), runtime.rejected.len());
        runtime
    }

    /// The loaded mods, in load order.
    pub fn loaded(&self) -> Vec<ModSummary> {
        self.loaded.iter().map(summary).collect()
    }

    /// The mods refused, with their reasons.
    pub fn rejected(&self) -> &[Rejected] {
        &self.rejected
    }

    /// Deliver one event to every loaded mod, in load order. A quarantined mod is skipped. A mod whose call returns a
    /// negative status gets one fault; the fifth quarantines it.
    pub fn dispatch(&mut self, kind: u32, frame: u64) -> DispatchReport {
        let event = BbEvent { size: size_of::<BbEvent>() as u32, kind, frame, payload: ptr::null() };
        self.dispatch_event(&event)
    }

    /// Deliver one event built by the game, payload included, to every loaded mod in load order. The payload pointer
    /// must stay valid for the call. Fault and quarantine rules are those of [`ModRuntime::dispatch`].
    pub fn dispatch_event(&mut self, event: &BbEvent) -> DispatchReport {
        let mut report = DispatchReport::default();
        for loaded in &mut self.loaded {
            if loaded.faults.is_quarantined() {
                continue;
            }
            let Some(module) = loaded.module.as_mut() else {
                continue;
            };
            let Some(status) = module.deliver(event) else {
                continue;
            };
            report.delivered += 1;
            if status == BB_EVENT_CANCEL && report.cancelled_by.is_none() {
                report.cancelled_by = Some(loaded.candidate.manifest.id.clone());
            }
            if status >= BB_OK {
                continue;
            }
            report.faults += 1;
            if loaded.faults.record() {
                log::error!("mod {} quarantined after {} faults", loaded.candidate.manifest.id, loaded.faults.faults());
            }
        }
        report
    }

    /// Reload one mod from its library, in a developer run. The old module is destroyed first; the new library is loaded
    /// from a fresh shadow copy. If it fails, the mod stays rejected.
    pub fn reload(&mut self, id: &str) -> Result<(), ReloadError> {
        let Some(cache) = self.dev.as_mut() else {
            return Err(ReloadError::NotDevMode);
        };
        let Some(index) = self.loaded.iter().position(|loaded| loaded.candidate.manifest.id == id) else {
            return Err(ReloadError::NotLoaded(id.to_owned()));
        };
        let Some(library) = self.loaded[index].candidate.library.clone() else {
            return Err(ReloadError::ContentOnly(id.to_owned()));
        };
        cache.serial += 1;
        let copy = shadow_copy(&library, &cache.dir, id, cache.serial)
            .map_err(|error| ReloadError::Copy(error.to_string()))?;

        let old = self.loaded.remove(index);
        let Loaded { candidate, module, faults: _ } = old;
        drop(module);
        match Module::load(&copy, id, &self.offer, self.registry.clone()) {
            Ok(module) => {
                self.loaded.insert(index, Loaded { candidate, module: Some(module), faults: FaultCounter::default() });
                log::info!("reloaded mod {id}");
                Ok(())
            }
            Err(error) => {
                let reason = RejectReason::LoadFailed(error.to_string());
                let rejected = Rejected::of_candidate(candidate, reason);
                log::warn!("{}", rejected.summary());
                self.rejected.push(rejected);
                Err(ReloadError::Load(error))
            }
        }
    }

    /// Destroy every loaded mod, last loaded first, then close its library. Safe to call twice.
    pub fn unload_all(&mut self) {
        while let Some(loaded) = self.loaded.pop() {
            log::info!("unloading mod {}", loaded.candidate.manifest.id);
            drop(loaded);
        }
    }

    /// Load one candidate in order. A candidate whose dependency is not loaded is refused. Its library is opened
    /// from a shadow copy in a developer run.
    fn admit(&mut self, candidate: Candidate) {
        if let Some(dependency) = self.unloaded_dependency(&candidate) {
            let reason = RejectReason::DependencyRejected { id: dependency };
            return self.refuse(candidate, reason);
        }
        let Some(library) = candidate.library.clone() else {
            return self.accept(candidate, None);
        };
        let id = candidate.manifest.id.clone();
        let file = match self.source_for(&library, &id) {
            Ok(file) => file,
            Err(reason) => return self.refuse(candidate, reason),
        };
        match Module::load(&file, &id, &self.offer, self.registry.clone()) {
            Ok(module) => self.accept(candidate, Some(module)),
            Err(error) => self.refuse(candidate, RejectReason::LoadFailed(error.to_string())),
        }
    }

    fn accept(&mut self, candidate: Candidate, module: Option<Module>) {
        match &module {
            Some(module) => log::info!(
                "loaded mod {} {} sha256={} from {}",
                candidate.manifest.id,
                candidate.manifest.version,
                module.sha256(),
                module.path().display()
            ),
            None => log::info!("loaded mod {} {} (content only)", candidate.manifest.id, candidate.manifest.version),
        }
        self.loaded.push(Loaded { candidate, module, faults: FaultCounter::default() });
    }

    fn refuse(&mut self, candidate: Candidate, reason: RejectReason) {
        let rejected = Rejected::of_candidate(candidate, reason);
        log::warn!("{}", rejected.summary());
        self.rejected.push(rejected);
    }

    /// The first dependency of `candidate` that is not loaded now.
    fn unloaded_dependency(&self, candidate: &Candidate) -> Option<String> {
        candidate
            .manifest
            .dependencies
            .iter()
            .find(|dependency| !self.loaded.iter().any(|loaded| loaded.candidate.manifest.id == dependency.id))
            .map(|dependency| dependency.id.clone())
    }

    /// The file to open for `library`: the library itself, or in a developer run a fresh shadow copy of it.
    fn source_for(&mut self, library: &Path, id: &str) -> Result<PathBuf, RejectReason> {
        let Some(cache) = self.dev.as_mut() else {
            return Ok(library.to_owned());
        };
        cache.serial += 1;
        shadow_copy(library, &cache.dir, id, cache.serial).map_err(|error| RejectReason::LoadFailed(error.to_string()))
    }
}

impl Drop for ModRuntime {
    fn drop(&mut self) {
        self.unload_all();
    }
}

/// In a developer run, purge the old cache and return the new one. A purge that fails only means a library is still
/// locked; it is logged, and the new copies get new names anyway.
fn dev_cache(mode: RunMode) -> Option<DevCache> {
    let RunMode::Dev { cache_dir } = mode else {
        return None;
    };
    if let Err(error) = purge(&cache_dir) {
        log::warn!("cannot purge the mod cache {}: {error}", cache_dir.display());
    }
    Some(DevCache { dir: cache_dir, serial: 0 })
}

fn summary(loaded: &Loaded) -> ModSummary {
    ModSummary {
        id: loaded.candidate.manifest.id.clone(),
        version: loaded.candidate.manifest.version.to_string(),
        library: loaded.candidate.library.clone(),
        sha256: loaded.module.as_ref().map(|module| module.sha256().to_owned()),
        faults: loaded.faults.faults(),
        quarantined: loaded.faults.is_quarantined(),
    }
}

#[cfg(test)]
mod tests;
