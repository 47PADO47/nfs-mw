//! The BearEffect resource (kind `0x301`) of the Chameleon engine: a particle effect made of emitters, each a list of
//! operations with parameter blocks. Format and evidence: `docs/formats/chameleon-effects.md`.
//!
//! [`Effect::parse`] reads the structure from the resource's main block: the header, the emitters, each emitter's tasks (an
//! operation id and a parameter block) and the stage lists that say in which order the tasks run. The operations whose
//! parameters are understood have typed readers in [`decode`] (and [`wheel`] for the wheel spawner).

pub mod collision;
pub mod decode;
pub mod ops;
pub mod wheel;

#[cfg(test)]
mod tests;

/// The magic number at the start of the resource.
pub const MAGIC: u32 = 0x2699_87A1;
/// The stage lists of an emitter: `Update`, `Render`, `RenderShadowMap` and `RenderShadowTest`, in that order.
pub const STAGES: usize = 4;

/// Why a resource is not read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The bytes do not start with [`MAGIC`].
    BadMagic(u32),
    /// A field points outside the resource.
    Truncated(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::BadMagic(found) => write!(f, "not a BearEffect: it starts with {found:#x}"),
            Error::Truncated(what) => write!(f, "the effect ends inside its {what}"),
        }
    }
}

impl std::error::Error for Error {}

/// A parsed effect.
#[derive(Debug, Clone, PartialEq)]
pub struct Effect {
    pub emitters: Vec<Emitter>,
}

/// One operation of an emitter with its parameter block, as the file holds it.
#[derive(Debug, Clone, PartialEq)]
pub struct Task {
    /// The operation (`ops::NAMES`).
    pub op: u8,
    /// The parameter block. Its layout depends on the operation (`decode`).
    pub block: Vec<u8>,
}

/// One stage list of an emitter: the tasks that run in a stage, as `(task index, count)` pairs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stage {
    /// How many of the pairs are in the first group and in the second (the executable runs the two groups differently).
    pub groups: (u8, u8),
    pub pairs: Vec<(u8, u8)>,
}

/// An emitter: the tasks that spawn, move and draw its particles.
#[derive(Debug, Clone, PartialEq)]
pub struct Emitter {
    /// The emitter's flag word.
    pub flags: u16,
    pub tasks: Vec<Task>,
    pub stages: [Option<Stage>; STAGES],
}

impl Effect {
    /// Read an effect from the main block of its resource.
    pub fn parse(main: &[u8]) -> Result<Self, Error> {
        let magic = u32_at(main, 0).ok_or(Error::Truncated("header"))?;
        if magic != MAGIC {
            return Err(Error::BadMagic(magic));
        }
        let count = usize::from(u16_at(main, 4).ok_or(Error::Truncated("header"))?);
        let table = usize::from(u16_at(main, 0x12).ok_or(Error::Truncated("header"))?);
        let mut emitters = Vec::with_capacity(count);
        for index in 0..count {
            let at = u16_at(main, table + 2 * index).ok_or(Error::Truncated("emitter table"))?;
            emitters.push(Emitter::parse(main, usize::from(at))?);
        }
        Ok(Effect { emitters })
    }
}

impl Emitter {
    fn parse(main: &[u8], at: usize) -> Result<Self, Error> {
        let short = Error::Truncated("emitter");
        let flags = u16_at(main, at + 0x0C).ok_or(short.clone())?;
        let count = usize::from(*main.get(at + 0x0D).ok_or(short.clone())?);
        let tasks_at = at + usize::from(u16_at(main, at + 4).ok_or(short.clone())?);
        let stage_table = at + usize::from(u16_at(main, at + 6).ok_or(short.clone())?);
        let offsets: Vec<usize> = (0..count)
            .map(|n| u16_at(main, tasks_at + 2 * n).map(|offset| at + usize::from(offset)))
            .collect::<Option<_>>()
            .ok_or(Error::Truncated("task offsets"))?;
        let ops = main.get(tasks_at + 2 * count..tasks_at + 3 * count).ok_or(Error::Truncated("task operations"))?;
        let mut tasks = Vec::with_capacity(count);
        for (n, op) in ops.iter().enumerate() {
            // A block runs to the next structure that starts after it: another block, or the stage lists.
            let end = offsets
                .iter()
                .copied()
                .filter(|&o| o > offsets[n])
                .chain([stage_table])
                .filter(|&o| o > offsets[n])
                .min();
            let end = end.unwrap_or(main.len()).min(main.len());
            let block = main.get(offsets[n]..end).ok_or(Error::Truncated("task block"))?.to_vec();
            tasks.push(Task { op: *op, block });
        }
        let mut stages: [Option<Stage>; STAGES] = Default::default();
        for (n, slot) in stages.iter_mut().enumerate() {
            let list = u16_at(main, stage_table + 2 * n).ok_or(Error::Truncated("stage table"))?;
            if list == 0 {
                continue;
            }
            let start = at + usize::from(list);
            let head = main.get(start..start + 4).ok_or(Error::Truncated("stage list"))?;
            let pairs = usize::from(head[0]) + usize::from(head[1]);
            let body = main.get(start + 4..start + 4 + 2 * pairs).ok_or(Error::Truncated("stage list"))?;
            *slot = Some(Stage {
                groups: (head[0], head[1]),
                pairs: body.as_chunks::<2>().0.iter().map(|p| (p[0], p[1])).collect(),
            });
        }
        Ok(Emitter { flags, tasks, stages })
    }

    /// The first task of operation `op`.
    pub fn task(&self, op: u8) -> Option<&Task> {
        self.tasks.iter().find(|task| task.op == op)
    }

    /// The operation that draws the particles (the last task).
    pub fn render_op(&self) -> Option<u8> {
        self.tasks.last().map(|task| task.op)
    }

    /// The names of the tasks, in order.
    pub fn task_names(&self) -> Vec<&'static str> {
        self.tasks.iter().map(|task| ops::name(task.op).unwrap_or("?")).collect()
    }
}

pub(crate) fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}

pub(crate) fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

pub(crate) fn f32_at(bytes: &[u8], at: usize) -> Option<f32> {
    u32_at(bytes, at).map(f32::from_bits)
}
