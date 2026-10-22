//! Every entity but the owner's car: decoded views into the interpolation buffers, sampled at display time
//! (sync-model.md section 3).

use blackbox_net_core::replicate::Frame;
use nfsmw_sim::{EntityId, PlayerId, Tick};

use crate::interp::{Drawn, InterpConfig, Interpolator};
use crate::schema::GameSchema;

/// The remote entities.
#[derive(Debug)]
pub struct Remotes {
    interp: Interpolator,
    /// The ids of the newest view fed, ascending, to see which entities left.
    known: Vec<EntityId>,
    scratch: Vec<EntityId>,
    newest: Option<Tick>,
    bad_records: u64,
    /// Player cars by owner in the newest view: `(owner, entity)`.
    owners: Vec<(PlayerId, EntityId)>,
}

impl Remotes {
    pub fn new(config: InterpConfig) -> Self {
        Self {
            interp: Interpolator::new(config),
            known: Vec::new(),
            scratch: Vec::new(),
            newest: None,
            bad_records: 0,
            owners: Vec::new(),
        }
    }

    /// Feeds the view of snapshot `tick`, which arrived when the server clock read `server_tick`. `own` is left out.
    /// A view older than the newest fed is ignored (it still serves as a baseline).
    pub fn on_view(&mut self, schema: &GameSchema, view: &Frame, tick: Tick, server_tick: f64, own: Option<EntityId>) {
        if self.newest.is_some_and(|newest| tick <= newest) {
            return;
        }
        self.newest = Some(tick);
        self.interp.on_snapshot(tick, server_tick);
        self.scratch.clear();
        self.owners.clear();
        for (id, kind, values) in view.iter() {
            if Some(id) == own {
                continue;
            }
            let Ok(decoded) = schema.decode(id, kind, values) else {
                self.bad_records += 1;
                continue;
            };
            if let Some(owner) = decoded.view.owner {
                self.owners.push((owner, id));
            }
            self.interp.push(tick, &decoded.view);
            self.scratch.push(id);
        }
        for gone in self.known.iter().filter(|id| self.scratch.binary_search(id).is_err()) {
            self.interp.remove(*gone, tick);
        }
        std::mem::swap(&mut self.known, &mut self.scratch);
    }

    /// Advances the display delay by `dt_ms`.
    pub fn advance(&mut self, dt_ms: f64) {
        self.interp.advance(dt_ms);
    }

    /// Every remote entity drawn at server tick estimate `server_tick`, into `out` (cleared first); records behind
    /// the display time are pruned.
    pub fn sample(&mut self, server_tick: f64, out: &mut Vec<(EntityId, Drawn)>) {
        out.clear();
        let r = self.interp.display_tick(server_tick);
        self.interp.sample_all(r, out);
        self.interp.prune(r);
    }

    /// The car entity of player `owner` in the newest view.
    pub fn entity_of(&self, owner: PlayerId) -> Option<EntityId> {
        self.owners.iter().find(|(player, _)| *player == owner).map(|(_, id)| *id)
    }

    /// Forgets what was fed (a reconnect); buffered records play out.
    pub fn new_connection(&mut self) {
        self.newest = None;
    }

    /// Records the schema refused.
    pub fn bad_records(&self) -> u64 {
        self.bad_records
    }
}
