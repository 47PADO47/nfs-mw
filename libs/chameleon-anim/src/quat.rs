//! Quaternions (x, y, z, w) and the 32-bit packing the clips store them in.

/// The scale of a packed field: `(field - 511) * PACK_SCALE`.
const PACK_SCALE: f32 = f32::from_bits(0x3ab5_3240);

pub type Quat = [f32; 4];

pub const IDENTITY: Quat = [0.0, 0.0, 0.0, 1.0];

/// A packed rotation: the top two bits say which of the four components is left out; the three 10-bit fields below hold
/// the others, in order, at bits 20, 10 and 0; the left-out one is what makes the quaternion unit (and positive).
pub fn unpack(packed: u32) -> Quat {
    let left_out = (packed >> 30) as usize;
    let field = |shift: u32| (((packed >> shift) & 0x3ff) as f32 - 511.0) * PACK_SCALE;
    let stored = [field(20), field(10), field(0)];
    let w = (1.0 - stored.iter().map(|c| c * c).sum::<f32>()).max(0.0).sqrt();
    let mut out = [0.0; 4];
    let mut next = stored.into_iter();
    for (lane, slot) in out.iter_mut().enumerate() {
        *slot = if lane == left_out { w } else { next.next().unwrap_or(0.0) };
    }
    out
}

/// `a * b`: the rotation that does `b` first, then `a`.
pub fn mul(a: Quat, b: Quat) -> Quat {
    let [ax, ay, az, aw] = a;
    let [bx, by, bz, bw] = b;
    [
        aw * bx + ax * bw + ay * bz - az * by,
        aw * by - ax * bz + ay * bw + az * bx,
        aw * bz + ax * by - ay * bx + az * bw,
        aw * bw - ax * bx - ay * by - az * bz,
    ]
}

pub fn conjugate(q: Quat) -> Quat {
    [-q[0], -q[1], -q[2], q[3]]
}

pub fn normalize(q: Quat) -> Quat {
    let length = q.iter().map(|c| c * c).sum::<f32>().sqrt();
    if length < 1e-9 {
        return IDENTITY;
    }
    q.map(|c| c / length)
}

/// Rotate `v` by the unit quaternion `q`.
pub fn rotate(q: Quat, v: [f32; 3]) -> [f32; 3] {
    let [x, y, z, w] = q;
    // t = 2 * cross(q.xyz, v); v' = v + w * t + cross(q.xyz, t)
    let t = [2.0 * (y * v[2] - z * v[1]), 2.0 * (z * v[0] - x * v[2]), 2.0 * (x * v[1] - y * v[0])];
    [
        v[0] + w * t[0] + (y * t[2] - z * t[1]),
        v[1] + w * t[1] + (z * t[0] - x * t[2]),
        v[2] + w * t[2] + (x * t[1] - y * t[0]),
    ]
}

/// Spherical interpolation from `a` to `b` by `t`, taking the short way round.
pub fn slerp(a: Quat, b: Quat, t: f32) -> Quat {
    let mut dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let mut b = b;
    if dot < 0.0 {
        b = b.map(|c| -c);
        dot = -dot;
    }
    if dot > 0.9995 {
        return normalize(std::array::from_fn(|n| a[n] + (b[n] - a[n]) * t));
    }
    let theta = dot.clamp(-1.0, 1.0).acos();
    let sin = theta.sin();
    let (wa, wb) = (((1.0 - t) * theta).sin() / sin, (t * theta).sin() / sin);
    std::array::from_fn(|n| a[n] * wa + b[n] * wb)
}

/// The angle of the rotation in degrees.
pub fn angle_degrees(q: Quat) -> f32 {
    2.0 * q[3].abs().min(1.0).acos().to_degrees()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_identity_packs_to_the_middle_of_each_field() {
        // The 911's spoiler: key 0 is the identity.
        let q = unpack(0xDFF7_FDFF);
        assert!(q[..3].iter().all(|c| c.abs() < 2e-3) && (q[3] - 1.0).abs() < 1e-4, "{q:?}");
    }

    #[test]
    fn a_turn_of_the_spoiler_reads_as_thirty_degrees() {
        let q = mul(unpack(0xEBD7_FDFF), conjugate(unpack(0xDFF7_FDFF)));
        assert!((angle_degrees(q) - 30.46).abs() < 0.05, "{}", angle_degrees(q));
    }

    #[test]
    fn rotating_a_vector_about_z_by_a_quarter_turn() {
        let s = std::f32::consts::FRAC_1_SQRT_2;
        let r = rotate([0.0, 0.0, s, s], [1.0, 0.0, 0.0]);
        assert!((r[0]).abs() < 1e-6 && (r[1] - 1.0).abs() < 1e-6 && r[2].abs() < 1e-6, "{r:?}");
    }

    #[test]
    fn slerp_goes_half_way() {
        let s = std::f32::consts::FRAC_1_SQRT_2;
        let q = slerp(IDENTITY, [0.0, 0.0, s, s], 0.5);
        assert!((angle_degrees(q) - 45.0).abs() < 1e-3);
    }

    #[test]
    fn multiplying_by_the_conjugate_gives_the_identity() {
        let q = normalize([0.1, 0.5, -0.3, 0.8]);
        let r = mul(q, conjugate(q));
        assert!(r[..3].iter().all(|c| c.abs() < 1e-6) && (r[3] - 1.0).abs() < 1e-6);
    }
}
