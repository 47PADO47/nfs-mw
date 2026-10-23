// Cloud density, height profiles and multiple-scattering approximation adapted from
// SkyGL, shaders/clouds_pass.frag and src/Sky/Weather.cpp,
// commit edbbb0148091bd85261dec0a2164bc026ff01a3e.
// Changes: WGSL, Z-up kilometres, periodic weather field, native renderer bindings.
// MIT License
//
// Copyright (c) 2026 Ivan Vasiliev
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

const CLOUD_BOTTOM: f32 = 2.0;
const CLOUD_THICKNESS: f32 = 4.0;

fn remap_cloud(value: f32, lo: f32, hi: f32) -> f32 {
    return clamp((value - lo) / max(hi - lo, 0.0001), 0.0, 1.0);
}

fn cloud_height(h: f32, kind: f32) -> f32 {
    let stratus = smoothstep(0.0, 0.07, h) * (1.0-smoothstep(0.07, 0.2, h));
    let cumulus = smoothstep(0.0, 0.2, h) * (1.0-smoothstep(0.35, 0.65, h));
    let storm = smoothstep(0.0, 0.1, h) * (1.0-smoothstep(0.4, 0.95, h));
    let a = 1.0-kind;
    return a*a*stratus + 2.0*kind*a*cumulus + kind*kind*storm;
}

fn density(p: vec3<f32>, amount: f32, footprint: f32) -> f32 {
    let height = (p.z - CLOUD_BOTTOM) / CLOUD_THICKNESS;
    if height <= 0.0 || height >= 1.0 { return 0.0; }
    // Scroll is seconds * 0.02. Bound height-dependent shear so long sessions
    // and clock seeks do not stretch clouds into arbitrarily thin streaks.
    let wind = material.options.y * mix(0.65, 0.95, amount);
    let shear = vec2(sin(wind*0.18)*0.9, sin(wind*0.13)*0.35) * height;
    let q = p + vec3(wind + shear.x, wind*0.22 + shear.y, 0.0);
    let weather_position = p.xy + vec2(wind, wind*0.22);
    // A horizontal coverage field groups clouds into systems, independently of billow noise.
    let weather = textureSampleLevel(cloud_weather, cloud_sampler, vec3(weather_position/128.0, 0.5), 0.0).rg;
    // SkyGL's partly/mostly-cloudy fields, continuously blended toward overcast.
    let partly = weather.r * smoothstep(0.3, 1.0, weather.g);
    let mostly = weather.r * smoothstep(0.0, 0.9, weather.g);
    let overcast = smoothstep(0.2, 1.0, weather.r);
    let broken = mix(partly, mostly, clamp(amount*2.0, 0.0, 1.0));
    let coverage = mix(broken, overcast, smoothstep(0.5, 1.0, amount)) * smoothstep(0.0, 0.3, amount);
    if coverage <= 0.001 { return 0.0; }
    let base = textureSampleLevel(cloud_noise, cloud_sampler, q/25.0, clamp(log2(max(footprint * 128.0 / 25.0, 1.0)), 0.0, 2.0));
    let low_fbm = dot(base.gba, vec3(0.625, 0.25, 0.125));
    var cloud = remap_cloud(base.r, -(1.0-low_fbm), 1.0);
    cloud = remap_cloud(cloud, 1.0-coverage, 1.0) * coverage;
    cloud *= cloud_height(height, mix(0.6*weather.g, 0.8, smoothstep(0.5, 1.0, amount)));
    if cloud <= 0.0 { return 0.0; }
    let detail_position = q + vec3(wind*0.27, -wind*0.16, wind*0.035);
    let detail = textureSampleLevel(cloud_detail, cloud_sampler, detail_position/1.5, log2(max(footprint * 32.0 / 1.5, 1.0))).rgb;
    let erosion = dot(detail, vec3(0.625, 0.25, 0.125));
    let result = remap_cloud(cloud, erosion*0.1, 1.0);
    return pow(result, mix(1.0, 0.75, amount));
}

fn cloud_phase(g: f32, mu: f32) -> f32 {
    return (1.0-g*g) / (12.5663706 * pow(max(1.0+g*g-2.0*g*mu, 0.001), 1.5));
}

fn cloud_scattering(optical_depth: f32, mu: f32) -> f32 {
    var attenuation = 1.0;
    var contribution = 1.0;
    var asymmetry = 1.0;
    var result = 0.0;
    for (var i = 0; i < 4; i++) {
        let phase = mix(cloud_phase(-0.2*asymmetry,mu), cloud_phase(0.8*asymmetry,mu), 0.3);
        result += contribution * phase * exp(-10.0 * optical_depth * attenuation);
        attenuation *= 0.5;
        contribution *= 0.6;
        asymmetry *= 0.2;
    }
    return result;
}
