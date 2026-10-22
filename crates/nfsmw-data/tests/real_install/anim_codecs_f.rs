//! The DeltaF1 and DeltaF3 channels of the install, decoded with `blackbox_anim` against `docs/specs/eagl-animation-codecs.md`
//! sections 8 to 10. The banks are walked as in `anim_banks.rs`, whose census gives 90 DeltaF3 and 39 DeltaF1 child
//! records. The airliner `_t` channel is checked against the spec's worked example and its smoothness numbers.

use std::collections::BTreeMap;

use blackbox_anim::codec::{Channel, DELTA_F1, DELTA_F3, read_leaf};
use blackbox_anim::{Bank, Compound, Object};

use super::anim_banks::{EAGL_ANIMATIONS, bank_files};
use crate::{install, unwrapped};

/// The DeltaF3 and DeltaF1 child records of the install, from the census in `docs/specs/eagl-animation-bank.md` section 11.
const DELTA_F3_RECORDS: usize = 90;
const DELTA_F1_RECORDS: usize = 39;
const AIRLINER_T: &str = "ANM_AirlinerA_10_XO_AirlinerA_1b_LB_10_t";
const AIRLINER_S: &str = "ANM_AirlinerA_10_XO_AirlinerA_1b_LB_10_s";
/// The frames of the airliner `_t` channel, and the bin length of its DeltaF3 leaf (2^3).
const AIRLINER_T_FRAMES: usize = 51;
const BIN_FRAMES: usize = 8;
/// The physical range of each component of the airliner `_t` channel (spec section 14, example B).
const AIRLINER_T_RANGE: [f64; 3] = [4643.01416, 3645.07251, 822.481384];

/// The floats one dof index writes for a binned codec: one for DeltaF1, three for DeltaF3.
fn width(type_id: u16) -> usize {
    if type_id == DELTA_F3 { 3 } else { 1 }
}

/// Samples every frame of `channel`. The slots it writes must be exactly the dof slots, and each one must be finite.
fn assert_finite(channel: &dyn Channel, type_id: u16, label: &str) {
    let dofs = channel.dofs();
    let len = dofs.iter().map(|&dof| usize::from(dof) + width(type_id)).max().unwrap_or(0);
    let expected = dofs.len() * width(type_id);
    for frame in 0..channel.frames() {
        let mut out = vec![f32::NAN; len];
        channel.sample(frame as f32, &mut out);
        let written: Vec<f32> = out.into_iter().filter(|value| !value.is_nan()).collect();
        assert_eq!(written.len(), expected, "{label} frame {frame}: slots written");
        assert!(written.iter().all(|value| value.is_finite()), "{label} frame {frame}: a value is not finite");
    }
}

/// The object and the compound of the animation `name` in the first bank of `data` that holds it.
fn find_compound(data: &[u8], name: &str) -> Option<(Object, Compound)> {
    blackbox_chunk::find_all(data, EAGL_ANIMATIONS).into_iter().find_map(|chunk| {
        let object = Object::read(chunk.payload).ok()?;
        let bank = Bank::read(&object).ok()?;
        let entry = bank.find(name)?;
        let compound = Compound::read(&object, entry.offset).ok()?;
        Some((object, compound))
    })
}

/// The frames of a three-component channel at dof `dof`, as the sampled `[x, y, z]` of every frame.
fn translations(channel: &dyn Channel, dof: usize) -> Vec<[f64; 3]> {
    (0..channel.frames())
        .map(|frame| {
            let mut out = [0.0f32; 64];
            channel.sample(frame as f32, &mut out);
            [f64::from(out[dof]), f64::from(out[dof + 1]), f64::from(out[dof + 2])]
        })
        .collect()
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn every_delta_f1_and_f3_channel_decodes() {
    let Some(dir) = install() else { return };
    let mut records: BTreeMap<u16, usize> = BTreeMap::new();
    for (group, rel) in bank_files(&dir) {
        let data = unwrapped(&dir, &rel);
        for chunk in blackbox_chunk::find_all(&data, EAGL_ANIMATIONS) {
            let at = chunk.offset;
            let object = Object::read(chunk.payload).unwrap_or_else(|e| panic!("{rel} at {at:#x}: {e}"));
            let bank = Bank::read(&object).unwrap_or_else(|e| panic!("{rel} at {at:#x}: {e}"));
            for animation in bank.iter() {
                let name = &animation.name;
                let compound = Compound::read(&object, animation.offset)
                    .unwrap_or_else(|e| panic!("{rel}: {name} at {:#x}: {e}", animation.offset));
                for child in
                    compound.children.iter().filter(|child| child.type_id == DELTA_F1 || child.type_id == DELTA_F3)
                {
                    let label = format!("{group} {rel}: {name} at {:#x}", child.offset);
                    let channel =
                        read_leaf(&object, child.type_id, child.offset).unwrap_or_else(|e| panic!("{label}: {e}"));
                    assert!(channel.frames() > 0, "{label}: no frames");
                    assert_finite(channel.as_ref(), child.type_id, &label);
                    *records.entry(child.type_id).or_default() += 1;
                }
            }
        }
    }

    let f3 = records.get(&DELTA_F3).copied().unwrap_or(0);
    let f1 = records.get(&DELTA_F1).copied().unwrap_or(0);
    eprintln!("DeltaF3 (type 20) child records decoded: {f3}");
    eprintln!("DeltaF1 (type 21) child records decoded: {f1}");
    assert_eq!(f3, DELTA_F3_RECORDS, "DeltaF3 child records");
    assert_eq!(f1, DELTA_F1_RECORDS, "DeltaF1 child records");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn airliner_translation_matches_the_worked_example_and_smoothness() {
    let Some(dir) = install() else { return };
    let data = unwrapped(&dir, "TRACKS/L2RA.BUN");
    let (object, compound) = find_compound(&data, AIRLINER_T).expect("the airliner _t animation is in L2RA.BUN");
    assert_eq!(compound.children.len(), 1, "one child channel");
    let child = compound.children[0];
    assert_eq!(child.type_id, DELTA_F3, "the child is a DeltaF3 channel");
    let channel = read_leaf(&object, DELTA_F3, child.offset).expect("the DeltaF3 leaf decodes");
    assert_eq!(channel.frames() as usize, AIRLINER_T_FRAMES);
    assert_eq!(channel.dofs(), [20], "bone 1, slots 8 to 10");

    // Spec section 14, example B: frame 0 is bin 0's physical key, frame 1 adds the first delta row.
    let frames = translations(channel.as_ref(), 20);
    let frame0 = [-0.36752995, 7.8517527, 12.832686];
    let frame1 = [-19.959046, -36.83224, 17.716608];
    for (got, want) in frames[0].iter().zip(frame0) {
        assert!((got - want).abs() < 1e-3, "frame 0 is {frames:?}");
    }
    for (got, want) in frames[1].iter().zip(frame1) {
        assert!((got - want).abs() < 1e-3, "frame 1 is {frames:?}");
    }

    // Spec section 10, row (a) with the decomp physical form. The spec does not define "mean step" in words. The
    // reading that gives its 0.063 is the mean over steps of the largest component fraction, and the max is the largest
    // component fraction. The boundary numbers are per component over the six bin boundaries (frames 7 to 8, 15 to 16,
    // ...), each step as a fraction of its component's range.
    let mut per_step_max = Vec::new();
    let mut all_steps = Vec::new();
    let mut boundaries = Vec::new();
    for frame in 0..AIRLINER_T_FRAMES - 1 {
        let mut largest: f64 = 0.0;
        for c in 0..3 {
            let step = (frames[frame + 1][c] - frames[frame][c]).abs() / AIRLINER_T_RANGE[c];
            largest = largest.max(step);
            all_steps.push(step);
            if (frame + 1) % BIN_FRAMES == 0 {
                boundaries.push(step);
            }
        }
        per_step_max.push(largest);
    }
    let mean = per_step_max.iter().sum::<f64>() / per_step_max.len() as f64;
    let max = all_steps.iter().copied().fold(0.0, f64::max);
    let boundary_mean = boundaries.iter().sum::<f64>() / boundaries.len() as f64;
    let boundary_max = boundaries.iter().copied().fold(0.0, f64::max);
    eprintln!("airliner _t: mean step {mean:.4}, max step {max:.4} of range");
    eprintln!(
        "airliner _t: {} bin boundaries, mean {boundary_mean:.4}, max {boundary_max:.4} of range",
        boundaries.len()
    );
    assert_eq!(boundaries.len(), 18, "six boundaries times three components");
    assert!((mean - 0.063).abs() < 0.005, "mean step {mean} (spec 0.063)");
    assert!((max - 0.103).abs() < 0.01, "max step {max} (spec 0.103)");
    assert!((boundary_mean - 0.041).abs() < 0.005, "boundary mean {boundary_mean} (spec 0.041)");
    assert!((boundary_max - 0.082).abs() < 0.01, "boundary max {boundary_max} (spec 0.082)");

    // The `_s` channel of the airliner is a DeltaF3 with no animated bone and one constant bone: (1, 1, 1) at dof 12.
    let (object, compound) = find_compound(&data, AIRLINER_S).expect("the airliner _s animation is in L2RA.BUN");
    let child = compound.children[0];
    assert_eq!(child.type_id, DELTA_F3, "the _s child is a DeltaF3 channel");
    let channel = read_leaf(&object, DELTA_F3, child.offset).expect("the _s DeltaF3 leaf decodes");
    assert_eq!(channel.dofs(), [12]);
    let mut out = [0.0f32; 16];
    channel.sample(0.0, &mut out);
    assert_eq!(&out[12..15], [1.0, 1.0, 1.0]);
}
