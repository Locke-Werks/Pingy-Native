#version 440

// a0: time A, time B (ms relative to the newest sample, <= 0), kind, side
// a1: value A, value B, endpoint (0 A, 1 B), unused
// a2: corner x, corner y
// kind: 0 fill, 1 line, 2 glow, 3 leading-edge dot, 4 timeout marker
layout(location = 0) in vec4 a0;
layout(location = 1) in vec4 a1;
layout(location = 2) in vec2 a2;

layout(location = 0) out float vKind;
layout(location = 1) out vec2 vD;

layout(std140, binding = 0) uniform buf {
    mat4 qt_Matrix;
    float qt_Opacity;
    float age;
    float pxPerMs;
    float halfWidth;
    vec4 color;
    vec2 size;
    float top;
    float fill;
    float glow;
    float dotRadius;
    float glowRadius;
    float edgeValue;
    float depthScale;
    float lift;
    float depthAlpha;
    float pad;
};

float yOf(float v)
{
    return size.y - clamp(v / max(top, 1e-6), 0.0, 1.0) * size.y;
}

float xOf(float t)
{
    return size.x + (t - age) * pxPerMs;
}

void main()
{
    float kind = a0.z;
    float side = a0.w;
    float endB = a1.z;

    // Position is a pure function of time: the right edge is the render time
    // and every sample sits at its own timestamp, so the whole graph moves at
    // one constant velocity regardless of when a reply was delivered.
    vec2 A = vec2(xOf(a0.x), yOf(a1.x));
    vec2 B = vec2(xOf(a0.y), yOf(a1.y));
    vec2 P = mix(A, B, endB);
    vec2 d = vec2(side, 0.0);

    if (kind < 0.5) {
        P.y = mix(size.y, P.y, side);
    } else if (kind < 2.5) {
        vec2 seg = B - A;
        float len = length(seg);
        vec2 dir = len > 1e-4 ? seg / len : vec2(1.0, 0.0);
        vec2 n = vec2(-dir.y, dir.x);
        // One pixel past the edge leaves room for the feather; extending along
        // the segment closes the wedge gap at each joint.
        float reach = (kind < 1.5 ? halfWidth : glowRadius) + 1.0;
        float ext = kind < 1.5 ? halfWidth : 0.0;
        P += n * side * reach + dir * (endB * 2.0 - 1.0) * ext;
        d = vec2(side * reach, 0.0);
    } else if (kind < 3.5) {
        float reach = dotRadius + 1.0;
        P = vec2(size.x - reach, yOf(max(edgeValue, 0.0))) + a2 * reach;
        d = a2 * reach;
    } else {
        float reach = dotRadius + 0.5;
        P = vec2(xOf(a0.x), size.y - reach) + a2 * reach;
        d = a2 * reach;
    }

    // Perspective: targets further back shrink toward the centre and rise a
    // little, which is what reads as depth between the stacked traces.
    P.x = size.x * 0.5 + (P.x - size.x * 0.5) * depthScale;
    P.y -= lift;

    vKind = kind;
    vD = d;
    gl_Position = qt_Matrix * vec4(P, 0.0, 1.0);
}
