//! A running copy of a module: the image it works on, the decoded code, and the update that runs the graph once.

use std::sync::Arc;

use crate::bank::{Module, ModuleBank};
use crate::code::{self, Op};
use crate::error::{Error, Result};
use crate::host::{Host, PlayerInputs};
use crate::mem::Mem;
use crate::nodes::{self, Cx, Rng};

/// The bytes before the first node of an instance (the run-time list links).
const HEADER: usize = 0x18;

/// What a player is doing, for tests and tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerView {
    /// The sample type of the playing sound; `None` when nothing plays.
    pub playing: Option<u8>,
    /// The play control the graph last gave (0 stop, 1 play, 2 pause).
    pub control: i32,
    /// The sample select the graph last gave.
    pub select: i32,
}

/// One instance of a module.
#[derive(Debug, Clone)]
pub struct Instance {
    mem: Mem,
    ops: Arc<Vec<Op>>,
    rng: Rng,
    base: usize,
    players: Vec<usize>,
    controllers: Vec<usize>,
    class_data: Option<usize>,
    class_destructor: Option<usize>,
    destroyed: bool,
    module_id: u32,
}

/// The nodes the code reaches with `call 0` (class destructor) and `call 1` (class data), found by walking the
/// pointer the code walks.
fn find_class_nodes(ops: &[Op], base: usize) -> (Option<usize>, Option<usize>) {
    let (mut esi, mut stack) = (base as i64, Vec::new());
    let (mut destructor, mut data) = (None, None);
    for op in ops {
        match *op {
            Op::Push => stack.push(esi),
            Op::Pop => esi = stack.pop().unwrap_or(esi),
            Op::LoadArg => esi = base as i64,
            Op::AddEsi(n) => esi += i64::from(n),
            Op::Call(0) if destructor.is_none() => destructor = usize::try_from(esi).ok(),
            Op::Call(1) if data.is_none() => data = usize::try_from(esi).ok(),
            _ => {}
        }
    }
    (destructor, data)
}

fn word(m: &Mem, esi: usize, disp: i32) -> i32 {
    m.i32(m.offset(esi, i64::from(disp)))
}

impl Instance {
    /// Makes an instance of module `module` of `bank`. `seed` seeds the random nodes: equal seeds give equal runs.
    pub fn new(bank: &ModuleBank, module: usize, seed: u32) -> Result<Instance> {
        let m: &Module = bank.modules().get(module).ok_or(Error::NoSuchModule(module))?;
        let image = bank.image();
        let base = m.data + HEADER;
        let end = m.data.checked_add(m.data_size).filter(|&e| e <= image.len()).ok_or(Error::Bad("module data"))?;
        let ops = code::decode(image, m.code, m.data.min(end))?;
        let (class_destructor, class_data) = find_class_nodes(&ops, base);
        let offsets = |range: std::ops::Range<usize>| -> Vec<usize> {
            m.node_offsets[range].iter().map(|o| m.data + o).collect()
        };
        let players = usize::from(m.players);
        Ok(Instance {
            mem: Mem::new(image.to_vec()),
            ops: Arc::new(ops),
            rng: Rng::new(seed),
            base,
            players: offsets(0..players),
            controllers: offsets(players..m.node_offsets.len()),
            class_data,
            class_destructor,
            destroyed: false,
            module_id: m.id,
        })
    }

    /// The module's id.
    pub fn module_id(&self) -> u32 {
        self.module_id
    }

    /// The number of players.
    pub fn players(&self) -> usize {
        self.players.len()
    }

    /// The number of class-data words the module has (the object's parameters); 0 without a class-data node.
    pub fn class_data_len(&self) -> usize {
        self.class_data.map_or(0, |at| usize::from(self.mem.u8(at + 0x10)))
    }

    /// Sets the object's parameters (extra values are dropped, missing ones keep their value).
    pub fn set_class_data(&mut self, params: &[i32]) {
        let Some(at) = self.class_data else { return };
        for (k, &value) in params.iter().take(self.class_data_len()).enumerate() {
            self.mem.set_i32(at + 0x14 + 4 * k, value);
        }
    }

    /// The current value of class-data word `k`.
    pub fn class_data(&self, k: usize) -> i32 {
        self.class_data.map_or(0, |at| self.mem.i32(at + 0x14 + 4 * k))
    }

    /// Tells the graph the object was deleted: its class-destructor node fires on the next update.
    pub fn request_destroy(&mut self) {
        if let Some(at) = self.class_destructor {
            self.mem.set_i32(at + 0x10, 1);
        }
    }

    /// Whether the graph ended the instance (its destroy node fired).
    pub fn is_destroyed(&self) -> bool {
        self.destroyed
    }

    /// The values of the inputs of player `player`.
    pub fn player_inputs(&self, player: usize) -> PlayerInputs {
        let Some(&p) = self.players.get(player) else { return PlayerInputs::default() };
        let mut out = PlayerInputs::default();
        for k in 0..usize::from(self.mem.u8(p + 0xE)) {
            let at = p + 0x1C + 12 * k;
            out.set(usize::from(self.mem.u8(at)), self.mem.i32(at + 8), false);
        }
        out
    }

    /// What player `player` is doing.
    pub fn player_view(&self, player: usize) -> Option<PlayerView> {
        let &p = self.players.get(player)?;
        let kind = self.mem.i8(p + 0x10);
        Some(PlayerView {
            playing: u8::try_from(kind).ok(),
            control: self.mem.i32(p + 0x18),
            select: self.mem.i32(p + 0x14),
        })
    }

    /// Ends the instance now: every voice stops and every object it made is released.
    pub fn destroy(&mut self, host: &mut dyn Host) {
        let mut cx = Cx {
            mem: &mut self.mem,
            period: 0.0,
            rng: &mut self.rng,
            host,
            players: &self.players,
            controllers: &self.controllers,
            destroyed: &mut self.destroyed,
        };
        nodes::teardown(&mut cx);
        self.destroyed = true;
    }

    /// Runs the graph once. `period_ms` is the time since the last run. Voices and objects are the host's.
    pub fn update(&mut self, period_ms: f32, host: &mut dyn Host) -> Result<()> {
        if self.destroyed {
            return Ok(());
        }
        let ops = Arc::clone(&self.ops);
        let (mut esi, mut stack) = (self.base, Vec::<usize>::new());
        let (mut eax, mut ecx, mut flags) = (0i32, 0i32, (0i32, 0i32));
        let mut at = 0;
        while let Some(&op) = ops.get(at) {
            at += 1;
            match op {
                Op::Push => stack.push(esi),
                Op::Pop => esi = stack.pop().ok_or(Error::Fault(esi))?,
                Op::Ret => break,
                Op::LoadArg => esi = self.base,
                Op::Call(id) => {
                    let node = *stack.last().ok_or(Error::Fault(esi))?;
                    let mut cx = Cx {
                        mem: &mut self.mem,
                        period: period_ms,
                        rng: &mut self.rng,
                        host: &mut *host,
                        players: &self.players,
                        controllers: &self.controllers,
                        destroyed: &mut self.destroyed,
                    };
                    eax = nodes::call(&mut cx, id, node)?;
                    if self.destroyed {
                        break;
                    }
                }
                Op::Store(d) => {
                    let to = self.mem.offset(esi, i64::from(d));
                    self.mem.set_i32(to, eax);
                }
                Op::StoreByte(d, v) => {
                    let to = self.mem.offset(esi, i64::from(d));
                    self.mem.set_u8(to, v);
                }
                Op::LoadEax(d) => eax = word(&self.mem, esi, d),
                Op::LoadEcx(d) => ecx = word(&self.mem, esi, d),
                Op::AddEax(d) => eax = eax.wrapping_add(word(&self.mem, esi, d)),
                Op::SubEax(d) => eax = eax.wrapping_sub(word(&self.mem, esi, d)),
                Op::MulEax(d) => eax = eax.wrapping_mul(word(&self.mem, esi, d)),
                Op::Cmp => flags = (eax, ecx),
                Op::MovEaxEcx => eax = ecx,
                Op::JumpLess(to) if flags.0 < flags.1 => at = to,
                Op::JumpGreater(to) if flags.0 > flags.1 => at = to,
                Op::JumpLess(_) | Op::JumpGreater(_) => {}
                Op::AddEsi(n) => esi = self.mem.offset(esi, i64::from(n)),
                Op::AddEsp(n) => stack.truncate(stack.len().saturating_sub(usize::try_from(n).unwrap_or(0) / 4)),
            }
        }
        match self.mem.take_fault() {
            Some(at) => Err(Error::Fault(at)),
            None => Ok(()),
        }
    }
}
