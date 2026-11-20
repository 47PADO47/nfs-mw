//! The front-end screen dump: what the front end hands the presenter for each screen, as data, once the screen has
//! settled. Off unless `NFSMW_FE_DUMP=<file>` names a file (`docs/testing.md`, "Front-end screen dump").
//!
//! One JSON line per snapshot of one screen: every object of its package with its visibility, whether it was drawn,
//! its place, size, colour, text, texture and current script, and the objects the host added to the tree (the footer
//! hints). A research script compares such lines between two runs object by object; nothing here reads pixels.
//!
//! A screen has settled once it has been drawn for [`SETTLE_SECONDS`] without a break and none of its objects plays a
//! play-once script that has not reached its end (looping scripts never end, so they do not hold a snapshot back).
//! A screen that is still animating after [`FORCE_SECONDS`] gets its first snapshot then anyway, marked
//! `settled: false` (a screen that loops a play-once script, or one closed after a few seconds).
//! A screen gets up to [`SNAPSHOTS`] snapshots, [`REPEAT_SECONDS`] apart. A screen that leaves the frame and comes back
//! starts again.

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use serde_json::{Value, json};

use super::race_trace::Build;

/// The variable that names the dump file.
pub const FE_DUMP_ENV: &str = "NFSMW_FE_DUMP";
/// The variable that limits the dump to some screens: file names, comma separated, with or without `.fng`.
pub const FE_DUMP_SCREENS_ENV: &str = "NFSMW_FE_DUMP_SCREENS";
/// The layout's name, in every line.
pub const SCHEMA: &str = "fe-screen-dump-v1";
/// How long a screen must have been drawn before its first snapshot.
pub const SETTLE_SECONDS: f64 = 1.5;
/// How long a screen that is still animating is drawn before it gets a snapshot anyway.
pub const FORCE_SECONDS: f64 = 2.5;
/// The time between two snapshots of one visit.
pub const REPEAT_SECONDS: f64 = 2.0;
/// Snapshots per visit of a screen.
pub const SNAPSHOTS: u32 = 3;

/// One screen being drawn frame after frame.
struct Visit {
    name: String,
    since: f64,
    taken: u32,
    last: f64,
    seen: bool,
}

/// Which screens are due a snapshot. Pure bookkeeping over the frame's clock, so it can be tested without a game.
#[derive(Default)]
pub struct Settle {
    visits: Vec<Visit>,
}

/// A snapshot that is due: its number within the visit and how long the screen has been drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Due {
    pub snapshot: u32,
    pub shown: f64,
    /// False when the snapshot was taken while a play-once script still ran ([`FORCE_SECONDS`]).
    pub settled: bool,
}

impl Settle {
    /// Starts a frame: a screen not drawn in it is forgotten at [`Settle::end`].
    pub fn begin(&mut self) {
        for visit in &mut self.visits {
            visit.seen = false;
        }
    }

    /// The screen `name` is drawn at `now` seconds; `busy` says a play-once script of it is still running.
    pub fn due(&mut self, name: &str, now: f64, busy: bool) -> Option<Due> {
        let at = match self.visits.iter().position(|v| v.name == name) {
            Some(at) => at,
            None => {
                self.visits.push(Visit { name: name.to_owned(), since: now, taken: 0, last: now, seen: true });
                self.visits.len() - 1
            }
        };
        let visit = &mut self.visits[at];
        visit.seen = true;
        let shown = now - visit.since;
        if (busy && shown < FORCE_SECONDS) || shown < SETTLE_SECONDS || visit.taken >= SNAPSHOTS {
            return None;
        }
        if visit.taken > 0 && now - visit.last < REPEAT_SECONDS {
            return None;
        }
        visit.taken += 1;
        visit.last = now;
        Some(Due { snapshot: visit.taken - 1, shown, settled: !busy })
    }

    /// Ends a frame: the screens it did not draw start again when they come back.
    pub fn end(&mut self) {
        self.visits.retain(|v| v.seen);
    }
}

struct Dump {
    path: PathBuf,
    out: File,
    opened: Instant,
    settle: Settle,
    only: Vec<String>,
}

static DUMP: OnceLock<Option<Mutex<Dump>>> = OnceLock::new();
/// Packages other systems drew since the front end's last frame (the in-game HUD), for the snapshots' frame lists.
static OTHERS: Mutex<Vec<(String, usize)>> = Mutex::new(Vec::new());

/// Notes that a package outside the front end's stack was drawn this frame (`HUD_SingleRace.fng`), with how many objects it
/// drew. Cheap when off.
pub fn note_drawn(name: &str, drawn: usize) {
    if !enabled() {
        return;
    }
    let Ok(mut others) = OTHERS.lock() else { return };
    if !others.iter().any(|(n, _)| n == name) {
        others.push((name.to_owned(), drawn));
    }
}

/// The packages noted since the last call, emptied, each with the number of objects it drew.
pub fn take_others() -> Vec<(String, usize)> {
    OTHERS.lock().map(|mut others| std::mem::take(&mut *others)).unwrap_or_default()
}

/// The screen name the dump uses: the file name without `.fng`, in lower case.
pub fn screen_key(name: &str) -> String {
    let lower = name.trim().to_ascii_lowercase();
    lower.strip_suffix(".fng").map_or(lower.clone(), str::to_owned)
}

fn open() -> Option<Mutex<Dump>> {
    let path = std::env::var_os(FE_DUMP_ENV).filter(|v| !v.is_empty()).map(PathBuf::from)?;
    let created = path
        .parent()
        .filter(|folder| !folder.as_os_str().is_empty())
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| File::create(&path));
    let out = match created {
        Ok(out) => out,
        Err(e) => {
            log::error!("front-end dump: cannot create {}: {e}", path.display());
            return None;
        }
    };
    let only = std::env::var(FE_DUMP_SCREENS_ENV)
        .unwrap_or_default()
        .split(',')
        .map(screen_key)
        .filter(|name| !name.is_empty())
        .collect();
    log::info!("front-end dump: recording into {}", path.display());
    Some(Mutex::new(Dump { path, out, opened: Instant::now(), settle: Settle::default(), only }))
}

/// Whether the run dumps screens. Callers test this before building anything.
pub fn enabled() -> bool {
    DUMP.get_or_init(open).is_some()
}

/// One frame's screens, bottom first, as `(file name, busy)`. Returns the screens due a snapshot, with the snapshot.
pub fn frame(screens: &[(String, bool)]) -> Vec<(usize, Due)> {
    let Some(Some(dump)) = DUMP.get() else { return Vec::new() };
    let Ok(mut dump) = dump.lock() else { return Vec::new() };
    let now = dump.opened.elapsed().as_secs_f64();
    let dump = &mut *dump;
    dump.settle.begin();
    let mut due = Vec::new();
    for (at, (name, busy)) in screens.iter().enumerate() {
        let key = screen_key(name);
        if !dump.only.is_empty() && !dump.only.contains(&key) {
            continue;
        }
        // A screen covered by another (the inbox under a message) is a new visit with its own snapshots, as in the
        // original's dump: the visit is the screen with the screens drawn above it.
        let above: Vec<String> = screens[at + 1..].iter().map(|(n, _)| screen_key(n)).collect();
        let visit = format!("{key}|{}", above.join(","));
        if let Some(d) = dump.settle.due(&visit, now, *busy) {
            due.push((at, d));
        }
    }
    dump.settle.end();
    due
}

/// Writes one snapshot of `name`: `frame` holds the file names of every screen drawn in the frame, bottom first.
/// `frame` lists the packages drawn in the frame, bottom first, each with the number of objects it drew (`frame_drawn`,
/// as the original's dump counts them: a package walked but drawing nothing has 0).
pub fn write(name: &str, due: Due, frame: &[(String, usize)], objects: Vec<Value>) {
    let Some(Some(dump)) = DUMP.get() else { return };
    let Ok(mut dump) = dump.lock() else { return };
    let line = json!({
        "schema": SCHEMA,
        "side": "remake",
        "screen": screen_key(name),
        "file": name,
        "snapshot": due.snapshot,
        "shown_s": due.shown,
        "wall_s": dump.opened.elapsed().as_secs_f64(),
        "frame": frame.iter().map(|(n, _)| screen_key(n)).collect::<Vec<_>>(),
        "frame_drawn": frame.iter().map(|(n, d)| (screen_key(n), json!(d))).collect::<serde_json::Map<_, _>>(),
        "build": Build::current(),
        "settled": due.settled,
        "settle": {"seconds": SETTLE_SECONDS, "force_seconds": FORCE_SECONDS,
            "rule": "drawn without a break and no play-once script running, or drawn for force_seconds"},
        "objects": objects,
    });
    let written = serde_json::to_vec(&line).map_err(std::io::Error::other).and_then(|mut bytes| {
        bytes.push(b'\n');
        dump.out.write_all(&bytes).and_then(|()| dump.out.flush())
    });
    if let Err(e) = written {
        log::error!("front-end dump: writing {} failed: {e}", dump.path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_screen_settles_after_its_time_and_its_scripts() {
        let mut s = Settle::default();
        let at = |s: &mut Settle, now: f64, busy: bool| {
            s.begin();
            let due = s.due("sms_message", now, busy);
            s.end();
            due
        };
        assert_eq!(at(&mut s, 0.0, false), None);
        assert_eq!(at(&mut s, 1.0, false), None, "not shown long enough");
        assert_eq!(at(&mut s, 1.6, true), None, "a play-once script still runs");
        assert_eq!(at(&mut s, 1.7, false).map(|d| (d.snapshot, d.settled)), Some((0, true)));
        assert_eq!(at(&mut s, 2.0, false), None, "too soon for the next one");
        assert_eq!(at(&mut s, 3.8, false).map(|d| d.snapshot), Some(1));
        assert_eq!(at(&mut s, 5.9, false).map(|d| d.snapshot), Some(2));
        assert_eq!(at(&mut s, 9.0, false), None, "three per visit");
    }

    #[test]
    fn a_screen_that_keeps_animating_gets_an_unsettled_snapshot() {
        let mut s = Settle::default();
        s.begin();
        assert_eq!(s.due("car_select", 0.0, true), None);
        assert_eq!(s.due("car_select", 2.4, true), None);
        assert_eq!(s.due("car_select", 2.6, true).map(|d| (d.snapshot, d.settled)), Some((0, false)));
        s.end();
    }

    #[test]
    fn a_screen_that_leaves_starts_again() {
        let mut s = Settle::default();
        s.begin();
        assert_eq!(s.due("a", 0.0, false), None);
        s.end();
        s.begin();
        s.end();
        s.begin();
        assert_eq!(s.due("a", 2.0, false), None, "the visit restarted at 2.0");
        s.end();
    }

    #[test]
    fn names_lose_case_and_extension() {
        assert_eq!(screen_key("SMS_Message.fng"), "sms_message");
        assert_eq!(screen_key(" InGamePhotoMaster "), "ingamephotomaster");
    }
}
