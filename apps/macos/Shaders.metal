#include <metal_stdlib>
using namespace metal;

struct FrameCB {
    float4x4 uView;
    float4x4 uProj;
    float4   uBend;     // (bend.x, bend.squash, bend.z, 0)
    float4   uCamRight; // world right vector for sprites
    float4   uCamUp;    // world up vector for sprites
};

struct FurCB {
    float4 uFur;
    float4 uLight;
    float4 uDark;
    float4 uPatchA;
    float4 uPatchB;
    float4 uPoint;
    float4 uInner;
    float4 uW0; // (stripes, muzzle, bib, calico)
    float4 uW1; // (tuxedo, points, curl, innerAmt)
    float4 uW2; // (shade, 0, 0, 0)
};

struct ObjCB {
    float4x4 uModel;
    float4x4 uNormalMat;
    float4   uColor;  // (r, g, b, alpha)
    // x: mode (0=flat, 1=vertexColor, 2=bodyBendRim, 3=chinLine, 4=armRootPush)
    // y: invScaleX (for bodyBend) or chinWidth (for chinLine)
    // z: invScaleZ (for bodyBend) or towardEye (for collar/bell)
    // w: furPart (0=HEAD, 1=BODY, 2=EAR_L, 3=EAR_R, 4=TAIL, 5=ARM)
    float4   uParams;
};

struct VSIn {
    float3 pos    [[attribute(0)]];
    float3 normal [[attribute(1)]];
    float3 rest   [[attribute(2)]];
    float3 color  [[attribute(3)]];
    float  along  [[attribute(4)]];
};

static inline float3 apply_body_bend(float3 rest, float2 invScale, float4 uBend) {
    float bwT = smoothstep(0.15, 1.32, rest.y);
    float bw = bwT > 0.0 ? pow(bwT, 1.6) : 0.0;
    float bsk = smoothstep(0.2, 1.32, rest.y);
    return float3(
        rest.x + uBend.x * bw * invScale.x,
        rest.y * (1.0 - uBend.y * bsk),
        rest.z + uBend.z * bw * invScale.y
    );
}

// ---------- Basic / Rim / Chin / VertexColor / Collar ----------

struct BasicVSOut {
    float4 pos   [[position]];
    float3 color;
};

vertex BasicVSOut vs_basic(
    VSIn input [[stage_in]],
    constant FrameCB& frame [[buffer(2)]],
    constant ObjCB& obj [[buffer(4)]]
) {
    BasicVSOut o;
    int mode = int(obj.uParams.x + 0.5);
    float3 p = input.pos;
    if (mode == 2) {
        p = apply_body_bend(input.rest, obj.uParams.yz, frame.uBend);
    } else if (mode == 3) {
        float w = obj.uParams.y * (1.0 - smoothstep(-0.55, 0.05, p.y));
        p = p + input.normal * w;
    }
    float4 mv = frame.uView * (obj.uModel * float4(p, 1.0));
    if (mode == 4) {
        mv.xyz *= 1.0 + 0.13 * (1.0 - smoothstep(0.0, 0.22, input.along)) / max(-mv.z, 0.01);
    } else if (mode != 2 && obj.uParams.z > 0.0) {
        mv.xyz *= 1.0 - obj.uParams.z / max(-mv.z, 0.01);
    }
    o.pos = frame.uProj * mv;
    o.color = (mode == 1) ? (input.color * obj.uColor.rgb) : obj.uColor.rgb;
    return o;
}

fragment float4 ps_basic(
    BasicVSOut input [[stage_in]],
    constant ObjCB& obj [[buffer(4)]]
) {
    float a = obj.uColor.a;
    return float4(input.color * a, a);
}

// ---------- Instanced Keyboard (all keycaps in 3 draw calls) ----------

struct InstVSIn {
    float3 pos    [[attribute(0)]];
    float3 normal [[attribute(1)]];
    float3 rest   [[attribute(2)]];
    float3 color  [[attribute(3)]];
    float  along  [[attribute(4)]];
    float4 m0     [[attribute(5)]];
    float4 m1     [[attribute(6)]];
    float4 m2     [[attribute(7)]];
    float4 m3     [[attribute(8)]];
    float4 icol   [[attribute(9)]];
};

vertex BasicVSOut vs_instanced(
    InstVSIn input [[stage_in]],
    constant FrameCB& frame [[buffer(2)]],
    constant ObjCB& obj [[buffer(4)]]
) {
    BasicVSOut o;
    float4 localPos = input.m0 * input.pos.x + input.m1 * input.pos.y + input.m2 * input.pos.z + input.m3;
    float4 worldPos = obj.uModel * localPos;
    float4 mv = frame.uView * worldPos;
    o.pos = frame.uProj * mv;
    o.color = input.icol.rgb;
    return o;
}

fragment float4 ps_instanced(BasicVSOut input [[stage_in]]) {
    return float4(input.color, 1.0);
}

// ---------- Procedural Fur Shader (HEAD / BODY / EAR / TAIL / ARM) ----------

struct FurVSOut {
    float4 pos    [[position]];
    float3 vRest;
    float3 vNorm;
    float  vAlong;
};

vertex FurVSOut vs_fur(
    VSIn input [[stage_in]],
    constant FrameCB& frame [[buffer(2)]],
    constant ObjCB& obj [[buffer(4)]]
) {
    FurVSOut o;
    int part = int(obj.uParams.w + 0.5);
    float3 p = input.pos;
    if (part == 1) {
        o.vRest = input.rest;
        p = apply_body_bend(input.rest, obj.uParams.yz, frame.uBend);
    } else if (part == 2 || part == 3) {
        o.vRest = input.rest;
    } else {
        o.vRest = input.pos;
    }
    o.vAlong = input.along;
    float3x3 nmat = float3x3(obj.uNormalMat[0].xyz, obj.uNormalMat[1].xyz, obj.uNormalMat[2].xyz);
    o.vNorm = normalize(nmat * input.normal);
    float4 mv = frame.uView * (obj.uModel * float4(p, 1.0));
    if (part == 5) {
        mv.xyz *= 1.0 + 0.13 * (1.0 - smoothstep(0.0, 0.22, input.along)) / max(-mv.z, 0.01);
    }
    o.pos = frame.uProj * mv;
    return o;
}

static inline float blob2(float2 p, float2 c, float2 r) {
    return 1.0 - smoothstep(0.96, 1.0, length((p - c) / r));
}

static inline float blob3(float3 p, float3 c, float r) {
    return 1.0 - smoothstep(r * 0.97, r, length(p - c));
}

static inline float stroke_seg(float2 p, float2 a, float2 b, float w, float taper) {
    float2 ab = b - a;
    float h = clamp(dot(p - a, ab) / dot(ab, ab), 0.0, 1.0);
    float r = w * mix(1.0, taper, h);
    return 1.0 - smoothstep(r * 0.88, r, length(p - a - ab * h));
}

static inline float rings(float x, float n, float fill) {
    float f = fract(x * n);
    float d = abs(f - 0.5);
    return 1.0 - smoothstep(fill * 0.5 - 0.025, fill * 0.5, d);
}

fragment float4 ps_fur(
    FurVSOut input [[stage_in]],
    constant FurCB& fur [[buffer(3)]],
    constant ObjCB& obj [[buffer(4)]]
) {
    constexpr float3 LIGHT_DIR = float3(-0.352208, 0.754732, 0.553470);
    int part = int(obj.uParams.w + 0.5);
    float uStripes  = fur.uW0.x;
    float uMuzzle   = fur.uW0.y;
    float uBib      = fur.uW0.z;
    float uCalico   = fur.uW0.w;
    float uTuxedo   = fur.uW1.x;
    float uPoints   = fur.uW1.y;
    float uCurl     = fur.uW1.z;
    float uInnerAmt = fur.uW1.w;
    float uShade    = fur.uW2.x;

    float3 col = fur.uFur.rgb;
    float3 p = input.vRest;
    float light = 0.0, dark = 0.0, inner = 0.0, pt = 0.0, curl = 0.0, pa = 0.0, pb = 0.0;

    if (part == 0) {
        // HEAD
        if (p.z > 0.0) {
            float2 q = p.xy;
            if (uPoints > 0.0) {
                pt = 1.0 - smoothstep(0.62, 1.0, length((q - float2(0.0, -0.2)) / float2(0.74, 0.74)));
            }
            if (uMuzzle > 0.0 || uTuxedo > 0.0) {
                float m = max(max(blob2(q, float2(-0.2, -0.47), float2(0.27, 0.22)),
                                  blob2(q, float2(0.2, -0.47), float2(0.27, 0.22))),
                              blob2(q, float2(0.0, -0.66), float2(0.26, 0.18)));
                light = max(light, m * uMuzzle);
                if (uTuxedo > 0.0) {
                    float blaze = max(m, blob2(q, float2(0.0, -0.1), float2(0.12, 0.42)) * step(q.y, 0.35));
                    light = max(light, blaze * uTuxedo);
                }
            }
            if (uStripes > 0.0) {
                float mk = stroke_seg(q, float2(0.0, 0.42), float2(0.0, 0.85), 0.07, 0.55);
                mk = max(mk, stroke_seg(float2(abs(q.x), q.y), float2(0.15, 0.4), float2(0.27, 0.84), 0.06, 0.45));
                mk = max(mk, stroke_seg(float2(abs(q.x), q.y), float2(0.72, 0.05), float2(0.98, 0.16), 0.055, 0.4));
                mk = max(mk, stroke_seg(float2(abs(q.x), q.y), float2(0.74, -0.16), float2(0.99, -0.12), 0.05, 0.4));
                dark = max(dark, mk * uStripes);
            }
        }
        if (uCalico > 0.0) {
            pa = blob3(p, float3(-0.6, 0.7, 0.35), 0.62);
            pb = blob3(p, float3(0.72, 0.55, 0.1), 0.55);
        }
    } else if (part == 1) {
        // BODY
        if (uBib > 0.0 || uTuxedo > 0.0 || uStripes > 0.0 || uCurl > 0.0 || uCalico > 0.0) {
            float a = abs(atan2(p.x, p.z));
            float bib = 0.0;
            if (uBib > 0.0 || uTuxedo > 0.0 || uCalico > 0.0) {
                float bibW = mix(0.55, 0.85, smoothstep(0.75, 1.2, p.y));
                bib = 1.0 - smoothstep(bibW * 0.96, bibW, a);
                light = max(light, bib * max(uBib, uTuxedo));
            }
            if (uStripes > 0.0) {
                float wave = p.y + 0.05 * sin(a * 3.0);
                float reach = smoothstep(0.62, 1.1, a);
                float band = rings(wave, 6.5, 0.36 * reach) * step(0.3, p.y) * step(p.y, 1.2);
                dark = max(dark, band * uStripes);
            }
            if (uCalico > 0.0) {
                pa = blob3(p, float3(-0.5, 0.95, -0.3), 0.55);
                pb = blob3(p, float3(0.55, 0.55, -0.15), 0.5) * (1.0 - bib);
            }
            if (uCurl > 0.0) {
                curl = rings(p.y + 0.035 * sin(a * 11.0 + p.y * 7.0), 10.0, 0.14) * step(0.18, p.y) * step(p.y, 1.22);
            }
        }
    } else if (part == 2 || part == 3) {
        // EAR_L (2) / EAR_R (3)
        if (uPoints > 0.0) pt = smoothstep(0.06, 0.2, p.y);
        if (uInnerAmt > 0.0) {
            float rMax = mix(0.28, 0.0, clamp((p.y + 0.06) / 0.43, 0.0, 1.0));
            inner = (1.0 - smoothstep(0.52, 0.58, abs(p.x) / max(rMax, 0.001)))
                * smoothstep(0.13, 0.14, p.y)
                * (1.0 - smoothstep(0.3, 0.31, p.y))
                * step(0.0, p.z);
        }
        if (part == 2) { pa = 1.0; pb = 0.0; } else { pa = 0.0; pb = 1.0; }
    } else if (part == 4) {
        // TAIL
        if (uStripes > 0.0) {
            dark = max(dark, rings(input.vAlong, 5.0, 0.42) * step(0.2, input.vAlong) * uStripes);
            dark = max(dark, smoothstep(0.9, 0.91, input.vAlong) * uStripes);
        }
        if (uCalico > 0.0) { pa = rings(input.vAlong + 0.12, 1.0, 0.3); pb = 1.0 - pa; }
        if (uPoints > 0.0) pt = smoothstep(0.0, 0.3, input.vAlong);
        if (uCurl > 0.0) curl = rings(input.vAlong, 9.0, 0.16) * step(0.12, input.vAlong);
    } else if (part == 5) {
        // ARM
        if (uStripes > 0.0) dark = max(dark, rings(input.vAlong, 3.0, 0.38) * step(0.4, input.vAlong) * uStripes);
        if (uPoints > 0.0) pt = smoothstep(0.3, 0.8, input.vAlong);
    }

    if (uCalico > 0.0) {
        col = mix(col, fur.uPatchA.rgb, pa * uCalico);
        col = mix(col, fur.uPatchB.rgb, pb * uCalico);
    }
    if (uPoints > 0.0) col = mix(col, fur.uPoint.rgb, pt * uPoints);
    if (uStripes > 0.0 || uCurl > 0.0) col = mix(col, fur.uDark.rgb, max(dark, curl * uCurl));
    if (light > 0.0) col = mix(col, fur.uLight.rgb, light);
    if (uInnerAmt > 0.0) col = mix(col, fur.uInner.rgb, inner * uInnerAmt);

    if (uShade > 0.0) {
        float3 n = normalize(input.vNorm);
        float lit = dot(n, LIGHT_DIR);
        float edge = (1.0 - clamp(n.z, 0.0, 1.0)) * abs(n.x);
        if (part == 0) {
            float calm = smoothstep(0.05, 0.45, p.y);
            lit = mix(lit, 0.8, calm);
            edge *= 1.0 - calm;
        } else if (part == 2 || part == 3) {
            lit = 0.8;
            edge = 0.0;
        }
        col *= 1.0 + uShade * (0.18 * lit - 0.4 * edge * edge - 0.05);
    }

    return float4(col, 1.0);
}

// ---------- Billboard Sprites (keycaps, sleep Z, anger, bang, dust) ----------

struct SpriteVSOut {
    float4 pos [[position]];
    float2 uv;
};

vertex SpriteVSOut vs_sprite(
    uint vid [[vertex_id]],
    constant FrameCB& frame [[buffer(2)]],
    constant ObjCB& obj [[buffer(4)]]
) {
    constexpr float2 corners[4] = {
        float2(-0.5,  0.5),
        float2( 0.5,  0.5),
        float2(-0.5, -0.5),
        float2( 0.5, -0.5),
    };
    constexpr float2 uvs[4] = {
        float2(0.0, 0.0),
        float2(1.0, 0.0),
        float2(0.0, 1.0),
        float2(1.0, 1.0),
    };
    float3 center = (obj.uModel * float4(0.0, 0.0, 0.0, 1.0)).xyz;
    float2 c = corners[vid];
    float3 world = center + frame.uCamRight.xyz * (c.x * obj.uParams.x) + frame.uCamUp.xyz * (c.y * obj.uParams.y);
    SpriteVSOut o;
    o.pos = frame.uProj * (frame.uView * float4(world, 1.0));
    o.uv = uvs[vid];
    return o;
}

fragment float4 ps_sprite(
    SpriteVSOut input [[stage_in]],
    constant ObjCB& obj [[buffer(4)]],
    texture2d<float> uSpriteTex [[texture(0)]],
    sampler uSpriteSmp [[sampler(0)]]
) {
    float4 tex = uSpriteTex.sample(uSpriteSmp, input.uv);
    return tex * obj.uColor.a;
}
