// Independently implemented periodic gradient and cellular noise.
// Channel structure follows the published Nubis base/detail texture arrangement.
@group(0) @binding(0) var output: texture_storage_3d<rgba8unorm, write>;

fn hash(p: vec3<i32>, period: i32, seed: u32) -> f32 {
    let c = vec3<u32>(((p % vec3(period)) + vec3(period)) % vec3(period));
    var n = (c.x * 1973u) ^ (c.y * 9277u) ^ (c.z * 26699u) ^ seed;
    n = (n ^ (n >> 13u)) * 1274126177u;
    return f32(n ^ (n >> 16u)) / 4294967295.0;
}

fn gradient_noise(p: vec3<f32>, period: i32) -> f32 {
    let cell = vec3<i32>(floor(p));
    let f = fract(p);
    let t = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    var value = 0.0;
    for (var z = 0; z < 2; z++) {
        for (var y = 0; y < 2; y++) {
            for (var x = 0; x < 2; x++) {
                let corner = vec3(x,y,z);
                let c = cell + corner;
                let random = vec3(hash(c,period,911u), hash(c,period,3571u), hash(c,period,6427u)) * 2.0 - 1.0;
                let gradient = random / max(length(random), 0.001);
                let weight = mix(vec3(1.0) - t, t, vec3<f32>(corner));
                value += dot(gradient, f - vec3<f32>(corner)) * weight.x * weight.y * weight.z;
            }
        }
    }
    return value * 1.7;
}

fn cellular(p: vec3<f32>, period: i32) -> f32 {
    let cell = vec3<i32>(floor(p));
    var distance = 3.0;
    for (var z = -1; z <= 1; z++) {
        for (var y = -1; y <= 1; y++) {
            for (var x = -1; x <= 1; x++) {
                let c = cell + vec3(x,y,z);
                let feature = vec3<f32>(c) + vec3(hash(c,period,911u), hash(c,period,3571u), hash(c,period,6427u));
                distance = min(distance, dot(feature-p, feature-p));
            }
        }
    }
    return clamp(sqrt(distance), 0.0, 1.0);
}

@compute @workgroup_size(8,8,1)
fn base_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let p = (vec3<f32>(id) + 0.5) / vec3<f32>(textureDimensions(output));
    var gradient = 0.0;
    var weight = 0.533333;
    var period = 32;
    for (var i = 0; i < 4; i++) {
        gradient += gradient_noise(p * f32(period), period) * weight;
        weight *= 0.5;
        period *= 2;
    }
    let perlin = clamp(gradient * 0.5 + 0.5, 0.0, 1.0);
    let billow = clamp(1.0 - cellular(p * 16.0, 16) / 0.8660254, 0.0, 1.0);
    let shape = mix(billow, 1.0, perlin);
    textureStore(output, id, vec4(shape, cellular(p*32.0,32), cellular(p*64.0,64), cellular(p*128.0,128)));
}

@compute @workgroup_size(8,8,1)
fn detail_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let p = (vec3<f32>(id) + 0.5) / vec3<f32>(textureDimensions(output));
    textureStore(output, id, vec4(1.0-cellular(p*4.0,4), 1.0-cellular(p*8.0,8), 1.0-cellular(p*16.0,16), 1.0));
}

@compute @workgroup_size(8,8,1)
fn weather_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let p = (vec3<f32>(id) + 0.5) / vec3<f32>(textureDimensions(output));
    var gradient = 0.0;
    var weight = 0.5714286;
    var period = 16;
    for (var i = 0; i < 3; i++) {
        gradient += gradient_noise(vec3(p.xy*f32(period), 3.37), period) * weight;
        weight *= 0.5;
        period *= 2;
    }
    let weather = clamp(gradient*0.5+0.5, 0.0, 1.0);
    let cells = 1.0-cellular(vec3(p.xy*32.0, 0.37), 32);
    textureStore(output, id, vec4(weather, cells, 0.0, 1.0));
}
