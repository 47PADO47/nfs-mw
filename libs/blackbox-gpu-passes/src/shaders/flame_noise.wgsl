// WGSL port of webgl-noise/src/noise3D.glsl, as used by mattatz/THREE.Fire.
// Copyright (C) 2011 Ashima Arts. All rights reserved.
// Copyright (C) 2011-2016 by Stefan Gustavson (Classic noise and others).
// Distributed under the MIT license. See LICENSE-FLAME-NOISE in this crate.
// https://github.com/ashima/webgl-noise
fn mod289_3(x: vec3<f32>) -> vec3<f32> { return x - floor(x / 289.0) * 289.0; }
fn mod289_4(x: vec4<f32>) -> vec4<f32> { return x - floor(x / 289.0) * 289.0; }
fn permute(x: vec4<f32>) -> vec4<f32> { return mod289_4(((x * 34.0) + 1.0) * x); }
fn snoise(v: vec3<f32>) -> f32 {
    let C = vec2<f32>(1.0 / 6.0, 1.0 / 3.0);
    let D = vec4<f32>(0.0, 0.5, 1.0, 2.0);
    var i = floor(v + dot(v, C.yyy));
    let x0 = v - i + dot(i, C.xxx);
    let g = step(x0.yzx, x0.xyz);
    let l = 1.0 - g;
    let i1 = min(g.xyz, l.zxy);
    let i2 = max(g.xyz, l.zxy);
    let x1 = x0 - i1 + C.xxx;
    let x2 = x0 - i2 + C.yyy;
    let x3 = x0 - D.yyy;
    i = mod289_3(i);
    let p = permute(permute(permute(i.z + vec4<f32>(0.0, i1.z, i2.z, 1.0))
        + i.y + vec4<f32>(0.0, i1.y, i2.y, 1.0)) + i.x + vec4<f32>(0.0, i1.x, i2.x, 1.0));
    let ns = (1.0 / 7.0) * D.wyz - D.xzx;
    let j = p - 49.0 * floor(p * ns.z * ns.z);
    let x_ = floor(j * ns.z);
    let y_ = floor(j - 7.0 * x_);
    let x = x_ * ns.x + ns.yyyy;
    let y = y_ * ns.x + ns.yyyy;
    let h = 1.0 - abs(x) - abs(y);
    let b0 = vec4<f32>(x.xy, y.xy);
    let b1 = vec4<f32>(x.zw, y.zw);
    let s0 = floor(b0) * 2.0 + 1.0;
    let s1 = floor(b1) * 2.0 + 1.0;
    let sh = -step(h, vec4<f32>(0.0));
    let a0 = b0.xzyw + s0.xzyw * sh.xxyy;
    let a1 = b1.xzyw + s1.xzyw * sh.zzww;
    var p0 = vec3<f32>(a0.xy, h.x);
    var p1 = vec3<f32>(a0.zw, h.y);
    var p2 = vec3<f32>(a1.xy, h.z);
    var p3 = vec3<f32>(a1.zw, h.w);
    let norm = 1.79284291400159 - 0.85373472095314
        * vec4<f32>(dot(p0, p0), dot(p1, p1), dot(p2, p2), dot(p3, p3));
    p0 *= norm.x; p1 *= norm.y; p2 *= norm.z; p3 *= norm.w;
    var m = max(0.6 - vec4<f32>(dot(x0, x0), dot(x1, x1), dot(x2, x2), dot(x3, x3)), vec4<f32>(0.0));
    m *= m;
    return 42.0 * dot(m * m, vec4<f32>(dot(p0, x0), dot(p1, x1), dot(p2, x2), dot(p3, x3)));
}
