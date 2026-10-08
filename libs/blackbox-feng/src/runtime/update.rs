//! Advancing the clocks of object scripts, firing their events and writing their tracks (spec section 3).

use super::interp::{evaluate, setup_move_to};
use super::messages::{MSG_SET_BUTTON, Target};
use super::{PackageId, Runtime};
use crate::package::word;

impl Runtime {
    /// Advances every object of a package by `dt` ticks.
    pub(super) fn update_package(&mut self, package: PackageId, dt: i32) {
        let Some(order) = self.running(package).map(|p| p.order.clone()) else { return };
        for i in order {
            self.update_object(package, i, dt);
        }
    }

    fn update_object(&mut self, package: PackageId, index: usize, dt: i32) {
        let Some(p) = self.running(package) else { return };
        let obj = &p.objects[index];
        let Some(script_index) = obj.current else { return };
        let script = &obj.scripts[script_index];
        let (prev, len) = (obj.time, script.length);
        let chain = script.chain.and_then(|id| obj.scripts.iter().position(|s| s.id == id));
        let behaviour = script.end_behaviour();
        let has_events = !script.events.is_empty();
        let mut time = (prev + dt).max(0);
        let mut dirty = obj.dirty;

        let mut apply_old_first = false;
        let mut fire: Vec<(usize, i32, i32)> = Vec::new(); // (script, from, to)
        let mut next_script = script_index;
        let mut restart = false;

        if time >= len {
            if let Some(next) = chain {
                apply_old_first = true;
                let over = time - len;
                if has_events {
                    fire.push((script_index, prev, len));
                }
                next_script = next;
                restart = true;
                time = over;
                if !self.running(package).unwrap().objects[index].scripts[next].events.is_empty() {
                    fire.push((next, 0, time));
                }
            } else {
                match behaviour {
                    0 => {
                        if has_events {
                            fire.push((script_index, prev, len));
                        }
                        time = len + 1;
                    }
                    1 => {
                        if len > 0 {
                            if has_events {
                                fire.push((script_index, prev, len));
                            }
                            time -= (time / len) * len;
                            if has_events {
                                fire.push((script_index, 0, time));
                            }
                            restart = true;
                        } else {
                            time = 0;
                        }
                    }
                    _ => {
                        time = if len > 0 { time - (time / (len * 2)) * (len * 2) } else { 0 };
                    }
                }
            }
        } else if has_events {
            match behaviour {
                0 => fire.push((script_index, prev, time)),
                1 => {
                    if time < prev {
                        fire.push((script_index, prev, len));
                        fire.push((script_index, 0, time));
                    } else {
                        fire.push((script_index, prev, time));
                    }
                }
                _ => {
                    if prev < len {
                        fire.push((script_index, prev, time));
                    } else {
                        fire.push((script_index, prev - len, 0));
                        fire.push((script_index, 0, time));
                    }
                }
            }
        }

        if apply_old_first {
            self.apply_tracks(package, index, script_index, prev.max(0).max(len));
        }
        {
            let p = self.running_mut(package).unwrap();
            let o = &mut p.objects[index];
            o.time = time;
            if next_script != script_index {
                o.current = Some(next_script);
                dirty = true;
            }
            if restart {
                let words = o.data.words.clone();
                setup_move_to(&mut o.scripts[next_script].tracks, &words);
                dirty = true;
            }
        }
        for (s, from, to) in fire {
            self.fire_events(package, index, s, from, to);
        }
        // Once a "once" script sits at its end and nothing changed, the tracks are not re-applied, so values
        // the host set stay put.
        let at_rest = prev == time && prev == len + 1 && !dirty;
        if !at_rest {
            self.apply_tracks(package, index, next_script, time);
        }
        if let Some(p) = self.running_mut(package) {
            p.objects[index].dirty = false;
        }
    }

    /// Writes the tracks of a script at `time` into the object data. The colour track always runs; the others
    /// only while the object is not fully transparent.
    fn apply_tracks(&mut self, package: PackageId, index: usize, script: usize, time: i32) {
        let Some(p) = self.running_mut(package) else { return };
        let o = &mut p.objects[index];
        let Some(s) = o.scripts.get(script) else { return };
        let write = |data: &mut crate::package::ObjectData, track: &crate::package::Track| {
            if let Some(v) = evaluate(track, time) {
                for (k, value) in v.iter().enumerate().take(track.param.components()) {
                    if let Some(w) = data.words.get_mut(track.offset + k) {
                        *w = *value;
                    }
                }
            }
        };
        for t in s.tracks.iter().filter(|t| t.offset == word::COLOUR) {
            write(&mut o.data, t);
        }
        if o.data.alpha() != 0 {
            for t in s.tracks.iter().filter(|t| t.offset != word::COLOUR) {
                write(&mut o.data, t);
            }
        }
    }

    /// Sends the messages of the events whose time lies in `[from, to)` (the end tick counts at the end of a
    /// script).
    fn fire_events(&mut self, package: PackageId, index: usize, script: usize, from: i32, to: i32) {
        if to < from {
            return;
        }
        let Some(p) = self.running(package) else { return };
        let s = &p.objects[index].scripts[script];
        let to = if to == s.length { to + 1 } else { to };
        let events: Vec<_> = s
            .events
            .iter()
            .filter(|e| (e.time as i64) >= from as i64 && (e.time as i64) < to as i64)
            .copied()
            .collect();
        for e in events {
            match e.target {
                0 if e.message == MSG_SET_BUTTON => self.set_focus_index(package, None, true),
                0 => self.queue(e.message, Some(index), package, Target::All),
                t => match Target::from_special(t) {
                    Some(special) => self.queue(e.message, Some(index), package, special),
                    None => {
                        let target = self.running(package).and_then(|p| p.by_guid.get(&t).copied());
                        if e.message == MSG_SET_BUTTON {
                            self.set_focus_index(package, target, true);
                        } else if let Some(target) = target {
                            self.queue(e.message, Some(index), package, Target::Object(target));
                        }
                    }
                },
            }
        }
    }

    /// Switches an object to the script with this id (time 0). Does nothing if it has none.
    pub(super) fn set_script(&mut self, package: PackageId, index: usize, id: u32) {
        let Some(p) = self.running_mut(package) else { return };
        let o = &mut p.objects[index];
        let Some(s) = o.scripts.iter().position(|s| s.id == id) else { return };
        o.current = Some(s);
        o.time = 0;
        o.dirty = true;
        let words = o.data.words.clone();
        setup_move_to(&mut o.scripts[s].tracks, &words);
    }

    pub(super) fn current_script_id(&self, package: PackageId, index: usize) -> Option<u32> {
        let o = self.running(package)?.objects.get(index)?;
        o.current.map(|s| o.scripts[s].id)
    }
}
