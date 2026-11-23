#version 440

layout(location = 0) in float vKind;
layout(location = 1) in vec2 vD;

layout(location = 0) out vec4 fragColor;

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

void main()
{
    vec3 rgb = color.rgb;
    float a;
    if (vKind < 0.5) {
        // vD.x runs 0 at the baseline to 1 at the line.
        a = fill * vD.x;
    } else if (vKind < 1.5) {
        a = clamp(halfWidth + 0.5 - abs(vD.x), 0.0, 1.0);
    } else if (vKind < 2.5) {
        // Quadratic falloff reads as light, not as a second blurry stroke.
        float k = clamp(1.0 - abs(vD.x) / (glowRadius + 1.0), 0.0, 1.0);
        a = glow * k * k;
    } else if (vKind < 3.5) {
        // No dot while the leading edge sits in a timeout gap.
        a = edgeValue < 0.0 ? 0.0 : clamp(dotRadius + 0.5 - length(vD), 0.0, 1.0);
    } else {
        rgb = vec3(1.0, 0.09, 0.267);
        a = 0.85 * clamp(dotRadius - length(vD), 0.0, 1.0);
    }
    a *= color.a * depthAlpha * qt_Opacity;
    fragColor = vec4(rgb * a, a);
}
