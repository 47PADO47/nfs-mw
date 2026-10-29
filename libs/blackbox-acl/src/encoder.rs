//! A small encoder of the buffers the decoder reads, for the tests. It follows the layout in `layout.rs`; it makes no
//! attempt to compress well (no search for bit rates, no error metric).

use crate::qvv::Qvv;

/// What to encode.
pub struct Spec {
    /// `frames[frame][track]`; with `wrap` the last frame repeats the first and is not stored.
    pub frames: Vec<Vec<Qvv>>,
    /// The value of a sub-track that has no data.
    pub reference: Vec<Qvv>,
    pub sample_rate: f32,
    /// Bits per component of animated sub-tracks (the `j`-th animated rotation uses `bits + j % 3`).
    pub bits: u8,
    /// The first animated rotation is stored as floats (bit rate byte 31).
    pub raw_first: bool,
    /// Samples per segment, 0 for one segment.
    pub segment_len: usize,
    /// Samples (clip numbers) left out of the stream; their segments must hold 32 samples or fewer.
    pub strip: Vec<usize>,
    pub wrap: bool,
    /// Store everything as floats (rotations as three floats, `w` dropped) instead of quantised.
    pub full_precision: bool,
}

fn put_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn put_f32(out: &mut Vec<u8>, v: f32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn pad(out: &mut Vec<u8>, to: usize) {
    while !out.len().is_multiple_of(to) {
        out.push(0);
    }
}

/// A most-significant-bit-first writer.
#[derive(Default)]
struct Bits {
    bytes: Vec<u8>,
    used: usize,
}

impl Bits {
    fn push(&mut self, value: u32, count: u32) {
        for i in (0..count).rev() {
            if self.used.is_multiple_of(8) {
                self.bytes.push(0);
            }
            let last = self.bytes.len() - 1;
            self.bytes[last] |= (((value >> i) & 1) as u8) << (7 - self.used % 8);
            self.used += 1;
        }
    }
}

fn positive_w(q: [f32; 4]) -> [f32; 4] {
    if q[3] < 0.0 { q.map(|v| -v) } else { q }
}

fn same(a: &[f32], b: &[f32]) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-7)
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Default,
    Constant,
    Animated,
}

fn classify(values: &[Vec<f32>], reference: &[f32]) -> Kind {
    if values.iter().all(|v| same(v, reference)) {
        return Kind::Default;
    }
    if values.iter().all(|v| same(v, &values[0])) {
        return Kind::Constant;
    }
    Kind::Animated
}

fn type_words(kinds: &[Kind]) -> Vec<u32> {
    let mut words = vec![0u32; kinds.len().div_ceil(16)];
    for (t, kind) in kinds.iter().enumerate() {
        let code = match kind {
            Kind::Default => 0,
            Kind::Constant => 1,
            Kind::Animated => 2,
        };
        words[t / 16] |= code << ((15 - t % 16) * 2);
    }
    words
}

/// The minimum and the extent of each of three components over some values.
fn range_of(values: &[[f32; 3]]) -> ([f32; 3], [f32; 3]) {
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    for v in values {
        for k in 0..3 {
            min[k] = min[k].min(v[k]);
            max[k] = max[k].max(v[k]);
        }
    }
    (min, [max[0] - min[0], max[1] - min[1], max[2] - min[2]])
}

fn unit(v: f32, min: f32, extent: f32) -> f32 {
    if extent <= 0.0 { 0.0 } else { ((v - min) / extent).clamp(0.0, 1.0) }
}

/// One animated sub-track: a value per stored sample and the clip range.
struct Animated {
    values: Vec<[f32; 3]>,
    min: [f32; 3],
    extent: [f32; 3],
    rotation: bool,
    bits: u8,
}

fn kinds_of(
    spec: &Spec,
    stored: usize,
    pick: impl Fn(&Qvv) -> Vec<f32>,
    reference: &dyn Fn(&Qvv) -> Vec<f32>,
) -> Vec<Kind> {
    (0..spec.reference.len())
        .map(|t| {
            let values: Vec<Vec<f32>> = spec.frames[..stored].iter().map(|f| pick(&f[t])).collect();
            classify(&values, &reference(&spec.reference[t]))
        })
        .collect()
}

fn animated_tracks(spec: &Spec, stored: usize, kinds: &[Kind], which: usize) -> Vec<Animated> {
    let mut out = Vec::new();
    for t in (0..kinds.len()).filter(|t| kinds[*t] == Kind::Animated) {
        let values: Vec<[f32; 3]> = spec.frames[..stored]
            .iter()
            .map(|f| match which {
                0 => {
                    let q = positive_w(f[t].rotation);
                    [q[0], q[1], q[2]]
                }
                1 => f[t].translation,
                _ => f[t].scale,
            })
            .collect();
        let (min, extent) = range_of(&values);
        out.push(Animated { values, min, extent, rotation: which == 0, bits: 0 });
    }
    out
}

/// One segment's bytes and header numbers.
struct SegmentData {
    formats: Vec<u8>,
    ranges: Vec<u8>,
    poses: Vec<u8>,
    rotation_bits: u32,
    translation_bits: u32,
    pose_bits: u32,
    sample_indices: u32,
}

#[allow(clippy::too_many_lines)]
pub fn encode(spec: &Spec) -> Vec<u8> {
    let tracks = spec.reference.len();
    let stored = if spec.wrap { spec.frames.len() - 1 } else { spec.frames.len() };
    let rk = kinds_of(spec, stored, |q| positive_w(q.rotation).to_vec(), &|q| positive_w(q.rotation).to_vec());
    let tk = kinds_of(spec, stored, |q| q.translation.to_vec(), &|q| q.translation.to_vec());
    let sk = kinds_of(spec, stored, |q| q.scale.to_vec(), &|q| q.scale.to_vec());
    let has_scale = sk.iter().any(|k| *k != Kind::Default);
    let sk = if has_scale { sk } else { vec![Kind::Default; tracks] };
    let variable = !spec.full_precision;
    let mut subs: Vec<Animated> = animated_tracks(spec, stored, &rk, 0);
    let n_rot = subs.len();
    subs.extend(animated_tracks(spec, stored, &tk, 1));
    let n_tra = subs.len() - n_rot;
    subs.extend(if has_scale { animated_tracks(spec, stored, &sk, 2) } else { Vec::new() });
    let n_sca = subs.len() - n_rot - n_tra;
    for (j, a) in subs.iter_mut().enumerate() {
        a.bits = match (a.rotation, j, spec.raw_first) {
            (true, 0, true) => 31,
            (true, _, _) => spec.bits + (j % 3) as u8,
            _ => spec.bits,
        };
    }
    let rot_padded = n_rot.div_ceil(4) * 4;

    // Blocks shared by all segments.
    let mut types = Vec::new();
    for w in
        type_words(&rk).into_iter().chain(type_words(&tk)).chain(if has_scale { type_words(&sk) } else { Vec::new() })
    {
        put_u32(&mut types, w);
    }
    let mut constants = Vec::new();
    let const_rot: Vec<[f32; 4]> =
        (0..tracks).filter(|t| rk[*t] == Kind::Constant).map(|t| positive_w(spec.frames[0][t].rotation)).collect();
    for group in const_rot.chunks(4) {
        for k in 0..3 {
            group.iter().for_each(|q| put_f32(&mut constants, q[k]));
        }
    }
    (0..tracks)
        .filter(|t| tk[*t] == Kind::Constant)
        .for_each(|t| spec.frames[0][t].translation.iter().for_each(|v| put_f32(&mut constants, *v)));
    (0..tracks)
        .filter(|t| sk[*t] == Kind::Constant)
        .for_each(|t| spec.frames[0][t].scale.iter().for_each(|v| put_f32(&mut constants, *v)));
    let mut clip = Vec::new();
    if variable {
        for group in subs[..n_rot].chunks(4) {
            for k in 0..3 {
                group.iter().for_each(|a| put_f32(&mut clip, a.min[k]));
            }
            for k in 0..3 {
                group.iter().for_each(|a| put_f32(&mut clip, a.extent[k]));
            }
        }
        for a in &subs[n_rot..] {
            a.min.iter().chain(&a.extent).for_each(|v| put_f32(&mut clip, *v));
        }
    }

    // Segments.
    let seg_len = if spec.segment_len == 0 { stored.max(1) } else { spec.segment_len };
    let starts: Vec<usize> = (0..stored).step_by(seg_len).collect();
    let multiple = starts.len() > 1;
    let stripped = !spec.strip.is_empty();
    let mut segments = Vec::new();
    for (s, &first) in starts.iter().enumerate() {
        let last = starts.get(s + 1).copied().unwrap_or(stored);
        let kept: Vec<usize> = (first..last).filter(|i| !spec.strip.contains(i)).collect();
        let mut seg = SegmentData {
            formats: Vec::new(),
            ranges: Vec::new(),
            poses: Vec::new(),
            rotation_bits: 0,
            translation_bits: 0,
            pose_bits: 0,
            sample_indices: u32::MAX,
        };
        // Quantisation range of the segment per sub-track, one byte per component.
        let mut seg_range = Vec::new();
        for a in &subs {
            let norm: Vec<[f32; 3]> =
                a.values[first..last].iter().map(|v| [0, 1, 2].map(|k| unit(v[k], a.min[k], a.extent[k]))).collect();
            let (mut lo8, mut ex8) = ([0u8; 3], [255u8; 3]);
            if multiple && variable && a.bits != 31 {
                let (lo, ex) = range_of(&norm);
                for k in 0..3 {
                    let low = (lo[k] * 255.0).floor().clamp(0.0, 255.0);
                    let high = ((lo[k] + ex[k]) * 255.0).ceil().clamp(0.0, 255.0);
                    lo8[k] = low as u8;
                    ex8[k] = (high - low).max(1.0) as u8;
                }
            }
            seg_range.push((lo8, ex8, norm));
        }
        let mut stream = Bits::default();
        for &i in &kept {
            for (a, (lo8, ex8, norm)) in subs.iter().zip(&seg_range) {
                for k in 0..3 {
                    if !variable || a.bits == 31 {
                        stream.push(a.values[i][k].to_bits(), 32);
                        continue;
                    }
                    let local = if multiple {
                        let (lo, ex) = (f32::from(lo8[k]) / 255.0, f32::from(ex8[k]) / 255.0);
                        ((norm[i - first][k] - lo) / ex).clamp(0.0, 1.0)
                    } else {
                        norm[i - first][k]
                    };
                    stream.push((local * ((1u64 << a.bits) - 1) as f32).round() as u32, u32::from(a.bits));
                }
            }
        }
        let width = |a: &Animated| 3 * if !variable || a.bits == 31 { 32 } else { u32::from(a.bits) };
        seg.rotation_bits = subs[..n_rot].iter().map(width).sum();
        seg.translation_bits = subs[n_rot..n_rot + n_tra].iter().map(width).sum();
        seg.pose_bits = subs.iter().map(width).sum();
        seg.poses = stream.bytes;
        if variable {
            subs[..n_rot].iter().for_each(|a| seg.formats.push(a.bits));
            seg.formats.resize(rot_padded, 0);
            subs[n_rot..].iter().for_each(|a| seg.formats.push(a.bits));
            if multiple {
                for group in seg_range[..n_rot].chunks(4) {
                    for k in 0..6 {
                        for i in 0..4 {
                            seg.ranges
                                .push(group.get(i).map_or(0, |(lo, ex, _)| if k < 3 { lo[k] } else { ex[k - 3] }));
                        }
                    }
                }
                for (lo, ex, _) in &seg_range[n_rot..] {
                    seg.ranges.extend_from_slice(lo);
                    seg.ranges.extend_from_slice(ex);
                }
            }
        }
        if stripped {
            seg.sample_indices =
                (0..last - first).filter(|i| !spec.strip.contains(&(first + i))).fold(0u32, |w, i| w | 1 << (31 - i));
        }
        segments.push(seg);
    }

    // Assemble: [th][start indices][segment headers][types][constants][clip range][segment data...], offsets from byte 32.
    let stride = if stripped { 20 } else { 16 };
    let headers = 52 + if multiple { 4 * starts.len() } else { 0 };
    let types_at = headers + stride * segments.len();
    let constants_at = types_at + types.len();
    let clip_at = constants_at + constants.len();
    let mut data = Vec::new();
    let mut data_at = Vec::new();
    let mut base = clip_at + clip.len();
    for seg in &segments {
        let mut blob = seg.formats.clone();
        debug_assert!(base.is_multiple_of(4));
        pad(&mut blob, 2);
        blob.extend_from_slice(&seg.ranges);
        pad(&mut blob, 4);
        blob.extend_from_slice(&seg.poses);
        pad(&mut blob, 4);
        data_at.push(base);
        base += blob.len();
        data.extend(blob);
    }
    let animated_variable = if variable { rot_padded + n_tra + n_sca } else { 0 };
    let mut th = Vec::new();
    for v in [
        segments.len(),
        animated_variable,
        n_rot,
        n_tra,
        n_sca,
        const_rot.len(),
        tk.iter().filter(|k| **k == Kind::Constant).count(),
        sk.iter().filter(|k| **k == Kind::Constant).count(),
    ] {
        put_u32(&mut th, v as u32);
    }
    for v in [u32::MAX, headers as u32, types_at as u32, constants_at as u32, clip_at as u32] {
        put_u32(&mut th, v);
    }
    for &start in starts.iter().filter(|_| multiple) {
        put_u32(&mut th, start as u32);
    }
    for (seg, at) in segments.iter().zip(&data_at) {
        for v in [seg.pose_bits, seg.rotation_bits, seg.translation_bits, *at as u32] {
            put_u32(&mut th, v);
        }
        if stripped {
            put_u32(&mut th, seg.sample_indices);
        }
    }
    th.extend(types);
    th.extend(constants);
    th.extend(clip);
    th.extend(data);
    th.extend([0u8; 15]);

    let misc = u32::from(has_scale)
        | 1 << 1
        | u32::from(variable) << 2
        | u32::from(variable) << 3
        | if variable { 3 } else { 2 } << 4
        | u32::from(spec.reference.iter().all(|r| *r == Qvv::IDENTITY)) << 9
        | u32::from(stripped) << 10
        | u32::from(spec.wrap) << 30;
    let mut out = Vec::new();
    put_u32(&mut out, 0);
    put_u32(&mut out, 0);
    put_u32(&mut out, 0xac11_ac11);
    out.extend_from_slice(&10u16.to_le_bytes());
    out.extend_from_slice(&[0, 12]);
    put_u32(&mut out, tracks as u32);
    put_u32(&mut out, stored as u32);
    put_f32(&mut out, spec.sample_rate);
    put_u32(&mut out, misc);
    out.extend(th);
    let size = out.len() as u32;
    out[0..4].copy_from_slice(&size.to_le_bytes());
    out
}
