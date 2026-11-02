//! The pursuit board's backup row: a backing, the "BACKUP:" label and a timer, under the board
//! (`docs/specs/hud-pursuit.md`, "Backup timer"). The game drives only the backing's scripts; the backing's appear,
//! leave and flash scripts send messages that the row's group (label and timer) answers with its own. Each HUD update
//! of a chase: no backup on its way plays the leave script, ten seconds or less the flash, more the appear, each only
//! when it is not the one playing; while a backup is on its way the timer shows the time left. The row is left as it
//! is while the board shows the cool-down.

use blackbox_feng::{ObjectRef, PackageId, Runtime};

use super::pursuit::{PursuitHud, timer_text};

/// The backing (`Outrun_Backing.tga` under the board) and the timer string `BACKUP_TIMER`.
const BACKING: u32 = 0x8c20_a763;
const TIMER: u32 = 0xbee4_4775;
/// The backing's scripts.
pub const APPEAR: u32 = 0x5079_c8f8;
pub const LEAVE: u32 = 0x0331_13ac;
pub const FLASH: u32 = 0x026d_ed57;
/// At this many seconds or fewer the row flashes.
pub const URGENT_SECS: f32 = 10.0;

/// The script the backing should play for `eta` seconds to the backup.
pub fn script_for(eta: f32) -> u32 {
    if eta <= 0.0 {
        return LEAVE;
    }
    if eta <= URGENT_SECS {
        return FLASH;
    }
    APPEAR
}

/// The row's objects, and whether it has been shown in this chase.
pub struct BackupTimer {
    backing: Option<ObjectRef>,
    timer: Option<ObjectRef>,
    shown: bool,
}

impl BackupTimer {
    pub fn new(rt: &Runtime, package: PackageId) -> Self {
        Self { backing: rt.find(package, BACKING), timer: rt.find(package, TIMER), shown: false }
    }

    /// Once per HUD update. Outside a chase the board's group hides the row with it.
    pub fn apply(&mut self, rt: &mut Runtime, pursuit: Option<PursuitHud>) {
        let Some(pursuit) = pursuit else {
            self.shown = false;
            return;
        };
        if pursuit.cooldown {
            return;
        }
        let Some(backing) = self.backing else { return };
        let script = script_for(pursuit.backup_eta);
        // The row rests hidden: its leave script only takes down a row that was shown in this chase.
        let leaving_hidden = script == LEAVE && !self.shown;
        if rt.script_of(backing) != Some(script) && !leaving_hidden {
            rt.run_script(backing, script);
        }
        self.shown = script != LEAVE;
        if pursuit.backup_eta <= 0.0 {
            return;
        }
        if let Some(timer) = self.timer {
            rt.set_text(timer, timer_text(pursuit.backup_eta));
        }
    }
}
