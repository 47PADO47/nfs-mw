/* MIT License

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE. */
// Histogram approach adapted from Bevy 0.20 auto_exposure.wesl (MIT, see NOTICE).
@group(0) @binding(0) var scene: texture_2d<f32>;
@group(0) @binding(1) var<storage, read_write> ev: f32;
struct Tuning { range: vec4<f32>, adaptation: vec4<f32> }
@group(0) @binding(2) var<uniform> tuning: Tuning;
var<workgroup> bins: array<atomic<u32>, 64>;

@compute @workgroup_size(64)
fn meter(@builtin(local_invocation_index) lane: u32) {
    let size = textureDimensions(scene);
    for (var row = 0u; row < 64u; row++) {
        let pixel = (vec2<u32>(lane, row) * 2u + 1u) * size / 128u;
        let rgb = textureLoad(scene, pixel, 0).rgb;
        let log_lum = log2(max(dot(rgb, vec3<f32>(0.2126, 0.7152, 0.0722)), 0.00024414));
        atomicAdd(&bins[u32(clamp((log_lum + 12.0) * 4.0, 0.0, 63.0))], 1u);
    }
    workgroupBarrier();
    if lane != 0u { return; }
    var sum = 0.0;
    var count = 0u;
    // Trim 10% from each tail, splitting the bins at both boundaries.
    for (var i = 0u; i < 64u; i++) {
        let end = count + atomicLoad(&bins[i]);
        let weight = clamp(end, 410u, 3686u) - clamp(count, 410u, 3686u);
        sum += f32(weight) * ((f32(i) + 0.5) * 0.25 - 12.0);
        count = end;
    }
    let mean = sum / 3276.0;
    let r = tuning.range;
    let a = tuning.adaptation;
    let goal = clamp(clamp(mean, r.x, r.y) - mean + a.z, r.z, r.w);
    let rate = select(a.y, a.x, goal > ev);
    let blend = select(1.0 - exp(-rate * a.w), 1.0, a.w < 0.0);
    ev = mix(ev, goal, blend);
}
