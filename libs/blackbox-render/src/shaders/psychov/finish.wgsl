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

                                            }
                                            let _e537: bool = phi_581_;
                                            if _e537 {
                                                phi_607_ = 0f;
                                                break;
                                            }
                                            let _e543: vec3<f32> = vec3<f32>((0.5048855f * (_e526.x / _e528)), 1f, (9.373644f * (_e526.y / _e528)));
                                            let _e548: vec3<f32> = (_e543 * (_e321 / ((_e236 * _e543) * mat3x3<f32>(vec3<f32>(1.9473547f, -1.4144512f, 0.36476326f), vec3<f32>(0.6899027f, 0.34832188f, 0f), vec3<f32>(0f, 0f, 1.9348534f))).y));
                                            let _e551: f32 = (_e548.x - _e548.y);
                                            let _e555: f32 = (((2f * _e548.z) - _e548.x) - _e548.y);
                                            phi_607_ = sqrt((((3f * _e551) * _e551) + (_e555 * _e555)));
                                            break;
                                        }
                                    }
                                    let _e562: f32 = phi_607_;
                                    let _e563: f32 = max(_e562, _e348);
                                    switch bitcast<i32>(0u) {
                                        default: {
                                            if (_e563 <= 0.000001f) {
                                                phi_703_ = 0f;
                                                break;
                                            }
                                            let _e567: f32 = (0.6899027f * _e236.x);
                                            let _e570: f32 = (_e567 + (0.34832188f * _e236.y));
                                            let _e571: f32 = (_e321 / _e570);
                                            let _e575: vec3<f32> = (vec3<f32>(_e571, _e571, (0.5f * (_e571 + _e571))) * _e236);
                                            if _e294 {
                                                phi_629_ = (_e575 * _e140);
                                            }
                                            if !(_e294) {
                                                phi_629_ = (_e575 * _e141);
                                            }
                                            let _e579: vec3<f32> = phi_629_;
                                            let _e583: f32 = ((_e321 - (_e567 * _e349.x)) / _e570);
                                            let _e584: f32 = (_e583 + _e349.x);
                                            let _e590: vec3<f32> = (vec3<f32>(_e584, _e583, (0.5f * ((_e349.y + _e584) + _e583))) * _e236);
                                            if _e294 {
                                                phi_646_ = (_e590 * _e140);
                                            }
                                            if !(_e294) {
                                                phi_646_ = (_e590 * _e141);
                                            }
                                            let _e594: vec3<f32> = phi_646_;
                                            let _e595: vec3<f32> = (_e594 - _e579);
                                            if (_e595.x > 0.000001f) {
                                                phi_665_ = min(_e563, ((_e192 - _e579.x) / _e595.x));
                                            }
                                            if !((_e595.x > 0.000001f)) {
                                                phi_660_ = _e563;
                                                if (_e595.x < -0.000001f) {
                                                    phi_660_ = min(_e563, (_e579.x / -(_e595.x)));
                                                }
                                                let _e604: f32 = phi_660_;
                                                phi_665_ = _e604;
                                            }
                                            let _e610: f32 = phi_665_;
                                            if (_e595.y > 0.000001f) {
                                                phi_683_ = min(_e610, ((_e192 - _e579.y) / _e595.y));
                                            }
                                            if !((_e595.y > 0.000001f)) {
                                                phi_678_ = _e610;
                                                if (_e595.y < -0.000001f) {
                                                    phi_678_ = min(_e610, (_e579.y / -(_e595.y)));
                                                }
                                                let _e619: f32 = phi_678_;
                                                phi_683_ = _e619;
                                            }
                                            let _e625: f32 = phi_683_;
                                            if (_e595.z > 0.000001f) {
                                                phi_701_ = min(_e625, ((_e192 - _e579.z) / _e595.z));
                                            }
                                            if !((_e595.z > 0.000001f)) {
                                                phi_696_ = _e625;
                                                if (_e595.z < -0.000001f) {
                                                    phi_696_ = min(_e625, (_e579.z / -(_e595.z)));
                                                }
                                                let _e634: f32 = phi_696_;
                                                phi_701_ = _e634;
                                            }
                                            let _e640: f32 = phi_701_;
                                            phi_703_ = max(_e640, 0f);
                                            break;
                                        }
                                    }
                                    let _e643: f32 = phi_703_;
                                    let _e644: f32 = max(_e563, _e643);
                                    switch bitcast<i32>(0u) {
                                        default: {
                                            if (_e643 <= 0.000001f) {
                                                phi_724_ = 0f;
                                                break;
                                            }
                                            if (_e644 <= (_e643 * 1.000001f)) {
                                                phi_724_ = min(_e348, _e643);
                                                break;
                                            }
                                            let _e651: f32 = clamp((_e348 / _e644), 0f, 1f);
                                            let _e653: f32 = clamp((_e643 / _e644), 0f, 1f);
                                            phi_724_ = ((_e643 * _e651) / ((_e651 * (1f - _e653)) + _e653));
                                            break;
                                        }
                                    }
                                    let _e660: f32 = phi_724_;
                                    let _e662: vec2<f32> = (_e349 * min(_e660, _e643));
                                    let _e664: f32 = (0.6899027f * _e236.x);
                                    let _e671: f32 = ((_e321 - (_e664 * _e662.x)) / (_e664 + (0.34832188f * _e236.y)));
                                    let _e672: f32 = (_e671 + _e662.x);
                                    phi_743_ = (vec3<f32>(_e672, _e671, (0.5f * ((_e662.y + _e672) + _e671))) * _e236);
                                    break;
                                }
                            }
                        }
                        let _e680: vec3<f32> = phi_743_;
                        phi_1027_ = (_e680 * _e140);
                        break;
                    }
                    let _e683: vec3<f32> = (_e236 / (_e151 * _e192));
                    let _e684: vec3<f32> = (vec3<f32>(1f, 1f, 1f) - _e683);
                    let _e689: vec3<f32> = (_e683 / (_e683 + (_e684 * pow(_e237, (vec3<f32>(-1f, -1f, -1f) / _e684)))));
                    let _e692: f32 = (_e237.x - _e237.y);
                    let _e696: f32 = (((2f * _e237.z) - _e237.x) - _e237.y);
                    let _e700: f32 = (_e689.x - _e689.y);
                    let _e701: f32 = (_e689.x + _e689.y);
                    let _e704: f32 = ((2f * _e689.z) - _e701);
                    let _e705: vec2<f32> = vec2<f32>(_e700, _e704);
                    let _e709: f32 = (((3f * _e692) * _e692) + (_e696 * _e696));
                    let _e713: f32 = (((3f * _e700) * _e700) + (_e704 * _e704));
                    phi_781_ = false;
                    if (_e709 > 0.000000000006f) {
                        phi_781_ = (_e713 > 0.000000000006f);
                    }
                    let _e717: bool = phi_781_;
                    phi_803_ = _e705;
                    if _e717 {
                        let _e718: f32 = inverseSqrt(_e713);
                        let _e723: vec2<f32> = ((vec2<f32>(_e692, _e696) * inverseSqrt(_e709)) + (_e705 * _e718));
                        let _e729: f32 = (((3f * _e723.x) * _e723.x) + (_e723.y * _e723.y));
                        phi_802_ = _e705;
                        if (_e729 > 0.000000000001f) {
                            phi_802_ = (_e723 * ((_e713 * _e718) * inverseSqrt(_e729)));
                        }
                        let _e735: vec2<f32> = phi_802_;
                        phi_803_ = _e735;
                    }
                    let _e737: vec2<f32> = phi_803_;
                    let _e740: f32 = ((_e155 * _e689.x) + (_e158 * _e689.y));
                    let _e742: f32 = (_e701 + _e689.z);
                    switch bitcast<i32>(0u) {
                        default: {
                            let _e746: f32 = (3f * _e159);
                            let _e748: f32 = ((_e746 * _e737.x) - _e737.y);
                            let _e749: f32 = (2f * _e742);
                            let _e751: f32 = ((_e749 + _e748) * 0.16666667f);
                            local_8 = _e746;
                            if !(_e196) {
                                phi_831_ = ((_e169 * _e737.x) + (_e170 * _e737.y));
                            }
                            if !(!(_e196)) {
                                phi_831_ = (((_e166 * _e141) * _e737.x) + ((_e168 * _e141) * _e737.y));
                            }
                            let _e760: vec3<f32> = phi_831_;
                            let _e762: vec3<f32> = (vec3(_e751) + _e760);
                            phi_838_ = false;
                            if (_e751 >= 0f) {
                                phi_838_ = (_e751 <= _e740);
                            }
                            let _e766: bool = phi_838_;
                            phi_843_ = false;
                            if _e766 {
                                phi_843_ = all((_e762 >= vec3<f32>(-0.000001f, -0.000001f, -0.000001f)));
                            }
                            let _e770: bool = phi_843_;
                            phi_848_ = false;
                            if _e770 {
                                phi_848_ = all((_e762 <= vec3<f32>(1.000001f, 1.000001f, 1.000001f)));
                            }
                            let _e774: bool = phi_848_;
                            if _e774 {
                                phi_1012_ = vec3<f32>(_e737.x, _e742, _e737.y);
                                break;
                            }
                            let _e778: f32 = (((3f * _e737.x) * _e737.x) + (_e737.y * _e737.y));
                            if (_e778 <= 0.000000000006f) {
                                phi_1012_ = vec3<f32>(0f, clamp(_e742, 0f, (3f * _e740)), 0f);
                                break;
                            }
                            let _e787: f32 = max(_e760.x, max(_e760.y, _e760.z));
                            let _e789: f32 = min(_e760.x, min(_e760.y, _e760.z));
                            let _e790: f32 = -(_e789);
                            let _e791: f32 = (_e787 - _e789);
                            let _e793: bool = ((_e740 * _e791) <= _e790);
                            if _e793 {
                                let _e798: f32 = (_e740 / _e790);
                                phi_880_ = _e798;
                                phi_881_ = _e740;
                                phi_882_ = _e798;
                            }
                            if !(_e793) {
                                let _e796: f32 = (1f / _e791);
                                phi_880_ = _e796;
                                phi_881_ = (_e790 * _e796);
                                phi_882_ = ((1f - _e740) / _e787);
                            }
                            let _e800: f32 = phi_880_;
                            let _e802: f32 = phi_881_;
                            let _e804: f32 = phi_882_;
                            let _e805: f32 = (3f * _e740);
                            let _e806: vec2<f32> = vec2<f32>(_e805, 0f);
                            let _e807: f32 = (0.5f * _e748);
                            let _e809: f32 = (_e805 - (_e807 * _e804));
                            let _e810: vec2<f32> = vec2<f32>(_e809, _e804);
                            let _e813: f32 = ((3f * _e802) - (_e807 * _e800));
                            let _e814: vec2<f32> = vec2<f32>(_e813, _e800);
                            let _e816: vec2<f32> = vec2<f32>(clamp(_e742, 0f, _e805), 0f);
                            let _e817: vec2<f32> = vec2<f32>(_e742, 1f);
                            let _e818: vec2<f32> = (_e816 - _e817);
                            let _e825: f32 = (((2f * _e818.x) * _e818.x) + ((_e778 * _e818.y) * _e818.y));
                            switch bitcast<i32>(0u) {
                                default: {
                                    let _e827: vec2<f32> = (_e810 - _e806);
                                    let _e832: f32 = (_e778 * _e827.y);
                                    let _e834: f32 = (((2f * _e827.x) * _e827.x) + (_e832 * _e827.y));
                                    if (_e834 <= 0.000000000001f) {
                                        phi_925_ = _e806;
                                        break;
                                    }
                                    phi_925_ = (_e806 + (_e827 * clamp(((((2f * (_e742 - _e805)) * _e827.x) + _e832) / _e834), 0f, 1f)));
                                    break;
                                }
                            }
                            let _e845: vec2<f32> = phi_925_;
                            let _e846: vec2<f32> = (_e845 - _e817);
                            let _e853: f32 = (((2f * _e846.x) * _e846.x) + ((_e778 * _e846.y) * _e846.y));
                            let _e854: bool = (_e853 < _e825);
                            let _e856: vec2<f32> = select(_e816, _e845, vec2(_e854));
                            let _e857: f32 = select(_e825, _e853, _e854);
                            phi_978_ = _e856;
                            phi_979_ = _e857;
                            if !(_e793) {
                                switch bitcast<i32>(0u) {
                                    default: {
                                        let _e860: vec2<f32> = (_e814 - _e810);
                                        let _e867: f32 = (((2f * _e860.x) * _e860.x) + ((_e778 * _e860.y) * _e860.y));
                                        if (_e867 <= 0.000000000001f) {
                                            phi_965_ = _e810;
                                            break;
                                        }
                                        phi_965_ = (_e810 + (_e860 * clamp(((((2f * (_e742 - _e809)) * _e860.x) + ((_e778 * (1f - _e804)) * _e860.y)) / _e867), 0f, 1f)));
                                        break;
                                    }
                                }
                                let _e881: vec2<f32> = phi_965_;
                                let _e882: vec2<f32> = (_e881 - _e817);
                                let _e889: f32 = (((2f * _e882.x) * _e882.x) + ((_e778 * _e882.y) * _e882.y));
                                let _e890: bool = (_e889 < _e857);
                                phi_978_ = select(_e856, _e881, vec2(_e890));
                                phi_979_ = select(_e857, _e889, _e890);
                            }
                            let _e895: vec2<f32> = phi_978_;
                            let _e897: f32 = phi_979_;
                            switch bitcast<i32>(0u) {
                                default: {
                                    let _e901: f32 = (_e778 * _e800);
                                    let _e903: f32 = (((2f * _e813) * _e813) + (_e901 * _e800));
                                    if (_e903 <= 0.000000000001f) {
                                        phi_995_ = vec2<f32>(0f, 0f);
                                        break;
                                    }
                                    phi_995_ = (_e814 * clamp((((_e749 * _e813) + _e901) / _e903), 0f, 1f));
                                    break;
                                }
                            }
                            let _e911: vec2<f32> = phi_995_;
                            let _e912: vec2<f32> = (_e911 - _e817);
                            let _e922: vec2<f32> = select(_e895, _e911, vec2(((((2f * _e912.x) * _e912.x) + ((_e778 * _e912.y) * _e912.y)) < _e897)));
                            phi_1012_ = vec3<f32>((_e737.x * _e922.y), _e922.x, (_e737.y * _e922.y));
                            break;
                        }
                    }
                    let _e929: vec3<f32> = phi_1012_;
                    let _e934: f32 = local_8;
                    phi_1027_ = (((vec3(((((2f * _e929.y) + (_e934 * _e929.x)) - _e929.z) * 0.16666667f)) + (_e169 * _e929.x)) + (_e170 * _e929.z)) * _e192);
                    break;
                }
            }
            let _e947: vec3<f32> = phi_1027_;
            if _e196 {
                let _e950: f32 = Controls.a[1i];
                let _e952: vec3<f32> = ((_e947 * _e950) * vec3<f32>(0.0125f, 0.0125f, 0.0125f));
                phi_1051_ = vec4<f32>(_e952.x, _e952.y, _e952.z, _e181.w);
                break;
            }
            let _e958: vec3<f32> = max(_e947, vec3<f32>(0f, 0f, 0f));
            let _e964: vec3<f32> = select(((pow(_e958, vec3<f32>(0.41666666f, 0.41666666f, 0.41666666f)) * 1.055f) - vec3<f32>(0.055f, 0.055f, 0.055f)), (_e958 * 12.92f), (_e958 <= vec3<f32>(0.0031308f, 0.0031308f, 0.0031308f)));
            phi_1051_ = vec4<f32>(_e964.x, _e964.y, _e964.z, _e181.w);
            break;
        }
    }
    let _e971: vec4<f32> = phi_1051_;
    out_u002e_var_u002e_SV_Target = _e971;
    return;
}

@fragment 
fn fs_main(@builtin(position) param: vec4<f32>) -> @location(0) vec4<f32> {
    if Controls.b.x > 0.5 {
        let c = textureSampleLevel(source,display_sampler,param.xy/Controls.b.yz,0.0);
        if Controls.a.z <= 0.5 { return c; }
        let rgb = max(c.rgb,vec3(0.0));
        let linear = select(pow((rgb+vec3(0.055))/1.055,vec3(2.4)),rgb/12.92,rgb<=vec3(0.04045));
        return vec4(linear*Controls.a.y/80.0,c.a);
    }
    global = param;
    fs_main_1();
    let _e3: vec4<f32> = out_u002e_var_u002e_SV_Target;
    return _e3;
}
