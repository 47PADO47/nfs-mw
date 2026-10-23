//! The server's input buffer of one client (`security-gameplay.md` section 4.3).

use nfsmw_sim::Tick;

use super::code::InputCode;
use super::expand_tick;
use super::frame::InputFrame;
use super::history::HISTORY_TICKS;

/// Ticks ahead of the server an input may be; further ahead is dropped as early.
pub const MAX_AHEAD_TICKS: u32 = 12;
/// Ticks the last received input is held when the next is missing: 100 ms at 60 Hz.
pub const HOLD_TICKS: u32 = 6;
/// Brake code of the fallback input after the hold: a gentle stop, about a quarter of the pedal.
const FALLBACK_BRAKE: u8 = 32;

/// What [`InputWindow::accept`] did with a frame's inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Accepted {
    /// Inputs stored for a tick not yet simulated.
    pub fresh: u8,
    /// Inputs for a tick already stored (redundant copies).
    pub duplicate: u8,
    /// Inputs for a tick already simulated: arrived too late to matter. Usual for the redundant tail of a frame.
    pub late: u8,
    /// Inputs more than [`MAX_AHEAD_TICKS`] ahead: dropped and worth a strike.
    pub early: u8,
}

/// Where the input of a tick came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputSource {
    /// The client's own input for the tick.
    Received,
    /// The last received input without its shift edges, held because this tick's has not arrived (within
    /// [`HOLD_TICKS`]).
    Held,
    /// Nothing usable: no pedal, wheel centred, a gentle brake.
    Fallback,
}

/// The inputs a server holds for one client, by tick.
#[derive(Debug, Clone)]
pub struct InputWindow {
    slots: [Option<(Tick, InputCode)>; HISTORY_TICKS],
    /// The next tick the server will simulate; inputs below it are late.
    next: Tick,
    /// The last input handed out as received, and its tick.
    last: Option<(Tick, InputCode)>,
}

impl InputWindow {
    /// An empty window whose next simulated tick is `next`.
    pub fn new(next: Tick) -> Self {
        Self { slots: [None; HISTORY_TICKS], next, last: None }
    }

    /// Stores the inputs of `frame`, whose 16-bit first tick is expanded against `server_tick` (the tick the server
    /// simulates next).
    pub fn accept(&mut self, frame: &InputFrame, server_tick: Tick) -> Accepted {
        self.next = self.next.max(server_tick);
        let first = expand_tick(frame.first_tick(), server_tick);
        let mut out = Accepted::default();
        for (offset, code) in frame.inputs().iter().enumerate() {
            let tick = first.wrapping_add(offset as u32);
            if tick < self.next {
                out.late += 1;
                continue;
            }
            if tick > server_tick.saturating_add(MAX_AHEAD_TICKS) {
                out.early += 1;
                continue;
            }
            let slot = &mut self.slots[tick as usize % HISTORY_TICKS];
            if slot.is_some_and(|(held, _)| held == tick) {
                out.duplicate += 1;
                continue;
            }
            *slot = Some((tick, *code));
            out.fresh += 1;
        }
        out
    }

    /// The input to simulate `tick` with, and where it came from. Moves the window past `tick`, so a later frame for it
    /// counts as late.
    pub fn input_for(&mut self, tick: Tick) -> (InputCode, InputSource) {
        self.next = self.next.max(tick.saturating_add(1));
        let slot = &mut self.slots[tick as usize % HISTORY_TICKS];
        if let Some((held, code)) = *slot
            && held == tick
        {
            *slot = None;
            self.last = Some((tick, code));
            return (code, InputSource::Received);
        }
        if let Some((at, code)) = self.last
            && tick.saturating_sub(at) <= HOLD_TICKS
        {
            // A shift is an edge: holding it would shift again every tick.
            return (InputCode { shift_up: false, shift_down: false, ..code }, InputSource::Held);
        }
        (InputCode { brake: FALLBACK_BRAKE, ..InputCode::NEUTRAL }, InputSource::Fallback)
    }

    /// The next tick the window expects to hand out.
    pub fn next_tick(&self) -> Tick {
        self.next
    }
}
