//! The maker of the units' lazy files: when the game reads a unit's stub it asks here, and the unit is made from the install in
//! memory (`convert`). Called on whatever thread reads the file, several at once. The install is opened on the first call, not
//! at start-up, and the last few units made are kept for a second read.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock, PoisonError};

use anyhow::{Result, anyhow};
use blackbox_modding::sdk::{Error, LazyFiles, ModResult};

use crate::convert::{Options, Stats, convert_unit};
use crate::install::Install;
use crate::manifest::{numbered, unit_of_key};

/// How many finished units are kept for a repeated read.
const KEPT: usize = 3;
/// How many conversions are remembered for the log.
const REPORTED: usize = 64;

/// A unit that was made: what it came to.
#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    pub unit: u32,
    pub stats: Stats,
}

/// Makes the files of the units.
pub struct Maker {
    source: PathBuf,
    options: Options,
    install: OnceLock<std::result::Result<Install, String>>,
    kept: Mutex<VecDeque<(u32, Vec<u8>)>>,
    reports: Mutex<VecDeque<Result<Report, String>>>,
}

impl Maker {
    pub fn new(source: PathBuf, options: Options) -> Self {
        Self {
            source,
            options,
            install: OnceLock::new(),
            kept: Mutex::new(VecDeque::new()),
            reports: Mutex::new(VecDeque::new()),
        }
    }

    fn install(&self) -> Result<&Install> {
        let opened = self.install.get_or_init(|| Install::open(&self.source).map_err(|error| format!("{error:#}")));
        opened.as_ref().map_err(|reason| anyhow!("{reason}"))
    }

    /// The file of unit `unit`: from the units kept, else made.
    pub fn make(&self, unit: u32) -> Result<Vec<u8>> {
        {
            let kept = self.kept.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some((_, bytes)) = kept.iter().find(|(number, _)| *number == unit) {
                return Ok(bytes.clone());
            }
        }
        let made = self.convert(unit);
        let report = made
            .as_ref()
            .map(|converted| Report { unit, stats: converted.stats.clone() })
            .map_err(|error| format!("unit {unit}: {error:#}"));
        self.report(report);
        let converted = made?;
        let mut kept = self.kept.lock().unwrap_or_else(PoisonError::into_inner);
        kept.push_back((unit, converted.bytes.clone()));
        while kept.len() > KEPT {
            kept.pop_front();
        }
        Ok(converted.bytes)
    }

    fn convert(&self, unit: u32) -> Result<crate::convert::Converted> {
        let install = self.install()?;
        let section = numbered(install.zones())
            .into_iter()
            .find(|(zone, _)| zone.unit == unit)
            .map(|(_, section)| section)
            .ok_or_else(|| anyhow!("unit {unit} is not in the zone list"))?;
        convert_unit(install, unit, section, &self.options)
    }

    fn report(&self, report: Result<Report, String>) {
        let mut reports = self.reports.lock().unwrap_or_else(PoisonError::into_inner);
        reports.push_back(report);
        while reports.len() > REPORTED {
            reports.pop_front();
        }
    }

    /// The conversions since the last call, oldest first: what each unit came to, or why it was not made.
    pub fn take_reports(&self) -> Vec<Result<Report, String>> {
        self.reports.lock().unwrap_or_else(PoisonError::into_inner).drain(..).collect()
    }
}

impl LazyFiles for Maker {
    fn read(&self, key: &str) -> ModResult<Vec<u8>> {
        let Some(unit) = unit_of_key(key) else { return Err(Error::Arg) };
        self.make(unit).map_err(|_| Error::Host)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_that_is_no_unit_is_refused_and_a_missing_install_is_reported_once_per_read() {
        let maker = Maker::new(
            std::env::temp_dir().join("fairhaven-no-install"),
            Options { max_texture: 512, brightness: 100 },
        );
        assert_eq!(maker.read("car/FASTCOUPE/GEOMETRY.BIN"), Err(Error::Arg));
        assert_eq!(maker.read("unit/161"), Err(Error::Host));
        let reports = maker.take_reports();
        assert!(!reports.is_empty());
        assert!(
            reports.iter().all(|report| report.as_ref().is_err_and(|reason| reason.contains("PVS.BNDL"))),
            "{reports:?}"
        );
        assert!(maker.take_reports().is_empty());
    }
}
