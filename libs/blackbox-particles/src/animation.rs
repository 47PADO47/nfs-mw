//! Quantized square particle atlases; contract: `docs/specs/prop-particle-attributes.md`.
use crate::Rng;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Animation {
    /// Zero for the full texture; otherwise 2, 4, 8 or 16 cells per axis.
    pub grid: u8,
    pub fps: u8,
    pub random_start: bool,
}

impl Animation {
    fn frames(self) -> u32 {
        match self.grid {
            2 | 4 | 8 | 16 => u32::from(self.grid).pow(2),
            _ => 0,
        }
    }

    pub(crate) fn birth(self, rng: &mut Rng) -> (u16, u16) {
        let frames = self.frames();
        if frames == 0 {
            return (0, 0);
        }
        let maximum = (frames - 1) as f32;
        let frame = match self.random_start {
            true => rng.below(maximum),
            false => 0.0,
        };
        ((frame / maximum * 65535.0) as u16, frame as u16)
    }

    pub(crate) fn advance(self, phase: &mut u16, cell: &mut u16, dt: f32) {
        let frames = self.frames();
        if frames == 0 {
            return;
        }
        let increment = (dt * f32::from(self.fps) / frames as f32 * 65535.0) as u32;
        let mut next = u32::from(*phase).wrapping_add(increment);
        if next.wrapping_add(increment) > 65535 {
            next = next.wrapping_sub(65535);
        }
        *phase = next as u16;
        *cell = (f32::from(*phase) / 65535.0 * (frames - 1) as f32) as u16;
    }

    pub(crate) fn uv(self, cell: u16) -> [[f32; 2]; 2] {
        if self.frames() == 0 {
            return [[0.0; 2], [1.0; 2]];
        }
        let grid = u16::from(self.grid);
        let x = f32::from(cell % grid) / f32::from(grid);
        let y = f32::from(cell / grid) / f32::from(grid);
        let width = 1.0 / f32::from(grid);
        let quantize = |v: f32| (v * 65535.0) as u16 as f32 / 65535.0;
        [[quantize(x), quantize(y)], [quantize(x + width), quantize(y + width)]]
    }
}
