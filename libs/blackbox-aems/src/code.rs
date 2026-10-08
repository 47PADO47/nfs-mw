//! The module code: the x86 subset the nodes' scheduling is compiled to, decoded once. Format:
//! `docs/formats/aems.md` ("The code").

use std::collections::HashMap;

use crate::error::{Error, Result};

/// One instruction. `disp` is the signed offset from `esi`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Op {
    /// `push esi`.
    Push,
    /// `pop esi`.
    Pop,
    Ret,
    /// `mov esi, [esp+8]`: the instance pointer.
    LoadArg,
    /// `call N`: node function `N` on the node the top of the stack points at.
    Call(u32),
    /// `mov [esi+disp], eax`.
    Store(i32),
    /// `mov byte [esi+disp], value`.
    StoreByte(i32, u8),
    /// `mov eax, [esi+disp]`.
    LoadEax(i32),
    /// `mov ecx, [esi+disp]`.
    LoadEcx(i32),
    /// `add / sub / imul eax, [esi+disp]`.
    AddEax(i32),
    SubEax(i32),
    MulEax(i32),
    /// `cmp eax, ecx`.
    Cmp,
    /// `mov eax, ecx`.
    MovEaxEcx,
    /// `jl / jg` to the instruction with this index (always forward).
    JumpLess(usize),
    JumpGreater(usize),
    /// `add esi, imm`.
    AddEsi(i32),
    /// `add esp, imm`: drop `imm / 4` pushed words.
    AddEsp(i32),
}

struct Cursor<'a> {
    data: &'a [u8],
    at: usize,
}

impl Cursor<'_> {
    fn byte(&self, ahead: usize) -> Result<u8> {
        self.data.get(self.at + ahead).copied().ok_or(Error::Fault(self.at + ahead))
    }
    fn i8(&self, ahead: usize) -> Result<i32> {
        Ok(i32::from(self.byte(ahead)? as i8))
    }
    fn i32(&self, ahead: usize) -> Result<i32> {
        let mut b = [0u8; 4];
        for (i, slot) in b.iter_mut().enumerate() {
            *slot = self.byte(ahead + i)?;
        }
        Ok(i32::from_le_bytes(b))
    }
}

/// A jump whose target address is known but whose index is not yet.
enum Pending {
    Less(usize),
    Greater(usize),
}

/// Decodes the code starting at `start`, up to `ret` or `limit`.
pub(crate) fn decode(data: &[u8], start: usize, limit: usize) -> Result<Vec<Op>> {
    let mut cur = Cursor { data, at: start };
    let mut ops = Vec::new();
    let mut index_of: HashMap<usize, usize> = HashMap::new();
    let mut jumps: Vec<(usize, Pending)> = Vec::new();
    while cur.at < limit {
        index_of.insert(cur.at, ops.len());
        let (op, len) = decode_one(&cur, &mut jumps, ops.len())?;
        cur.at += len;
        ops.push(op);
        if op == Op::Ret {
            break;
        }
    }
    index_of.insert(cur.at, ops.len());
    for (slot, pending) in jumps {
        let (Pending::Less(target) | Pending::Greater(target)) = pending;
        let to = *index_of.get(&target).ok_or(Error::Fault(target))?;
        if to <= slot {
            return Err(Error::Fault(target));
        }
        ops[slot] = match pending {
            Pending::Less(_) => Op::JumpLess(to),
            Pending::Greater(_) => Op::JumpGreater(to),
        };
    }
    Ok(ops)
}

fn decode_one(c: &Cursor<'_>, jumps: &mut Vec<(usize, Pending)>, slot: usize) -> Result<(Op, usize)> {
    let (b0, b1) = (c.byte(0)?, c.byte(1).unwrap_or(0));
    let bad = || Error::BadCode { at: c.at, opcode: b0 };
    match (b0, b1) {
        (0x56, _) => Ok((Op::Push, 1)),
        (0x5E, _) => Ok((Op::Pop, 1)),
        (0xC3, _) => Ok((Op::Ret, 1)),
        (0x8B, 0x74) if c.byte(2)? == 0x24 && c.byte(3)? == 0x08 => Ok((Op::LoadArg, 4)),
        (0xE8, _) => Ok((Op::Call(c.i32(1)? as u32), 5)),
        (0x89, 0x86) => Ok((Op::Store(c.i32(2)?), 6)),
        (0x89, 0x46) => Ok((Op::Store(c.i8(2)?), 3)),
        (0x89, 0x06) => Ok((Op::Store(0), 2)),
        (0x8B, 0x06) => Ok((Op::LoadEax(0), 2)),
        (0x8B, 0x46) => Ok((Op::LoadEax(c.i8(2)?), 3)),
        (0x8B, 0x86) => Ok((Op::LoadEax(c.i32(2)?), 6)),
        (0x8B, 0x4E) => Ok((Op::LoadEcx(c.i8(2)?), 3)),
        (0x03, 0x46) => Ok((Op::AddEax(c.i8(2)?), 3)),
        (0x2B, 0x46) => Ok((Op::SubEax(c.i8(2)?), 3)),
        (0x0F, 0xAF) if c.byte(2)? == 0x46 => Ok((Op::MulEax(c.i8(3)?), 4)),
        (0x3B, 0xC1) => Ok((Op::Cmp, 2)),
        (0x8B, 0xC1) => Ok((Op::MovEaxEcx, 2)),
        (0x7C, _) => {
            jumps.push((slot, Pending::Less((c.at as i64 + 2 + i64::from(c.i8(1)?)) as usize)));
            Ok((Op::JumpLess(0), 2))
        }
        (0x7F, _) => {
            jumps.push((slot, Pending::Greater((c.at as i64 + 2 + i64::from(c.i8(1)?)) as usize)));
            Ok((Op::JumpGreater(0), 2))
        }
        (0x83, 0xC6) => Ok((Op::AddEsi(c.i8(2)?), 3)),
        (0x81, 0xC6) => Ok((Op::AddEsi(c.i32(2)?), 6)),
        (0x83, 0xC4) => Ok((Op::AddEsp(c.i8(2)?), 3)),
        (0x81, 0xC4) => Ok((Op::AddEsp(c.i32(2)?), 6)),
        (0xC6, 0x06) => Ok((Op::StoreByte(0, c.byte(2)?), 3)),
        (0xC6, 0x46) => Ok((Op::StoreByte(c.i8(2)?, c.byte(3)?), 4)),
        (0xC6, 0x86) => Ok((Op::StoreByte(c.i32(2)?, c.byte(6)?), 7)),
        _ => Err(bad()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_forms_of_the_shipped_code_decode() {
        let code = [
            0x56, 0x8B, 0x74, 0x24, 0x08, // push esi; mov esi, [esp+8]
            0x56, 0xE8, 0x0F, 0x00, 0x00, 0x00, // push esi; call 15
            0x89, 0x86, 0x48, 0x08, 0x00, 0x00, // mov [esi+0x848], eax
            0x89, 0x46, 0x18, // mov [esi+0x18], eax
            0x83, 0xC6, 0x10, // add esi, 0x10
            0x8B, 0x06, 0x8B, 0x4E, 0x04, 0x3B, 0xC1, 0x7C, 0x02, 0x8B, 0xC1, // min2
            0x89, 0x46, 0x10, 0x5E, 0xC3,
        ];
        let ops = decode(&code, 0, code.len()).unwrap();
        assert_eq!(ops[0], Op::Push);
        assert_eq!(ops[1], Op::LoadArg);
        assert_eq!(ops[3], Op::Call(15));
        assert_eq!(ops[4], Op::Store(0x848));
        assert_eq!(ops[5], Op::Store(0x18));
        assert_eq!(ops[6], Op::AddEsi(0x10));
        let jump = ops.iter().position(|o| matches!(o, Op::JumpLess(_))).unwrap();
        assert_eq!(ops[jump], Op::JumpLess(jump + 2), "over the mov");
        assert_eq!(ops.last(), Some(&Op::Ret));
    }

    #[test]
    fn unknown_instructions_and_backward_jumps_are_errors() {
        assert!(matches!(decode(&[0x90], 0, 1), Err(Error::BadCode { opcode: 0x90, .. })));
        // jl to itself.
        assert!(decode(&[0x7C, 0xFE, 0xC3], 0, 3).is_err());
        // Truncated operand.
        assert!(decode(&[0xE8, 0x00], 0, 2).is_err());
    }

    #[test]
    fn code_stops_at_the_limit_without_a_ret() {
        let ops = decode(&[0x56, 0x5E], 0, 2).unwrap();
        assert_eq!(ops, vec![Op::Push, Op::Pop]);
    }
}
