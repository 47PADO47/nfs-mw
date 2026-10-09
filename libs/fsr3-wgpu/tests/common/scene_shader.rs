//! The WGSL of the test scene: an analytic world evaluated at jittered render pixels (colour, depth and
//! motion vectors) or supersampled at the output resolution (the reference).

#![allow(dead_code)]

pub const SHADER: &str = "
struct Scene {
    render_size: vec2<f32>,
    display_size: vec2<f32>,
    jitter: vec2<f32>,
    velocity: vec2<f32>,
    time: f32,
    kind: f32,
    near: f32,
    far: f32,
    depth_mode: f32,
    samples: f32,
    gain: f32,
    motion_gain: f32,
    mv_offset: vec2<f32>,
    pad: vec2<f32>,
}
@group(0) @binding(0) var<uniform> scene: Scene;

struct Hit {
    color: vec3<f32>,
    z: f32,
    velocity: vec2<f32>,
}

fn checker(p: vec2<f32>, cell: f32) -> f32 {
    let c = vec2<i32>(floor(p / cell));
    return select(0.08, 0.92, ((c.x + c.y) & 1) == 0);
}

fn static_scene(d: vec2<f32>) -> vec3<f32> {
    let size = scene.display_size;
    let uv = d / size;
    var c = vec3<f32>(0.25 + 0.4 * uv.x, 0.3 + 0.3 * uv.y, 0.45 - 0.2 * uv.x);
    if length(d - size * vec2<f32>(0.35, 0.45)) < size.y * 0.25 {
        c = vec3<f32>(0.95, 0.75, 0.2);
    }
    if fract((d.x + d.y) / 29.0) < 1.5 / 29.0 {
        c = vec3<f32>(0.05);
    }
    if d.x > size.x * 0.6 && d.x < size.x * 0.9 && d.y > size.y * 0.55 && d.y < size.y * 0.85 {
        c = vec3<f32>(checker(d, 4.0));
    }
    return c;
}

fn eval(d: vec2<f32>) -> Hit {
    let kind = u32(scene.kind);
    if kind == 0u {
        return Hit(static_scene(d) * scene.gain, 20.0, vec2<f32>(0.0));
    }
    if kind == 4u {
        // Band-limited content: soft edges, a grating and a soft line, nothing sharper than ~3 px.
        let size = scene.display_size;
        var c = vec3<f32>(0.25 + 0.4 * d.x / size.x, 0.3 + 0.3 * d.y / size.y, 0.4);
        let radius = size.y * 0.25;
        let r = length(d - size * vec2<f32>(0.35, 0.45));
        c = mix(vec3<f32>(0.95, 0.75, 0.2), c, smoothstep(radius - 1.5, radius + 1.5, r));
        if d.x > size.x * 0.6 && d.x < size.x * 0.9 && d.y > size.y * 0.55 && d.y < size.y * 0.85 {
            let wave = sin(dot(d, vec2<f32>(0.93, 0.37)) * 6.2831853 / 5.0);
            c = vec3<f32>(0.5 + 0.4 * wave);
        }
        let across = abs(fract((d.x + d.y) / 41.0 + 0.5) - 0.5) * 41.0 / 1.4142;
        c *= 1.0 - 0.8 * exp(-across * across / (2.0 * 1.2 * 1.2));
        return Hit(c * scene.gain, 20.0, vec2<f32>(0.0));
    }
    if kind == 3u {
        return Hit((vec3<f32>(1.0) - static_scene(d)) * scene.gain, 20.0, vec2<f32>(0.0));
    }
    if kind == 1u {
        let v = checker(d - scene.velocity * scene.time, 8.0);
        return Hit(vec3<f32>(v, v * 0.9, v * 0.8) * scene.gain, 20.0, scene.velocity);
    }
    // Two layers: a static checker background far away and a moving striped square close to the camera.
    let origin = scene.display_size * 0.3 + scene.velocity * scene.time;
    let extent = vec2<f32>(scene.display_size.y * 0.35);
    if all(d >= origin) && all(d < origin + extent) {
        let stripes = checker(d - scene.velocity * scene.time, 6.0);
        return Hit(vec3<f32>(0.9, 0.2, 0.1) * (0.6 + 0.4 * stripes) * scene.gain, 5.0, scene.velocity);
    }
    return Hit(vec3<f32>(checker(d, 16.0) * 0.7 + 0.1) * scene.gain, 50.0, vec2<f32>(0.0));
}

fn encode_depth(z: f32) -> f32 {
    let n = scene.near;
    let f = scene.far;
    switch u32(scene.depth_mode) {
        case 0u: { return f * (z - n) / (z * (f - n)); }
        case 1u: { return n / z; }
        case 2u: { return n * (f - z) / (z * (f - n)); }
        default: { return 1.0 - n / z; }
    }
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let p = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
}

struct Out {
    @location(0) color: vec4<f32>,
    @location(1) motion: vec4<f32>,
    @builtin(frag_depth) depth: f32,
}

@fragment
fn fs_scene(@builtin(position) pos: vec4<f32>) -> Out {
    // The jitter shifts the image by `jitter` pixels, so a pixel sees the scene that far back.
    let unjittered = pos.xy - scene.jitter;
    let ratio = scene.display_size / scene.render_size;
    let hit = eval(unjittered * ratio);
    let motion = -hit.velocity / ratio * scene.motion_gain + scene.mv_offset;
    return Out(vec4<f32>(hit.color, 1.0), vec4<f32>(motion, 0.0, 0.0), encode_depth(hit.z));
}

// The motion vectors at the output resolution, in output pixels.
@fragment
fn fs_motion(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let hit = eval(pos.xy);
    let ratio = scene.display_size / scene.render_size;
    return vec4<f32>(-hit.velocity * scene.motion_gain + scene.mv_offset * ratio, 0.0, 0.0);
}

@fragment
fn fs_reference(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let n = i32(scene.samples);
    var sum = vec3<f32>(0.0);
    for (var j = 0; j < n; j++) {
        for (var i = 0; i < n; i++) {
            let d = floor(pos.xy) + (vec2<f32>(f32(i), f32(j)) + 0.5) / f32(n);
            sum += eval(d).color;
        }
    }
    return vec4<f32>(sum / f32(n * n), 1.0);
}
";
