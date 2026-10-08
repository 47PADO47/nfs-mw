//! The nodes that touch the outside: players (voices), class controllers (other objects) and the destroy node.
//! Spec: `docs/specs/aems.md` §5 and §6.

use super::Cx;
use crate::error::{Error, Result};
use crate::host::{ClassCall, PlayerInputs, SampleEntry};
use crate::mem::Mem;

/// The first input of a player and the size of one.
const INPUTS: usize = 0x1C;
const INPUT_SIZE: usize = 12;

fn read_inputs(m: &Mem, p: usize, all: bool) -> PlayerInputs {
    let mut out = PlayerInputs::default();
    for k in 0..usize::from(m.u8(p + 0xE)) {
        let at = p + INPUTS + INPUT_SIZE * k;
        let (kind, previous, value) = (m.u8(at), m.i32(at + 4), m.i32(at + 8));
        out.set(usize::from(kind), value, all || value != previous);
    }
    out
}

/// Remembers the values the voice has been told, for the input types `keep` accepts.
fn remember(m: &mut Mem, p: usize, keep: impl Fn(u8) -> bool) {
    for k in 0..usize::from(m.u8(p + 0xE)) {
        let at = p + INPUTS + INPUT_SIZE * k;
        if keep(m.u8(at)) {
            m.set_i32(at + 4, m.i32(at + 8));
        }
    }
}

/// Forgets the voice: no sound, no elapsed or remaining time.
fn reset_outputs(m: &mut Mem, p: usize) {
    m.set_i8(p + 0x10, -1);
    if m.u8(p + 0xF) == 0 {
        return;
    }
    let outputs = p + INPUTS + INPUT_SIZE * usize::from(m.u8(p + 0xE));
    m.set_i32(outputs, 0);
    m.set_i32(outputs + 4, 0);
}

/// The sample group entry the player's select picks, if the group has any.
fn picked_entry(m: &Mem, p: usize) -> Option<SampleEntry> {
    let group = m.u32(p + 4) as usize;
    let count = m.u32(group);
    if count == 0 {
        return None;
    }
    let select = m.i32(p + 0x14).clamp(0, (count - 1).min(i32::MAX as u32) as i32) as usize;
    let at = group + 4 + 12 * select;
    Some(SampleEntry { kind: m.u8(at), priority: m.u8(at + 1), index: m.u32(at + 4), loop_offset: m.u32(at + 8) })
}

/// The play control changed to `control`.
fn change(cx: &mut Cx<'_>, index: usize, p: usize, control: i32) {
    let m = &mut *cx.mem;
    let playing = m.i8(p + 0x10) >= 0;
    match control {
        0 if playing => {
            cx.host.stop(index);
            reset_outputs(m, p);
        }
        1 if playing => {
            cx.host.resume(index, &read_inputs(m, p, true));
            remember(m, p, |kind| kind == 0 || kind == 2);
        }
        1 => {
            let Some(entry) = picked_entry(m, p) else { return };
            m.set_i8(p + 0x10, entry.kind as i8);
            let inputs = read_inputs(m, p, true);
            remember(m, p, |_| true);
            if !cx.host.play(index, &entry, &inputs) {
                reset_outputs(m, p);
            }
        }
        2 if playing => cx.host.pause(index),
        _ => {}
    }
}

pub fn player(cx: &mut Cx<'_>, p: usize) -> Result<i32> {
    let index = cx.players.iter().position(|&o| o == p).ok_or(Error::Fault(p))?;
    let control = cx.mem.i32(p + 0x18).clamp(0, 2);
    let previous = i32::from(cx.mem.i8(p + 0xC));
    if control != previous {
        change(cx, index, p, control);
        cx.mem.set_i8(p + 0xD, previous as i8);
        cx.mem.set_i8(p + 0xC, control as i8);
    }
    let m = &mut *cx.mem;
    if m.i8(p + 0x10) < 0 {
        return Ok(0);
    }
    if control != 1 {
        return Ok(control);
    }
    let state = cx.host.update(index, &read_inputs(m, p, false));
    remember(m, p, |_| true);
    if !state.playing {
        reset_outputs(m, p);
        return Ok(0);
    }
    if m.u8(p + 0xF) != 0 {
        let outputs = p + INPUTS + INPUT_SIZE * usize::from(m.u8(p + 0xE));
        m.set_i32(outputs, state.remaining_ms);
        m.set_i32(outputs + 4, state.elapsed_ms);
    }
    Ok(1)
}

pub fn class_controller(cx: &mut Cx<'_>, p: usize) -> Result<i32> {
    let controller = cx.controllers.iter().position(|&o| o == p).ok_or(Error::Fault(p))?;
    let m = &mut *cx.mem;
    let (clamped, count) = (m.u8(p + 0xC) != 0, usize::from(m.u8(p + 0xD)));
    let inputs = if clamped { p + 0x10 + 8 * count } else { p + 0x10 };
    let created = m.i32(p + 8) != 0;
    let (construct, destruct) = (m.i32(inputs) != 0, m.i32(inputs + 4) != 0);
    if destruct {
        if !created {
            return Ok(0);
        }
        cx.host.class_call(&ClassCall { controller, created: false, released: true, params: &[] });
        m.set_i32(p + 8, 0);
        return Ok(0);
    }
    if !construct && !created {
        return Ok(0);
    }
    let mut params = Vec::with_capacity(count);
    for i in 0..count {
        let at = inputs + 8 + 4 * i;
        let value = m.i32(at);
        let value = match clamped {
            true => value.max(m.i32(p + 0x10 + 8 * i)).min(m.i32(p + 0x14 + 8 * i)),
            false => value,
        };
        m.set_i32(at, value);
        params.push(value);
    }
    let make = !created;
    let refs = cx.host.class_call(&ClassCall { controller, created: make, released: false, params: &params });
    if make && refs > 0 {
        m.set_i32(p + 8, 1);
    }
    Ok(refs.max(0))
}

/// Stops every voice and releases every object the instance made.
pub(crate) fn teardown(cx: &mut Cx<'_>) {
    for (index, &p) in cx.players.iter().enumerate() {
        if cx.mem.i8(p + 0x10) >= 0 {
            cx.host.stop(index);
            reset_outputs(cx.mem, p);
        }
    }
    for (controller, &p) in cx.controllers.iter().enumerate() {
        if cx.mem.i32(p + 8) != 0 {
            cx.host.class_call(&ClassCall { controller, created: false, released: true, params: &[] });
            cx.mem.set_i32(p + 8, 0);
        }
    }
}

pub fn destroy(cx: &mut Cx<'_>, p: usize) -> Result<i32> {
    if cx.mem.i32(p + 0xC) == 0 {
        return Ok(0);
    }
    teardown(cx);
    *cx.destroyed = true;
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::{Host, VoiceState, input};
    use crate::nodes::Rng;

    /// A host that remembers what it was asked and lets a voice live for `life` updates.
    #[derive(Default)]
    struct Recorder {
        log: Vec<String>,
        life: i32,
        updates: i32,
        objects: i32,
    }

    impl Host for Recorder {
        fn play(&mut self, player: usize, entry: &SampleEntry, inputs: &PlayerInputs) -> bool {
            self.log.push(format!("play {player} #{} vol {:?}", entry.index, inputs.get(input::VOLUME)));
            self.updates = 0;
            entry.index != 99
        }
        fn stop(&mut self, player: usize) {
            self.log.push(format!("stop {player}"));
        }
        fn pause(&mut self, player: usize) {
            self.log.push(format!("pause {player}"));
        }
        fn resume(&mut self, player: usize, _: &PlayerInputs) {
            self.log.push(format!("resume {player}"));
        }
        fn update(&mut self, player: usize, inputs: &PlayerInputs) -> VoiceState {
            self.updates += 1;
            let changed: Vec<usize> = (0..input::COUNT).filter(|&k| inputs.changed(k)).collect();
            self.log.push(format!("update {player} {changed:?}"));
            VoiceState { playing: self.updates <= self.life, elapsed_ms: 10 * self.updates, remaining_ms: 500 }
        }
        fn class_call(&mut self, call: &ClassCall<'_>) -> i32 {
            self.log.push(format!(
                "class {} new={} released={} {:?}",
                call.controller, call.created, call.released, call.params
            ));
            self.objects = i32::from(!call.released);
            self.objects
        }
    }

    /// Image: group at 0x10 with two entries (sounds 5 and 99); player node at 0x40 with a pitch and a volume input
    /// and outputs; class controller at 0x100 with two parameters clamped to 0..=10.
    fn image() -> Mem {
        let mut m = Mem::new(vec![0; 0x200]);
        m.set_i32(0x10, 2);
        m.set_i32(0x18, 5);
        m.set_i32(0x24, 99);
        m.set_i32(0x44, 0x10);
        m.set_u8(0x40 + 0xE, 2);
        m.set_u8(0x40 + 0xF, 1);
        m.set_i8(0x40 + 0x10, -1);
        m.set_u8(0x40 + 0x1C, 0);
        m.set_u8(0x40 + 0x1C + 12, 2);
        m.set_i32(0x40 + 0x1C + 8, 4096);
        m.set_i32(0x40 + 0x1C + 20, 20000);
        m.set_u8(0x100 + 0xC, 1);
        m.set_u8(0x100 + 0xD, 2);
        m.set_i32(0x110, 0);
        m.set_i32(0x114, 10);
        m.set_i32(0x118, 0);
        m.set_i32(0x11C, 10);
        m
    }

    fn run<R>(mem: &mut Mem, host: &mut Recorder, f: impl FnOnce(&mut Cx<'_>) -> R) -> R {
        let (mut rng, mut destroyed) = (Rng::new(1), false);
        let mut cx = Cx {
            mem,
            period: 16.0,
            rng: &mut rng,
            host,
            players: &[0x40],
            controllers: &[0x100],
            destroyed: &mut destroyed,
        };
        f(&mut cx)
    }

    #[test]
    fn a_player_starts_updates_and_ends_with_its_voice() {
        let (mut m, mut h) = (image(), Recorder { life: 2, ..Default::default() });
        assert_eq!(run(&mut m, &mut h, |cx| player(cx, 0x40)).unwrap(), 0, "control 0 plays nothing");
        m.set_i32(0x40 + 0x18, 1);
        assert_eq!(run(&mut m, &mut h, |cx| player(cx, 0x40)).unwrap(), 1, "started and updated");
        assert_eq!(h.log[0], "play 0 #5 vol Some(20000)");
        assert_eq!(m.i8(0x40 + 0x10), 0, "the sample type is remembered");
        m.set_i32(0x40 + 0x1C + 20, 15000);
        assert_eq!(run(&mut m, &mut h, |cx| player(cx, 0x40)).unwrap(), 1);
        assert_eq!(h.log.last().unwrap(), "update 0 [2]", "only the changed input is passed");
        assert_eq!(m.i32(0x40 + 0x1C + 24 + 4), 20, "elapsed time is an output");
        assert_eq!(m.i32(0x40 + 0x1C + 24), 500, "and the time left");
        assert_eq!(run(&mut m, &mut h, |cx| player(cx, 0x40)).unwrap(), 0, "the voice ended");
        assert_eq!(m.i8(0x40 + 0x10), -1);
        assert_eq!(m.i32(0x40 + 0x1C + 24 + 4), 0, "the outputs are cleared");
        assert_eq!(run(&mut m, &mut h, |cx| player(cx, 0x40)).unwrap(), 0, "it stays quiet until the control restarts");
    }

    #[test]
    fn a_player_pauses_resumes_and_stops() {
        let (mut m, mut h) = (image(), Recorder { life: 99, ..Default::default() });
        m.set_i32(0x40 + 0x18, 1);
        run(&mut m, &mut h, |cx| player(cx, 0x40)).unwrap();
        m.set_i32(0x40 + 0x18, 2);
        assert_eq!(run(&mut m, &mut h, |cx| player(cx, 0x40)).unwrap(), 2);
        m.set_i32(0x40 + 0x18, 1);
        run(&mut m, &mut h, |cx| player(cx, 0x40)).unwrap();
        m.set_i32(0x40 + 0x18, 0);
        assert_eq!(run(&mut m, &mut h, |cx| player(cx, 0x40)).unwrap(), 0);
        let verbs: Vec<&str> = h.log.iter().map(|l| l.split(' ').next().unwrap()).collect();
        assert_eq!(verbs, ["play", "update", "pause", "resume", "update", "stop"]);
    }

    #[test]
    fn the_select_picks_the_entry_and_a_failed_start_leaves_no_sound() {
        let (mut m, mut h) = (image(), Recorder { life: 9, ..Default::default() });
        m.set_i32(0x40 + 0x14, 7);
        m.set_i32(0x40 + 0x18, 1);
        assert_eq!(
            run(&mut m, &mut h, |cx| player(cx, 0x40)).unwrap(),
            0,
            "the select clamps to the last entry, which fails"
        );
        assert_eq!(h.log, ["play 0 #99 vol Some(20000)"]);
        assert_eq!(m.i8(0x40 + 0x10), -1);
    }

    #[test]
    fn a_class_controller_creates_feeds_and_releases() {
        let (mut m, mut h) = (image(), Recorder::default());
        assert_eq!(run(&mut m, &mut h, |cx| class_controller(cx, 0x100)).unwrap(), 0, "idle");
        m.set_i32(0x120, 1);
        m.set_i32(0x128, 50);
        m.set_i32(0x12C, 3);
        assert_eq!(run(&mut m, &mut h, |cx| class_controller(cx, 0x100)).unwrap(), 1);
        assert_eq!(h.log[0], "class 0 new=true released=false [10, 3]", "parameters are clamped");
        m.set_i32(0x120, 0);
        m.set_i32(0x128, 2);
        run(&mut m, &mut h, |cx| class_controller(cx, 0x100)).unwrap();
        assert_eq!(h.log[1], "class 0 new=false released=false [2, 3]", "later ticks only feed it");
        m.set_i32(0x124, 1);
        assert_eq!(run(&mut m, &mut h, |cx| class_controller(cx, 0x100)).unwrap(), 0);
        assert_eq!(h.log[2], "class 0 new=false released=true []");
    }

    #[test]
    fn destroy_stops_the_voices_and_ends_the_instance() {
        let (mut m, mut h) = (image(), Recorder { life: 9, ..Default::default() });
        m.set_i32(0x40 + 0x18, 1);
        run(&mut m, &mut h, |cx| player(cx, 0x40)).unwrap();
        assert!(!run(&mut m, &mut h, |cx| destroy(cx, 0x180).map(|_| *cx.destroyed)).unwrap());
        m.set_i32(0x180 + 0xC, 1);
        assert!(run(&mut m, &mut h, |cx| destroy(cx, 0x180).map(|_| *cx.destroyed)).unwrap());
        assert_eq!(h.log.last().unwrap(), "stop 0");
    }

    #[test]
    fn nodes_the_module_does_not_list_are_faults() {
        let (mut m, mut h) = (image(), Recorder::default());
        assert_eq!(run(&mut m, &mut h, |cx| player(cx, 0x44)), Err(Error::Fault(0x44)));
        assert_eq!(run(&mut m, &mut h, |cx| class_controller(cx, 0x44)), Err(Error::Fault(0x44)));
    }
}
