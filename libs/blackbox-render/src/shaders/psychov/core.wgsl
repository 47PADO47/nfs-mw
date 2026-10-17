// PsychoV test30, Copyright (c) 2026 Carlos Lopez Jr.
// RenoDX Copyright (c) 2025 Carlos Lopez Jr.
// WGSL port of pinned MIT source; see docs/provenance/psychov.md.
// MIT License
// 
// Copyright (c) 2025 Carlos Lopez Jr.
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

struct type_u002e_Controls {
    a: vec4<f32>,
    b: vec4<f32>,
}

@group(0) @binding(0) 
var source: texture_2d<f32>;
@group(0) @binding(2) var display_sampler: sampler;
@group(0) @binding(3) 
var<uniform> Controls: type_u002e_Controls;
var<private> global: vec4<f32>;
var<private> out_u002e_var_u002e_SV_Target: vec4<f32>;

fn fs_main_1() {
    var phi_234_: bool;
    var phi_267_: vec3<f32>;
    var phi_293_: bool;
    var phi_328_: vec3<f32>;
    var phi_329_: vec3<f32>;
    var phi_340_: vec3<f32>;
    var phi_368_: bool;
    var phi_374_: vec3<f32>;
    var phi_487_: bool;
    var phi_491_: bool;
    var phi_493_: f32;
    var local: f32;
    var local_1: f32;
    var local_2: f32;
    var local_3: f32;
    var phi_521_: bool;
    var phi_525_: bool;
    var phi_527_: f32;
    var local_4: f32;
    var local_5: f32;
    var local_6: f32;
    var local_7: f32;
    var phi_555_: bool;
    var phi_559_: bool;
    var phi_561_: f32;
    var phi_575_: bool;
    var phi_581_: bool;
    var phi_607_: f32;
    var phi_629_: vec3<f32>;
    var phi_646_: vec3<f32>;
    var phi_660_: f32;
    var phi_665_: f32;
    var phi_678_: f32;
    var phi_683_: f32;
    var phi_696_: f32;
    var phi_701_: f32;
    var phi_703_: f32;
    var phi_724_: f32;
    var phi_743_: vec3<f32>;
    var phi_781_: bool;
    var phi_802_: vec2<f32>;
    var phi_803_: vec2<f32>;
    var phi_831_: vec3<f32>;
    var phi_838_: bool;
    var phi_843_: bool;
    var phi_848_: bool;
    var phi_880_: f32;
    var phi_881_: f32;
    var phi_882_: f32;
    var phi_925_: vec2<f32>;
    var phi_965_: vec2<f32>;
    var phi_978_: vec2<f32>;
    var phi_979_: f32;
    var phi_995_: vec2<f32>;
    var phi_1012_: vec3<f32>;
    var local_8: f32;
    var phi_1027_: vec3<f32>;
    var phi_1051_: vec4<f32>;

    let _e138: mat3x3<f32> = (mat3x3<f32>(vec3<f32>(0.6243666f, -0.4508868f, 0.12772614f), vec3<f32>(0.20918773f, 0.14335263f, -0.014079386f), vec3<f32>(-0.020585753f, 0.029931914f, 0.63915277f)) * 2.901281f);
    let _e139: mat3x3<f32> = (mat3x3<f32>(vec3<f32>(0.4123908f, 0.35758433f, 0.1804808f), vec3<f32>(0.212639f, 0.71516865f, 0.07219232f), vec3<f32>(0.019330818f, 0.11919478f, 0.95053214f)) * mat3x3<f32>(vec3<f32>(0.2670503f, 0.847199f, -0.034704167f), vec3<f32>(-0.38706884f, 1.16543f, 0.103022866f), vec3<f32>(0.026727794f, -0.027291317f, 0.53332675f)));
    let _e140: mat3x3<f32> = (_e138 * mat3x3<f32>(vec3<f32>(3.24097f, -1.5373832f, -0.49861076f), vec3<f32>(-0.96924365f, 1.8759675f, 0.04155506f), vec3<f32>(0.05563008f, -0.20397696f, 1.0569715f)));
    let _e141: mat3x3<f32> = (_e138 * mat3x3<f32>(vec3<f32>(1.7166512f, -0.35567078f, -0.2533663f), vec3<f32>(-0.6666843f, 1.6164812f, 0.015768547f), vec3<f32>(0.017639857f, -0.042770613f, 0.94210315f)));
    let _e143: vec3<f32> = max((_e139 * vec3<f32>(0.6899027f, 0.34832188f, 0f)), vec3<f32>(0f, 0f, 0f));
    let _e151: vec3<f32> = (vec3<f32>(1f, 1f, 1f) * _e139);
    let _e152: f32 = dot(vec3<f32>(0.6899027f, 0.34832188f, 0f), _e151);
    let _e155: f32 = ((0.6899027f * _e151.x) / _e152);
    let _e158: f32 = ((0.34832188f * _e151.y) / _e152);
    let _e159: f32 = (_e155 - _e158);
    let _e166: vec3<f32> = vec3<f32>((_e158 * _e151.x), (-(_e155) * _e151.y), ((-0.5f * _e159) * _e151.z));
    let _e168: vec3<f32> = vec3<f32>(0f, 0f, (0.5f * _e151.z));
    let _e169: vec3<f32> = (_e166 * _e140);
    let _e170: vec3<f32> = (_e168 * _e140);
    let _e173: vec4<f32> = global;
    switch bitcast<i32>(0u) {
        default: {
            let _e181: vec4<f32> = textureSampleLevel(source, display_sampler, _e173.xy/Controls.b.yz, 0.0);
            let _e183: vec3<f32> = max(_e181.xyz, vec3<f32>(0f, 0f, 0f));
            let _e192: f32 = Controls.a[0i];
            let _e195: f32 = Controls.a[2i];
            let _e196: bool = (_e195 > 0.5f);
            switch bitcast<i32>(0u) {
                default: {
                    let _e199: vec3<f32> = (select(pow(((_e183 + vec3<f32>(0.055f, 0.055f, 0.055f)) * vec3<f32>(0.94786733f, 0.94786733f, 0.94786733f)), vec3<f32>(2.4f, 2.4f, 2.4f)), (_e183 * vec3<f32>(0.07739938f, 0.07739938f, 0.07739938f)), (_e183 <= vec3<f32>(0.04045f, 0.04045f, 0.04045f))) * 1f);
                    phi_234_ = false;
                    if all((_e199 <= vec3<f32>(0f, 0f, 0f))) {
                        phi_234_ = !(any((abs(_e199) > vec3<f32>(340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f, 340282350000000000000000000000000000000f))));
                    }
                    let _e207: bool = phi_234_;
                    if _e207 {
                        phi_1027_ = vec3<f32>(0f, 0f, 0f);
                        break;
                    }
                    switch bitcast<i32>(0u) {
                        default: {
                            if all((_e199 >= vec3<f32>(0f, 0f, 0f))) {
                                phi_267_ = (_e199 * _e139);
                                break;
                            }
                            if !(all((_e199 >= vec3<f32>(0f, 0f, 0f)))) {
                                let _e212: f32 = dot(max(_e199, vec3<f32>(0f, 0f, 0f)), (_e143 / vec3(((_e143.x + _e143.y) + _e143.z))));
                                let _e213: vec3<f32> = vec3(_e212);
                                let _e214: vec3<f32> = (_e199 - _e213);
                                let _e218: vec3<f32> = select(vec3<f32>(100000000000000000000f, 100000000000000000000f, 100000000000000000000f), (_e213 / -(_e214)), (_e214 < vec3<f32>(-0.000001f, -0.000001f, -0.000001f)));
                                let _e227: vec3<f32> = ((_e213 + (_e214 * min(1f, min(_e218.x, min(_e218.y, _e218.z))))) * _e139);
                                phi_267_ = (_e227 * ((_e212 * _e152) / (_e227 * mat3x3<f32>(vec3<f32>(1.9473547f, -1.4144512f, 0.36476326f), vec3<f32>(0.6899027f, 0.34832188f, 0f), vec3<f32>(0f, 0f, 1.9348534f))).y));
                                break;
                            }
                        }
                    }
                    let _e235: vec3<f32> = phi_267_;
                    let _e236: vec3<f32> = (vec3<f32>(0.18f, 0.18f, 0.18f) * _e139);
                    let _e237: vec3<f32> = (_e235 / _e236);
                    if any((_e237 <= vec3<f32>(0f, 0f, 0f))) {
                        switch bitcast<i32>(0u) {
                            default: {
                                let _e245: f32 = (_e237.x - _e237.y);
                                let _e249: f32 = (((2f * _e237.z) - _e237.x) - _e237.y);
                                let _e254: f32 = (((3f * _e245) * _e245) + (_e249 * _e249));
                                let _e255: bool = (_e254 > 0.000000000006f);
                                phi_293_ = false;
                                if _e255 {
                                    phi_293_ = _e255;
                                }
                                let _e257: bool = phi_293_;
                                phi_329_ = _e235;
                                if _e257 {
                                    let _e258: f32 = inverseSqrt(_e254);
                                    let _e260: vec2<f32> = (vec2<f32>(_e245, _e249) * _e258);
                                    let _e261: vec2<f32> = (_e260 + _e260);
                                    let _e267: f32 = (((3f * _e261.x) * _e261.x) + (_e261.y * _e261.y));
                                    phi_328_ = _e235;
                                    if (_e267 > 0.000000000001f) {
                                        let _e271: vec2<f32> = ((_e261 * inverseSqrt(_e267)) * (_e254 * _e258));
                                        let _e273: f32 = (0.6899027f * _e236.x);
                                        let _e280: f32 = (((_e235 * mat3x3<f32>(vec3<f32>(1.9473547f, -1.4144512f, 0.36476326f), vec3<f32>(0.6899027f, 0.34832188f, 0f), vec3<f32>(0f, 0f, 1.9348534f))).y - (_e273 * _e271.x)) / (_e273 + (0.34832188f * _e236.y)));
                                        let _e281: f32 = (_e280 + _e271.x);
                                        phi_328_ = (vec3<f32>(_e281, _e280, (0.5f * ((_e271.y + _e281) + _e280))) * _e236);
                                    }
                                    let _e289: vec3<f32> = phi_328_;
                                    phi_329_ = _e289;
                                }
                                let _e291: vec3<f32> = phi_329_;
                                let _e294: bool = !(_e196);
                                if _e294 {
                                    phi_340_ = (_e236 * _e140);
                                }
                                if !(_e294) {
                                    phi_340_ = (_e236 * _e141);
                                }
                                let _e298: vec3<f32> = phi_340_;
                                let _e305: f32 = ((_e192 * (_e236 * mat3x3<f32>(vec3<f32>(1.9473547f, -1.4144512f, 0.36476326f), vec3<f32>(0.6899027f, 0.34832188f, 0f), vec3<f32>(0f, 0f, 1.9348534f))).y) / max(_e298.x, max(_e298.y, _e298.z)));
                                let _e307: f32 = ((_e236 * mat3x3<f32>(vec3<f32>(1.9473547f, -1.4144512f, 0.36476326f), vec3<f32>(0.6899027f, 0.34832188f, 0f), vec3<f32>(0f, 0f, 1.9348534f))).y / _e305);
                                let _e308: f32 = (1f - _e307);
                                let _e321: f32 = bitcast<f32>(((bitcast<u32>((_e235 * mat3x3<f32>(vec3<f32>(1.9473547f, -1.4144512f, 0.36476326f), vec3<f32>(0.6899027f, 0.34832188f, 0f), vec3<f32>(0f, 0f, 1.9348534f))).y) & 2147483648u) | (bitcast<u32>((_e305 * (_e307 / (_e307 + (_e308 * pow((abs((_e235 * mat3x3<f32>(vec3<f32>(1.9473547f, -1.4144512f, 0.36476326f), vec3<f32>(0.6899027f, 0.34832188f, 0f), vec3<f32>(0f, 0f, 1.9348534f))).y) / (_e236 * mat3x3<f32>(vec3<f32>(1.9473547f, -1.4144512f, 0.36476326f), vec3<f32>(0.6899027f, 0.34832188f, 0f), vec3<f32>(0f, 0f, 1.9348534f))).y), (-1f / _e308))))))) & 2147483647u)));
                                phi_368_ = false;
                                if ((_e235 * mat3x3<f32>(vec3<f32>(1.9473547f, -1.4144512f, 0.36476326f), vec3<f32>(0.6899027f, 0.34832188f, 0f), vec3<f32>(0f, 0f, 1.9348534f))).y == 0f) {
                                    phi_368_ = (_e321 == 0f);
                                }
                                let _e325: bool = phi_368_;
                                if _e325 {
                                    phi_374_ = vec3<f32>(0f, 0f, 0f);
                                }
                                if !(_e325) {
                                    phi_374_ = (_e291 * (_e321 / (_e235 * mat3x3<f32>(vec3<f32>(1.9473547f, -1.4144512f, 0.36476326f), vec3<f32>(0.6899027f, 0.34832188f, 0f), vec3<f32>(0f, 0f, 1.9348534f))).y));
                                }
                                let _e329: vec3<f32> = phi_374_;
                                if (_e321 <= 0f) {
                                    phi_743_ = vec3<f32>();
                                    break;
                                }
                                if !((_e321 <= 0f)) {
                                    let _e331: vec3<f32> = (_e329 / _e236);
                                    let _e334: f32 = (_e331.x - _e331.y);
                                    let _e338: f32 = (((2f * _e331.z) - _e331.x) - _e331.y);
                                    let _e343: f32 = (((3f * _e334) * _e334) + (_e338 * _e338));
                                    if (_e343 <= 0.000000000006f) {
                                        phi_743_ = (_e236 * (_e321 / (_e236 * mat3x3<f32>(vec3<f32>(1.9473547f, -1.4144512f, 0.36476326f), vec3<f32>(0.6899027f, 0.34832188f, 0f), vec3<f32>(0f, 0f, 1.9348534f))).y));
                                        break;
                                    }
                                    let _e347: f32 = inverseSqrt(_e343);
                                    let _e348: f32 = (_e343 * _e347);
                                    let _e349: vec2<f32> = (vec2<f32>(_e334, _e338) * _e347);
                                    switch bitcast<i32>(0u) {
                                        default: {
                                            let _e358: vec2<f32> = (vec2<f32>(((_e237.x * 0.6899027f) / (_e237 * mat3x3<f32>(vec3<f32>(1.9473547f, -1.4144512f, 0.36476326f), vec3<f32>(0.6899027f, 0.34832188f, 0f), vec3<f32>(0f, 0f, 1.9348534f))).y), ((_e237.z * 0.037159707f) / (_e237 * mat3x3<f32>(vec3<f32>(1.9473547f, -1.4144512f, 0.36476326f), vec3<f32>(0.6899027f, 0.34832188f, 0f), vec3<f32>(0f, 0f, 1.9348534f))).y)) - vec2<f32>(0.6645024f, 0.03579159f));
                                            let _e359: f32 = dot(_e358, _e358);
                                            if (_e359 <= 0.000000000001f) {
                                                phi_607_ = 0f;
                                                break;
                                            }
                                            let _e368: vec3<f32> = (vec3<f32>(_e139[0].x, _e139[1].x, _e139[2].x) / _e236);
                                            let _e377: vec2<f32> = vec2<f32>(((_e368.x * 0.6899027f) / (_e368 * mat3x3<f32>(vec3<f32>(1.9473547f, -1.4144512f, 0.36476326f), vec3<f32>(0.6899027f, 0.34832188f, 0f), vec3<f32>(0f, 0f, 1.9348534f))).y), ((_e368.z * 0.037159707f) / (_e368 * mat3x3<f32>(vec3<f32>(1.9473547f, -1.4144512f, 0.36476326f), vec3<f32>(0.6899027f, 0.34832188f, 0f), vec3<f32>(0f, 0f, 1.9348534f))).y));
                                            let _e385: vec3<f32> = (vec3<f32>(_e139[0].y, _e139[1].y, _e139[2].y) / _e236);
                                            let _e394: vec2<f32> = vec2<f32>(((_e385.x * 0.6899027f) / (_e385 * mat3x3<f32>(vec3<f32>(1.9473547f, -1.4144512f, 0.36476326f), vec3<f32>(0.6899027f, 0.34832188f, 0f), vec3<f32>(0f, 0f, 1.9348534f))).y), ((_e385.z * 0.037159707f) / (_e385 * mat3x3<f32>(vec3<f32>(1.9473547f, -1.4144512f, 0.36476326f), vec3<f32>(0.6899027f, 0.34832188f, 0f), vec3<f32>(0f, 0f, 1.9348534f))).y));
                                            let _e402: vec3<f32> = (vec3<f32>(_e139[0].z, _e139[1].z, _e139[2].z) / _e236);
                                            let _e411: vec2<f32> = vec2<f32>(((_e402.x * 0.6899027f) / (_e402 * mat3x3<f32>(vec3<f32>(1.9473547f, -1.4144512f, 0.36476326f), vec3<f32>(0.6899027f, 0.34832188f, 0f), vec3<f32>(0f, 0f, 1.9348534f))).y), ((_e402.z * 0.037159707f) / (_e402 * mat3x3<f32>(vec3<f32>(1.9473547f, -1.4144512f, 0.36476326f), vec3<f32>(0.6899027f, 0.34832188f, 0f), vec3<f32>(0f, 0f, 1.9348534f))).y));
                                            let _e413: vec2<f32> = (_e358 * inverseSqrt(_e359));
                                            switch bitcast<i32>(0u) {
                                                default: {
                                                    let _e415: vec2<f32> = (_e394 - _e377);
                                                    let _e422: f32 = ((_e413.x * _e415.y) - (_e413.y * _e415.x));
                                                    local = _e413.x;
                                                    local_1 = _e413.y;
                                                    local_2 = _e413.y;
                                                    local_3 = _e413.x;
                                                    local_4 = _e413.x;
                                                    local_5 = _e413.y;
                                                    local_6 = _e413.y;
                                                    local_7 = _e413.x;
                                                    if (abs(_e422) <= 0.000001f) {
                                                        phi_493_ = 100000000000000000000f;
                                                        break;
                                                    }
                                                    let _e425: vec2<f32> = (_e377 - vec2<f32>(0.6645024f, 0.03579159f));
                                                    let _e431: f32 = (((_e425.x * _e415.y) - (_e425.y * _e415.x)) / _e422);
                                                    let _e435: f32 = (((_e425.x * _e413.y) - (_e425.y * _e413.x)) / _e422);
                                                    phi_487_ = false;
                                                    if (_e431 >= 0f) {
                                                        phi_487_ = (_e435 >= 0f);
                                                    }
                                                    let _e439: bool = phi_487_;
                                                    phi_491_ = false;
                                                    if _e439 {
                                                        phi_491_ = (_e435 <= 1f);
                                                    }
                                                    let _e442: bool = phi_491_;
                                                    phi_493_ = select(100000000000000000000f, _e431, _e442);
                                                    break;
                                                }
                                            }
                                            let _e445: f32 = phi_493_;
                                            switch bitcast<i32>(0u) {
                                                default: {
                                                    let _e447: vec2<f32> = (_e411 - _e394);
                                                    let _e450: f32 = local;
                                                    let _e454: f32 = local_1;
                                                    let _e456: f32 = ((_e450 * _e447.y) - (_e454 * _e447.x));
                                                    if (abs(_e456) <= 0.000001f) {
                                                        phi_527_ = 100000000000000000000f;
                                                        break;
                                                    }
                                                    let _e459: vec2<f32> = (_e394 - vec2<f32>(0.6645024f, 0.03579159f));
                                                    let _e465: f32 = (((_e459.x * _e447.y) - (_e459.y * _e447.x)) / _e456);
                                                    let _e467: f32 = local_2;
                                                    let _e470: f32 = local_3;
                                                    let _e473: f32 = (((_e459.x * _e467) - (_e459.y * _e470)) / _e456);
                                                    phi_521_ = false;
                                                    if (_e465 >= 0f) {
                                                        phi_521_ = (_e473 >= 0f);
                                                    }
                                                    let _e477: bool = phi_521_;
                                                    phi_525_ = false;
                                                    if _e477 {
                                                        phi_525_ = (_e473 <= 1f);
                                                    }
                                                    let _e480: bool = phi_525_;
                                                    phi_527_ = select(100000000000000000000f, _e465, _e480);
                                                    break;
                                                }
                                            }
                                            let _e483: f32 = phi_527_;
                                            switch bitcast<i32>(0u) {
                                                default: {
                                                    let _e485: vec2<f32> = (_e377 - _e411);
                                                    let _e488: f32 = local_4;
                                                    let _e492: f32 = local_5;
                                                    let _e494: f32 = ((_e488 * _e485.y) - (_e492 * _e485.x));
                                                    if (abs(_e494) <= 0.000001f) {
                                                        phi_561_ = 100000000000000000000f;
                                                        break;
                                                    }
                                                    let _e497: vec2<f32> = (_e411 - vec2<f32>(0.6645024f, 0.03579159f));
                                                    let _e503: f32 = (((_e497.x * _e485.y) - (_e497.y * _e485.x)) / _e494);
                                                    let _e505: f32 = local_6;
                                                    let _e508: f32 = local_7;
                                                    let _e511: f32 = (((_e497.x * _e505) - (_e497.y * _e508)) / _e494);
                                                    phi_555_ = false;
                                                    if (_e503 >= 0f) {
                                                        phi_555_ = (_e511 >= 0f);
                                                    }
                                                    let _e515: bool = phi_555_;
                                                    phi_559_ = false;
                                                    if _e515 {
                                                        phi_559_ = (_e511 <= 1f);
                                                    }
                                                    let _e518: bool = phi_559_;
                                                    phi_561_ = select(100000000000000000000f, _e503, _e518);
                                                    break;
                                                }
                                            }
                                            let _e521: f32 = phi_561_;
                                            let _e523: f32 = min(_e445, min(_e483, _e521));
                                            if (_e523 >= 100000000000000000000f) {
                                                phi_607_ = 0f;
                                                break;
                                            }
                                            let _e526: vec2<f32> = (vec2<f32>(0.6645024f, 0.03579159f) + (_e413 * _e523));
                                            let _e528: f32 = (1f - _e526.x);
                                            phi_575_ = true;
                                            if (_e526.x > 0.000001f) {
                                                phi_575_ = (_e528 <= 0.000001f);
                                            }
                                            let _e532: bool = phi_575_;
                                            phi_581_ = true;
                                            if !(_e532) {
                                                phi_581_ = (_e526.y <= 0.000001f);
