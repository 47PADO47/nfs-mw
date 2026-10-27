//! The post effect states of the Chameleon engine (NFS Most Wanted 2012): a [`State`] binds a [`Keyframe`] (vignette, bloom
//! and general values) and a [`ColourCube`] to a live value of the game (`Reporter.NitrousState.UsingNitrous`, `Status.HeatValue`,
//! ...). Format and evidence: `docs/formats/chameleon-nitrous.md`. Everything here takes the bytes of a resource's main block
//! (or of the cube's block) and finds the fields by offset: the field names of the file are hashed, so what a number means is
//! a reading, marked in the docs as such.
//!
//! Nothing here opens a file: [`find_bound`] searches a parsed bundle for the state turned on by a live value, and follows
//! its imports to the keyframe and the cube.

use chameleon_bundle::{Bundle, ResourceKind};

#[cfg(test)]
mod install_tests;
#[cfg(test)]
mod tests;

/// The size of a keyframe's main block.
pub const KEYFRAME_LEN: usize = 400;
/// The offsets of the keyframe's fields, in its main block.
mod at {
    pub const VIGNETTE_A: usize = 0x20;
    pub const VIGNETTE_B: usize = 0x30;
    pub const VIGNETTE_INNER: usize = 0x40;
    pub const VIGNETTE_OUTER: usize = 0x50;
    pub const VIGNETTE_TAIL: usize = 0x64;
    pub const BLOOM_COLOUR: usize = 0x90;
    pub const BLOOM_A: usize = 0xA8;
    pub const BLOOM_B: usize = 0xBC;
    pub const GENERAL: usize = 0xE4;
    /// Inside a state: the blend time and the weight.
    pub const STATE_BLEND: usize = 0xE8;
    pub const STATE_WEIGHT: usize = 0xEC;
}

fn float(block: &[u8], offset: usize) -> Option<f32> {
    let bytes = block.get(offset..offset + 4)?;
    let value = f32::from_le_bytes(bytes.try_into().ok()?);
    value.is_finite().then_some(value)
}

fn floats<const N: usize>(block: &[u8], offset: usize) -> Option<[f32; N]> {
    let mut out = [0.0; N];
    for (index, slot) in out.iter_mut().enumerate() {
        *slot = float(block, offset + index * 4)?;
    }
    Some(out)
}

/// The vignette part of a keyframe. The roles are readings from the executable's names (`VignetteCentre`, `VignetteAmount`,
/// `VignetteInnerColour`, `VignetteOuterColour`, `VignetteAngle`, `VignetteSharpness`) and from how the values differ between
/// states **[unconfirmed]**.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vignette {
    /// Two numbers at `+0x20` (0.5, 0.3 by default; 0.526, 0.51 in the nitrous).
    pub a: [f32; 2],
    /// Two numbers at `+0x30` (0.7, 0.555 by default; 0.459, 0.522 in the nitrous): read as how far from the centre the
    /// vignette starts, as a share of the screen.
    pub b: [f32; 2],
    /// The colour at `+0x40` (white in every state).
    pub inner: [f32; 4],
    /// The colour at `+0x50`: what the screen's edge goes to (grey 0.45 by default, a dark teal in the nitrous).
    pub outer: [f32; 4],
    /// Five numbers at `+0x64` (the nitrous's last is 0.258).
    pub tail: [f32; 5],
}

/// The bloom part of a keyframe.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bloom {
    /// The colour at `+0x90` (grey 0.4 by default; a light blue in the nitrous).
    pub colour: [f32; 4],
    /// The numbers at `+0xA8` and `+0xBC`.
    pub a: f32,
    pub b: f32,
}

/// A `Genesys.Gen.PostFxKeyframe` object.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Keyframe {
    pub vignette: Vignette,
    pub bloom: Bloom,
    /// Three numbers at `+0xE4` of the General part.
    pub general: [f32; 3],
}

impl Keyframe {
    /// Read a keyframe from the main block of its resource; `None` when the block is not the size of a keyframe or holds a
    /// number that is not finite.
    pub fn parse(block: &[u8]) -> Option<Self> {
        if block.len() != KEYFRAME_LEN {
            return None;
        }
        Some(Self {
            vignette: Vignette {
                a: floats(block, at::VIGNETTE_A)?,
                b: floats(block, at::VIGNETTE_B)?,
                inner: floats(block, at::VIGNETTE_INNER)?,
                outer: floats(block, at::VIGNETTE_OUTER)?,
                tail: floats(block, at::VIGNETTE_TAIL)?,
            },
            bloom: Bloom {
                colour: floats(block, at::BLOOM_COLOUR)?,
                a: float(block, at::BLOOM_A)?,
                b: float(block, at::BLOOM_B)?,
            },
            general: floats(block, at::GENERAL)?,
        })
    }
}

/// A state of the post effect config (`Genesys.Gen.PostFXState`): the keyframe and the cube it blends in come from its
/// imports, the live value that turns it on from the strings of its block.
#[derive(Debug, Clone, PartialEq)]
pub struct State {
    /// The expression that turns the state on (`Reporter.NitrousState.UsingNitrous`); `None` for the default state.
    pub expression: Option<String>,
    /// The number at `+0xE8`: 0 for the default state, 2 for the nitrous, the overdrive, the wreck and the escape, 3 for the
    /// cooldown, 20 for the heat. Read as the seconds the state takes to blend in **[unconfirmed]**.
    pub blend_seconds: f32,
    /// The number at `+0xEC`, 1 in every state.
    pub weight: f32,
}

impl State {
    /// Read a state from the main block of its resource.
    pub fn parse(block: &[u8]) -> Option<Self> {
        Some(Self {
            expression: expression(block),
            blend_seconds: float(block, at::STATE_BLEND)?,
            weight: float(block, at::STATE_WEIGHT)?,
        })
    }

    /// Whether the state is turned on by an expression that holds `needle`.
    pub fn is_bound_to(&self, needle: &str) -> bool {
        self.expression.as_deref().is_some_and(|text| text.contains(needle))
    }
}

/// The first run of printable characters after the state's fixed fields (the expression string).
fn expression(block: &[u8]) -> Option<String> {
    let tail = block.get(0xF8..)?;
    let start = tail.iter().position(|b| (0x21..0x7F).contains(b))?;
    let run: Vec<u8> = tail[start..].iter().copied().take_while(|b| (0x20..0x7F).contains(b)).collect();
    (run.len() >= 4).then(|| String::from_utf8_lossy(&run).into_owned())
}

/// The edge of a colour cube.
pub const CUBE_SIZE: usize = 16;
/// The size of a colour cube's data block: a 16-byte header and `16 x 16 x 16` entries of 3 bytes.
pub const CUBE_LEN: usize = 16 + CUBE_SIZE * CUBE_SIZE * CUBE_SIZE * 3;

/// A colour cube (resource kind `0x52`): the entries along the grey diagonal, which is all the grade the host can take.
/// The bytes of an entry are stored blue, green, red (the heat cube is orange and the cooldown cube blue only in that order,
/// and the identity cube reads that way) **[unconfirmed]**.
#[derive(Debug, Clone, PartialEq)]
pub struct ColourCube {
    /// The grey diagonal, `[red, green, blue]` 0 to 1, at `CUBE_SIZE` positions.
    pub diagonal: [[f32; 3]; CUBE_SIZE],
}

/// A grade a host can draw: every colour multiplied by `tint`, then the contrast moved about mid grey.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grade {
    pub tint: [f32; 3],
    pub contrast: f32,
}

impl ColourCube {
    /// Read the cube from its data block; `None` when the block is not the size of a cube.
    pub fn parse(block: &[u8]) -> Option<Self> {
        if block.len() != CUBE_LEN {
            return None;
        }
        let mut diagonal = [[0.0; 3]; CUBE_SIZE];
        for (step, slot) in diagonal.iter_mut().enumerate() {
            let entry = 16 + step * (1 + CUBE_SIZE + CUBE_SIZE * CUBE_SIZE) * 3;
            let [blue, green, red] = [block[entry], block[entry + 1], block[entry + 2]].map(|b| f32::from(b) / 255.0);
            *slot = [red, green, blue];
        }
        Some(Self { diagonal })
    }

    /// Whether the diagonal is the identity (every entry at its own position, within a byte).
    pub fn is_identity(&self) -> bool {
        self.diagonal.iter().enumerate().all(|(step, rgb)| {
            let identity = step as f32 / (CUBE_SIZE - 1) as f32;
            rgb.iter().all(|c| (c - identity).abs() <= 1.5 / 255.0)
        })
    }

    /// The grade that fits the diagonal best: for each channel a straight line `out = slope * in + offset` over the mid
    /// positions (the ends clip), the contrast from the offsets all channels share (`offset = 0.5 * (1 - contrast)`) and the
    /// tint from the slopes (`slope = tint * contrast`). The fit is the host's closest grade, not the cube: a cube bends.
    pub fn grade(&self) -> Grade {
        let steps: Vec<usize> = (3..CUBE_SIZE - 3).collect();
        let xs: Vec<f32> = steps.iter().map(|s| *s as f32 / (CUBE_SIZE - 1) as f32).collect();
        let fits: Vec<(f32, f32)> = (0..3)
            .map(|channel| {
                let ys: Vec<f32> = steps.iter().map(|s| self.diagonal[*s][channel]).collect();
                line(&xs, &ys)
            })
            .collect();
        let contrast = (1.0 - 2.0 * fits.iter().map(|fit| fit.1).sum::<f32>() / 3.0).max(0.05);
        Grade { tint: [fits[0].0 / contrast, fits[1].0 / contrast, fits[2].0 / contrast], contrast }
    }
}

/// The least squares line through the points, as `(slope, offset)`.
fn line(xs: &[f32], ys: &[f32]) -> (f32, f32) {
    let n = xs.len() as f32;
    let (mean_x, mean_y) = (xs.iter().sum::<f32>() / n, ys.iter().sum::<f32>() / n);
    let spread: f32 = xs.iter().map(|x| (x - mean_x) * (x - mean_x)).sum();
    if spread <= 0.0 {
        return (1.0, 0.0);
    }
    let slope = xs.iter().zip(ys).map(|(x, y)| (x - mean_x) * (y - mean_y)).sum::<f32>() / spread;
    (slope, mean_y - slope * mean_x)
}

/// A state with the keyframe and the cube it blends in.
#[derive(Debug, Clone, PartialEq)]
pub struct Bound {
    pub state: State,
    pub keyframe: Keyframe,
    /// `None` when the state imports no cube that reads.
    pub cube: Option<ColourCube>,
}

/// The state of `bundle` turned on by an expression that holds `needle`, with the keyframe and the cube its imports name;
/// `None` when there is no such state or it imports no keyframe.
pub fn find_bound(bundle: &Bundle, needle: &str) -> Option<Bound> {
    let object = ResourceKind::GenesysObject.id();
    let cube_kind = ResourceKind::ColourCube.id();
    for (index, entry) in bundle.entries().iter().enumerate() {
        if entry.kind != object || entry.import_count == 0 {
            continue;
        }
        let Ok(resource) = bundle.resource(index) else { continue };
        let Some(state) = State::parse(&resource.blocks[0]) else { continue };
        if !state.is_bound_to(needle) {
            continue;
        }
        let mut keyframe = None;
        let mut cube = None;
        for import in &resource.imports {
            let Some(at) = bundle.find(import.id) else { continue };
            let Ok(target) = bundle.resource(at) else { continue };
            if target.entry.kind == cube_kind {
                cube = target
                    .blocks
                    .iter()
                    .find(|block| block.len() == CUBE_LEN)
                    .and_then(|block| ColourCube::parse(block));
                continue;
            }
            if target.entry.kind == object && keyframe.is_none() {
                keyframe = Keyframe::parse(&target.blocks[0]);
            }
        }
        return Some(Bound { state, keyframe: keyframe?, cube });
    }
    None
}
