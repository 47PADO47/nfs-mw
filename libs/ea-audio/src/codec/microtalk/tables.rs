//! Constant tables of the MicroTalk decoder: reflection coefficients, the two code books and the command table.
//!
// Derived from vgmstream's src/coding/libs/utkdec.c (ISC-style licence) which is derived from
// utkencode by Andrew D'Addesio (Unlicense, public domain):
//
//   Copyright (c) 2008-2025 Adam Gashlin, Fastelbja, Ronny Elfert, bnnm, Christopher Snowhill,
//   NicknineTheEagle, bxaimc, Thealexbarney, CyberBotX, EdnessP, et al.
//
//   Permission to use, copy, modify, and distribute this software for any purpose with or without
//   fee is hereby granted, provided that the above copyright notice and this permission notice
//   appear in all copies.
//
//   THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH REGARD TO THIS
//   SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL
//   THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
//   WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT,
//   NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR
//   PERFORMANCE OF THIS SOFTWARE.

/// Reflection coefficients (rounded), indexed by the 6-bit or 5-bit (+16) codes of a frame header.
pub const RC_TABLE: [f32; 64] = [
    0.000000, -0.996776, -0.990327, -0.983879, -0.977431, -0.970982, -0.964534, -0.958085, -0.951637, -0.930754,
    -0.904960, -0.879167, -0.853373, -0.827579, -0.801786, -0.775992, -0.750198, -0.724405, -0.698611, -0.670635,
    -0.619048, -0.567460, -0.515873, -0.464286, -0.412698, -0.361111, -0.309524, -0.257937, -0.206349, -0.154762,
    -0.103175, -0.051587, 0.000000, 0.051587, 0.103175, 0.154762, 0.206349, 0.257937, 0.309524, 0.361111, 0.412698,
    0.464286, 0.515873, 0.567460, 0.619048, 0.670635, 0.698611, 0.724405, 0.750198, 0.775992, 0.801786, 0.827579,
    0.853373, 0.879167, 0.904960, 0.930754, 0.951637, 0.958085, 0.964534, 0.970982, 0.977431, 0.983879, 0.990327,
    0.996776,
];

/// Next excitation model after a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Model {
    Normal = 0,
    LargePulse = 1,
}

/// Code books: for the next 8 bits of the stream, which command comes next. Index = model.
pub const CODEBOOKS: [[u8; 256]; 2] = [
    [
        4, 6, 5, 9, 4, 6, 5, 13, 4, 6, 5, 10, 4, 6, 5, 17, 4, 6, 5, 9, 4, 6, 5, 14, 4, 6, 5, 10, 4, 6, 5, 21, 4, 6, 5,
        9, 4, 6, 5, 13, 4, 6, 5, 10, 4, 6, 5, 18, 4, 6, 5, 9, 4, 6, 5, 14, 4, 6, 5, 10, 4, 6, 5, 25, 4, 6, 5, 9, 4, 6,
        5, 13, 4, 6, 5, 10, 4, 6, 5, 17, 4, 6, 5, 9, 4, 6, 5, 14, 4, 6, 5, 10, 4, 6, 5, 22, 4, 6, 5, 9, 4, 6, 5, 13, 4,
        6, 5, 10, 4, 6, 5, 18, 4, 6, 5, 9, 4, 6, 5, 14, 4, 6, 5, 10, 4, 6, 5, 0, 4, 6, 5, 9, 4, 6, 5, 13, 4, 6, 5, 10,
        4, 6, 5, 17, 4, 6, 5, 9, 4, 6, 5, 14, 4, 6, 5, 10, 4, 6, 5, 21, 4, 6, 5, 9, 4, 6, 5, 13, 4, 6, 5, 10, 4, 6, 5,
        18, 4, 6, 5, 9, 4, 6, 5, 14, 4, 6, 5, 10, 4, 6, 5, 26, 4, 6, 5, 9, 4, 6, 5, 13, 4, 6, 5, 10, 4, 6, 5, 17, 4, 6,
        5, 9, 4, 6, 5, 14, 4, 6, 5, 10, 4, 6, 5, 22, 4, 6, 5, 9, 4, 6, 5, 13, 4, 6, 5, 10, 4, 6, 5, 18, 4, 6, 5, 9, 4,
        6, 5, 14, 4, 6, 5, 10, 4, 6, 5, 2,
    ],
    [
        4, 11, 7, 15, 4, 12, 8, 19, 4, 11, 7, 16, 4, 12, 8, 23, 4, 11, 7, 15, 4, 12, 8, 20, 4, 11, 7, 16, 4, 12, 8, 27,
        4, 11, 7, 15, 4, 12, 8, 19, 4, 11, 7, 16, 4, 12, 8, 24, 4, 11, 7, 15, 4, 12, 8, 20, 4, 11, 7, 16, 4, 12, 8, 1,
        4, 11, 7, 15, 4, 12, 8, 19, 4, 11, 7, 16, 4, 12, 8, 23, 4, 11, 7, 15, 4, 12, 8, 20, 4, 11, 7, 16, 4, 12, 8, 28,
        4, 11, 7, 15, 4, 12, 8, 19, 4, 11, 7, 16, 4, 12, 8, 24, 4, 11, 7, 15, 4, 12, 8, 20, 4, 11, 7, 16, 4, 12, 8, 3,
        4, 11, 7, 15, 4, 12, 8, 19, 4, 11, 7, 16, 4, 12, 8, 23, 4, 11, 7, 15, 4, 12, 8, 20, 4, 11, 7, 16, 4, 12, 8, 27,
        4, 11, 7, 15, 4, 12, 8, 19, 4, 11, 7, 16, 4, 12, 8, 24, 4, 11, 7, 15, 4, 12, 8, 20, 4, 11, 7, 16, 4, 12, 8, 1,
        4, 11, 7, 15, 4, 12, 8, 19, 4, 11, 7, 16, 4, 12, 8, 23, 4, 11, 7, 15, 4, 12, 8, 20, 4, 11, 7, 16, 4, 12, 8, 28,
        4, 11, 7, 15, 4, 12, 8, 19, 4, 11, 7, 16, 4, 12, 8, 24, 4, 11, 7, 15, 4, 12, 8, 20, 4, 11, 7, 16, 4, 12, 8, 3,
    ],
];

/// One excitation command.
#[derive(Debug, Clone, Copy)]
pub struct Command {
    pub next_model: Model,
    /// Bits consumed by the code.
    pub code_size: u32,
    /// Pulse magnitude for commands 4 and up (and for 0/1, the escape for larger ones).
    pub pulse: f32,
}

/// The 29 excitation commands.
pub const COMMANDS: [Command; 29] = [
    Command { next_model: Model::LargePulse, code_size: 8, pulse: 0.0 },
    Command { next_model: Model::LargePulse, code_size: 7, pulse: 0.0 },
    Command { next_model: Model::Normal, code_size: 8, pulse: 0.0 },
    Command { next_model: Model::Normal, code_size: 7, pulse: 0.0 },
    Command { next_model: Model::Normal, code_size: 2, pulse: 0.0 },
    Command { next_model: Model::Normal, code_size: 2, pulse: -1.0 },
    Command { next_model: Model::Normal, code_size: 2, pulse: 1.0 },
    Command { next_model: Model::Normal, code_size: 3, pulse: -1.0 },
    Command { next_model: Model::Normal, code_size: 3, pulse: 1.0 },
    Command { next_model: Model::LargePulse, code_size: 4, pulse: -2.0 },
    Command { next_model: Model::LargePulse, code_size: 4, pulse: 2.0 },
    Command { next_model: Model::LargePulse, code_size: 3, pulse: -2.0 },
    Command { next_model: Model::LargePulse, code_size: 3, pulse: 2.0 },
    Command { next_model: Model::LargePulse, code_size: 5, pulse: -3.0 },
    Command { next_model: Model::LargePulse, code_size: 5, pulse: 3.0 },
    Command { next_model: Model::LargePulse, code_size: 4, pulse: -3.0 },
    Command { next_model: Model::LargePulse, code_size: 4, pulse: 3.0 },
    Command { next_model: Model::LargePulse, code_size: 6, pulse: -4.0 },
    Command { next_model: Model::LargePulse, code_size: 6, pulse: 4.0 },
    Command { next_model: Model::LargePulse, code_size: 5, pulse: -4.0 },
    Command { next_model: Model::LargePulse, code_size: 5, pulse: 4.0 },
    Command { next_model: Model::LargePulse, code_size: 7, pulse: -5.0 },
    Command { next_model: Model::LargePulse, code_size: 7, pulse: 5.0 },
    Command { next_model: Model::LargePulse, code_size: 6, pulse: -5.0 },
    Command { next_model: Model::LargePulse, code_size: 6, pulse: 5.0 },
    Command { next_model: Model::LargePulse, code_size: 8, pulse: -6.0 },
    Command { next_model: Model::LargePulse, code_size: 8, pulse: 6.0 },
    Command { next_model: Model::LargePulse, code_size: 7, pulse: -6.0 },
    Command { next_model: Model::LargePulse, code_size: 7, pulse: 6.0 },
];
