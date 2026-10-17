//! Fake backends: they record what the trampolines pass in, and answer with fixed values.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use blackbox_modding::abi::BB_ERR_UNSUPPORTED;

use crate::abi::{BbHeatRow, BbMilestoneDef, BbName, BbOwnedCar, BbRaceDef, BbRaceState, BbVec3, CopTable, EndMode};
use crate::host::BackendResult;
use crate::host::career::CareerBackend;
use crate::host::pursuit::{HeatRowSpec, PursuitBackend};
use crate::host::races::RacesBackend;

/// A pursuit backend that keeps the heat, the cop waves and the chase in memory. `panic_on_heat` makes `heat_get` panic.
#[derive(Default)]
pub struct FakePursuit {
    pub heat: Cell<f32>,
    pub waves: RefCell<HashMap<(u32, u32), HeatRowSpec>>,
    pub started: RefCell<Vec<(u32, Option<u64>)>>,
    pub ended: RefCell<Vec<EndMode>>,
    pub pursuing: Cell<bool>,
    pub panic_on_heat: bool,
}

impl PursuitBackend for FakePursuit {
    fn heat_get(&self, _owner: &str) -> BackendResult<f32> {
        if self.panic_on_heat {
            panic!("fake pursuit: expected panic");
        }
        Ok(self.heat.get())
    }

    fn heat_set(&self, _owner: &str, heat: f32) -> BackendResult<()> {
        // The game clamps to the pursuit's range; the fake clamps to 1..=10.
        self.heat.set(heat.clamp(1.0, 10.0));
        Ok(())
    }

    fn cop_table_get(&self, _owner: &str, table: CopTable, level: u32) -> BackendResult<HeatRowSpec> {
        let key = (table.raw(), level);
        Ok(self.waves.borrow().get(&key).cloned().unwrap_or_else(|| HeatRowSpec {
            row: BbHeatRow {
                size: 0,
                cop_count: 0,
                patrol_cars: 0,
                full_engagement_cops: 0,
                first_four_interval: 0.0,
                spawn_interval: 0.0,
            },
            cops: Vec::new(),
        }))
    }

    fn cop_table_set(&self, _owner: &str, table: CopTable, level: u32, spec: &HeatRowSpec) -> BackendResult<()> {
        self.waves.borrow_mut().insert((table.raw(), level), spec.clone());
        Ok(())
    }

    fn start(&self, _owner: &str, level: u32, near: Option<u64>) -> BackendResult<()> {
        self.started.borrow_mut().push((level, near));
        self.pursuing.set(true);
        Ok(())
    }

    fn end(&self, _owner: &str, mode: EndMode) -> BackendResult<()> {
        self.ended.borrow_mut().push(mode);
        self.pursuing.set(false);
        Ok(())
    }

    fn is_pursuing(&self, _owner: &str) -> BackendResult<bool> {
        Ok(self.pursuing.get())
    }

    fn busted_get(&self, _owner: &str) -> BackendResult<f32> {
        Ok(0.25)
    }

    fn cops_in_pursuit(&self, _owner: &str) -> BackendResult<u32> {
        Ok(if self.pursuing.get() { 3 } else { 0 })
    }
}

/// A career backend that answers two calls and leaves the rest unsupported: the career has no state in the port.
#[derive(Default)]
pub struct FakeCareer {
    pub reasons: RefCell<Vec<String>>,
    pub milestone_defs: RefCell<Vec<BbMilestoneDef>>,
}

impl CareerBackend for FakeCareer {
    fn cash_get(&self, _owner: &str) -> BackendResult<i64> {
        Ok(1234)
    }

    fn cash_add(&self, _owner: &str, delta: i64, reason: &str) -> BackendResult<()> {
        self.reasons.borrow_mut().push(format!("{delta}:{reason}"));
        Ok(())
    }

    fn milestone_register(&self, _owner: &str, def: &BbMilestoneDef) -> BackendResult<u32> {
        self.milestone_defs.borrow_mut().push(*def);
        Ok(7)
    }

    fn garage_count(&self, _owner: &str) -> BackendResult<u32> {
        Ok(1)
    }

    fn garage_at(&self, _owner: &str, _index: u32) -> BackendResult<BbOwnedCar> {
        let name = BbName::new("Zonda").ok_or(BB_ERR_UNSUPPORTED)?;
        Ok(BbOwnedCar {
            size: core::mem::size_of::<BbOwnedCar>() as u32,
            _pad: 0,
            car: 42,
            current: 1,
            _pad2: [0; 3],
            name,
        })
    }
}

/// A races backend that records the registrations and answers an active race.
#[derive(Default)]
pub struct FakeRaces {
    pub checkpoints_seen: RefCell<Vec<usize>>,
    pub names: RefCell<Vec<String>>,
}

impl RacesBackend for FakeRaces {
    fn race_register(&self, _owner: &str, def: &BbRaceDef, checkpoints: &[BbVec3]) -> BackendResult<u32> {
        self.checkpoints_seen.borrow_mut().push(checkpoints.len());
        self.names.borrow_mut().push(def.name.text().unwrap_or("").to_owned());
        Ok(9)
    }

    fn race_current(&self, _owner: &str) -> BackendResult<BbRaceState> {
        Ok(BbRaceState {
            size: core::mem::size_of::<BbRaceState>() as u32,
            active: 1,
            _pad: [0; 3],
            id: 9,
            lap: 2,
            checkpoint: 3,
            position: 1,
            elapsed: 12.5,
        })
    }
}
