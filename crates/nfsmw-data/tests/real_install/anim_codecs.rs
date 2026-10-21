//! The airliner's DeltaQuat channel in `TRACKS/L2RA.BUN` against `docs/specs/eagl-animation-codecs.md` section 7.
//!
//! The bank is walked here by hand: the bank header and the animation table per `docs/specs/eagl-animation-bank.md`
//! sections 3 and 4. Only the first bank that holds the animation is read.

use blackbox_anim::Object;
use blackbox_anim::codec::read_leaf;

const AIRLINER: &str = "ANM_AirlinerA_10_XO_AirlinerA_1b_LB_10_q";
const BANK_SYMBOL: &str = "__AnimationBank:::bank";
const COMPOUND: u16 = 15;
const DELTA_QUAT: u16 = 11;
const FRAMES: u32 = 51;
/// The dof index of the quaternion: bone 1, slots 4 to 7.
const DOF: u16 = 16;

/// The compound entry of the animation `name` in the bank of `object`, or `None` when the bank has no such name.
fn bank_entry(object: &Object, name: &str) -> Option<u32> {
    let header = object.symbol(BANK_SYMBOL)?.value as usize;
    let count = object.u32_at(header + 4).ok()?;
    let anims = object.ptr_at(header + 0x10).ok()??;
    let names = object.ptr_at(header + 0x14).ok()??;
    (0..count as usize).find_map(|i| {
        let text_at = object.ptr_at(names as usize + 4 * i).ok()??;
        if object.cstr_at(text_at as usize).ok()? != name {
            return None;
        }
        object.ptr_at(anims as usize + 4 * i).ok()?
    })
}

/// The object and the compound entry of the airliner animation in `L2RA.BUN`.
fn find_airliner(data: &[u8]) -> Option<(Object, u32)> {
    blackbox_chunk::find_all(data, blackbox_chunk::ids::EAGL_ANIMATIONS).into_iter().find_map(|chunk| {
        let object = Object::read(chunk.payload).ok()?;
        let compound = bank_entry(&object, AIRLINER)?;
        Some((object, compound))
    })
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn airliner_delta_quat_decodes() {
    let Some(dir) = crate::install() else { return };
    let data = crate::unwrapped(&dir, "TRACKS/L2RA.BUN");
    let (object, compound) = find_airliner(&data).expect("the airliner animation is in a bank of L2RA.BUN");

    assert_eq!(object.u16_at(compound as usize).unwrap(), COMPOUND);
    assert_eq!(object.u16_at(compound as usize + 8).unwrap(), 1, "one child channel");
    assert_eq!(u32::from(object.u16_at(compound as usize + 0x0A).unwrap()), FRAMES);
    let leaf = object.ptr_at(compound as usize + 0x0C).unwrap().expect("a child pointer");
    assert_eq!(object.u16_at(leaf as usize).unwrap(), DELTA_QUAT, "the child is a DeltaQuat channel");

    let channel = read_leaf(&object, DELTA_QUAT, leaf).expect("the DeltaQuat leaf decodes");
    assert_eq!(channel.frames(), FRAMES);
    assert_eq!(channel.dofs(), [DOF]);

    let quats: Vec<[f32; 4]> = (0..FRAMES)
        .map(|frame| {
            let mut out = [0.0f32; 24];
            channel.sample(frame as f32, &mut out);
            let q = &out[DOF as usize..DOF as usize + 4];
            [q[0], q[1], q[2], q[3]]
        })
        .collect();

    for (frame, q) in quats.iter().enumerate() {
        let norm = q.iter().map(|c| c * c).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-3, "frame {frame}: |q| = {norm}");
    }

    let angles: Vec<f32> = quats
        .windows(2)
        .map(|pair| {
            let dot: f32 = pair[0].iter().zip(&pair[1]).map(|(a, b)| a * b).sum();
            let na = pair[0].iter().map(|c| c * c).sum::<f32>().sqrt();
            let nb = pair[1].iter().map(|c| c * c).sum::<f32>().sqrt();
            2.0 * (dot / (na * nb)).abs().min(1.0).acos()
        })
        .collect();
    let mean = angles.iter().sum::<f32>() / angles.len() as f32;
    let max = angles.iter().copied().fold(0.0f32, f32::max);
    println!("airliner _q: {FRAMES} frames, mean step {mean:.4} rad, max step {max:.4} rad");
    assert!(mean < 0.15, "mean step {mean} rad (spec 0.1209)");
    assert!(max < 0.2, "max step {max} rad (spec 0.1838)");
}
