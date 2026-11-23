#include "latency_graph.h"
#include "pingy/src/graph_data.cxx.h"

#include <QQuickWindow>
#include <QSGGeometryNode>
#include <QSGMaterial>
#include <QSGMaterialShader>
#include <QSGVertexColorMaterial>

#include <algorithm>
#include <cmath>
#include <cstddef>
#include <cstring>

namespace {

constexpr int kSubdiv = 8;             // spline segments per sample interval
constexpr double kSlackMs = 300;       // render delay beyond one interval
constexpr double kScaleTauMs = 260;    // auto-scale glide time constant
constexpr double kMinTop = 50;         // ms: the scale never zooms past this
constexpr double kHeadroom = 1.2;
constexpr float kDotRadius = 2.25f;
constexpr float kGlowRadius = 7.0f;
constexpr float kGlowMax = 0.22f;
constexpr float kFill = 0.10f;
// Perspective: each target sits one step further back. Values follow the 1.x
// camera (50 degree FOV, 5-unit graph, 0.12 units per target).
constexpr float kCameraDist = 5.36f;
constexpr float kDepthStep = 0.12f;
constexpr float kLiftPx = 5.0f;

const QColor kColors[] = {
    QColor::fromRgbF(1.0f, 0.09f, 0.267f), QColor::fromRgbF(1.0f, 0.45f, 0.45f),
    QColor::fromRgbF(1.0f, 0.6f, 0.8f),    QColor::fromRgbF(0.9f, 0.9f, 0.9f),
    QColor::fromRgbF(1.0f, 0.7f, 0.3f),    QColor::fromRgbF(0.8f, 0.2f, 0.5f),
    QColor::fromRgbF(1.0f, 0.85f, 0.85f),  QColor::fromRgbF(0.7f, 0.1f, 0.1f),
};

struct Uniforms {
    float matrix[16];
    float opacity;
    float age;
    float px_per_ms;
    float half_width;
    float color[4];
    float size[2];
    float top;
    float fill;
    float glow;
    float dot_radius;
    float glow_radius;
    float edge_value;
    float depth_scale;
    float lift;
    float depth_alpha;
    float pad;
};
static_assert(sizeof(Uniforms) == 144);
static_assert(offsetof(Uniforms, color) == 80);
static_assert(offsetof(Uniforms, size) == 96);
static_assert(offsetof(Uniforms, glow) == 112);
static_assert(offsetof(Uniforms, depth_scale) == 128);

// a0: time A, time B (ms relative to the newest knot), kind, side
// a1: value A, value B, endpoint (0 A, 1 B), unused
// a2: corner x, corner y (dots and markers)
struct Vertex {
    float ta, tb, kind, side;
    float va, vb, end_b, unused;
    float cx, cy;
};
static_assert(sizeof(Vertex) == 10 * sizeof(float));

const QSGGeometry::AttributeSet& vertexAttributes()
{
    static const QSGGeometry::Attribute attrs[] = {
        QSGGeometry::Attribute::createWithAttributeType(0, 4, QSGGeometry::FloatType, QSGGeometry::PositionAttribute),
        QSGGeometry::Attribute::createWithAttributeType(1, 4, QSGGeometry::FloatType, QSGGeometry::UnknownAttribute),
        QSGGeometry::Attribute::createWithAttributeType(2, 2, QSGGeometry::FloatType, QSGGeometry::UnknownAttribute),
    };
    static const QSGGeometry::AttributeSet set = {3, int(sizeof(Vertex)), attrs};
    return set;
}

class TraceShader : public QSGMaterialShader {
public:
    TraceShader()
    {
        setShaderFileName(VertexStage, QStringLiteral(":/qt/qml/Pingy/shaders/trace.vert.qsb"));
        setShaderFileName(FragmentStage, QStringLiteral(":/qt/qml/Pingy/shaders/trace.frag.qsb"));
    }
    bool updateUniformData(RenderState& state, QSGMaterial* next, QSGMaterial* prev) override;
};

class TraceMaterial : public QSGMaterial {
public:
    TraceMaterial() { setFlag(Blending, true); }
    QSGMaterialType* type() const override
    {
        static QSGMaterialType t;
        return &t;
    }
    QSGMaterialShader* createShader(QSGRendererInterface::RenderMode) const override { return new TraceShader; }
    int compare(const QSGMaterial* other) const override
    {
        // Each trace carries its own clock offset and depth; never batch two.
        return this == other ? 0 : (this < other ? -1 : 1);
    }
    Uniforms u{};
};

bool TraceShader::updateUniformData(RenderState& state, QSGMaterial* next, QSGMaterial*)
{
    QByteArray* buf = state.uniformData();
    Q_ASSERT(buf->size() >= int(sizeof(Uniforms)));
    auto* mat = static_cast<TraceMaterial*>(next);
    // Unbatched: the dirty flags may describe another node's draw, so the
    // whole 144-byte block goes up every time.
    const QMatrix4x4 m = state.combinedMatrix();
    std::memcpy(mat->u.matrix, m.constData(), 64);
    mat->u.opacity = state.opacity();
    std::memcpy(buf->data(), &mat->u, sizeof(Uniforms));
    return true;
}

// Fritsch-Carlson monotone cubic tangents over non-uniform knots. No overshoot,
// so the curve never dips below zero or invents a spike the data lacks.
std::vector<double> monotoneTangents(const std::vector<double>& x, const std::vector<double>& y)
{
    const std::size_t n = y.size();
    std::vector<double> m(n, 0.0);
    if (n < 2) return m;
    std::vector<double> h(n - 1), d(n - 1);
    for (std::size_t i = 0; i + 1 < n; ++i) {
        h[i] = std::max(1e-6, x[i + 1] - x[i]);
        d[i] = (y[i + 1] - y[i]) / h[i];
    }
    m[0] = d[0];
    m[n - 1] = d[n - 2];
    for (std::size_t i = 1; i + 1 < n; ++i) {
        if (d[i - 1] * d[i] <= 0) continue;
        const double w1 = 2 * h[i] + h[i - 1], w2 = h[i] + 2 * h[i - 1];
        m[i] = (w1 + w2) / (w1 / d[i - 1] + w2 / d[i]);
    }
    return m;
}

LatencyGraph::Run tessellate(const std::vector<double>& xs, const std::vector<double>& ys)
{
    LatencyGraph::Run r;
    const auto n = xs.size();
    const auto m = monotoneTangents(xs, ys);
    r.t.reserve((n - 1) * kSubdiv + 1);
    r.v.reserve((n - 1) * kSubdiv + 1);
    for (std::size_t i = 0; i + 1 < n; ++i) {
        const double hh = xs[i + 1] - xs[i];
        for (int s = 0; s < kSubdiv; ++s) {
            const double u = double(s) / kSubdiv, u2 = u * u, u3 = u2 * u;
            const double h00 = 2 * u3 - 3 * u2 + 1, h10 = u3 - 2 * u2 + u;
            const double h01 = -2 * u3 + 3 * u2, h11 = u3 - u2;
            r.t.push_back(float(xs[i] + u * hh));
            r.v.push_back(float(std::max(0.0, h00 * ys[i] + h10 * hh * m[i] + h01 * ys[i + 1] + h11 * hh * m[i + 1])));
        }
    }
    r.t.push_back(float(xs.back()));
    r.v.push_back(float(ys.back()));
    return r;
}

// Curve value at time `at`, or -1 when `at` falls in a gap.
float valueAt(const LatencyGraph::Track& tr, float at)
{
    for (const auto& r : tr.runs) {
        if (r.t.empty() || at < r.t.front() || at > r.t.back()) continue;
        const auto it = std::upper_bound(r.t.begin(), r.t.end(), at);
        if (it == r.t.end()) return r.v.back();
        const auto i = std::size_t(it - r.t.begin());
        if (i == 0) return r.v.front();
        const float span = r.t[i] - r.t[i - 1];
        const float f = span > 0 ? (at - r.t[i - 1]) / span : 0.0f;
        return r.v[i - 1] + (r.v[i] - r.v[i - 1]) * f;
    }
    return -1.0f;
}

float depthScale(std::size_t i) { return kCameraDist / (kCameraDist + kDepthStep * float(i)); }
float depthAlpha(std::size_t i) { return std::max(0.45f, 1.0f - 0.07f * float(i)); }

class TraceNode : public QSGGeometryNode {
public:
    TraceNode()
    {
        auto* geo = new QSGGeometry(vertexAttributes(), 0);
        geo->setDrawingMode(QSGGeometry::DrawTriangles);
        setGeometry(geo);
        setFlag(OwnsGeometry);
        setMaterial(new TraceMaterial);
        setFlag(OwnsMaterial);
    }

    void rebuild(const LatencyGraph::Track& tr)
    {
        std::size_t segs = 0;
        for (const auto& r : tr.runs) segs += r.t.size() - 1;
        auto* geo = geometry();
        geo->allocate(int(segs * 18 + 6 + tr.misses.size() * 6));
        auto* v = static_cast<Vertex*>(geo->vertexData());
        std::size_t o = 0;
        const auto quads = [&](float kind, float lo, float hi) {
            for (const auto& r : tr.runs) {
                for (std::size_t k = 0; k + 1 < r.t.size(); ++k) {
                    const auto put = [&](float side, float end) {
                        v[o++] = {r.t[k], r.t[k + 1], kind, side, r.v[k], r.v[k + 1], end, 0, 0, 0};
                    };
                    put(lo, 0); put(hi, 0); put(hi, 1);
                    put(lo, 0); put(hi, 1); put(lo, 1);
                }
            }
        };
        // Buffer order is layer order: fill, halo, stroke.
        quads(0.0f, 0.0f, 1.0f);
        quads(2.0f, -1.0f, 1.0f);
        quads(1.0f, -1.0f, 1.0f);
        const auto corners = [&](float t, float kind) {
            const float c[6][2] = {{-1, -1}, {1, -1}, {1, 1}, {-1, -1}, {1, 1}, {-1, 1}};
            for (const auto& p : c) v[o++] = {t, t, kind, 0, 0, 0, 0, 0, p[0], p[1]};
        };
        corners(0, 3.0f);  // leading-edge dot; its height is a uniform
        for (float t : tr.misses) corners(t, 4.0f);
        markDirty(DirtyGeometry);
    }

    Uniforms& u() { return static_cast<TraceMaterial*>(material())->u; }
};

} // namespace

LatencyGraph::LatencyGraph(QQuickItem* parent) : QQuickItem(parent)
{
    setFlag(ItemHasContents, true);
    // The newest samples wait past the right edge until their time comes.
    setClip(true);
}

void LatencyGraph::setLineWidth(double w)
{
    if (w == line_width_) return;
    line_width_ = w;
    emit lineWidthChanged();
    update();
}

void LatencyGraph::pull()
{
    if (pingy::revision() == revision_ && !tracks_.empty()) return;
    // Two extra knots keep the curve continuous past the left edge as it scrolls.
    const pingy::GraphFrame f = pingy::frame(2);
    revision_ = f.revision;
    running_ = f.running;
    interval_ms_ = std::max(1.0, f.interval_ms);
    const double window = double(std::max<std::uint32_t>(f.history, 2) - 1) * interval_ms_;
    if (window != window_ms_) { window_ms_ = window; emit windowMsChanged(); }

    if (tracks_.size() != f.series.size()) tracks_changed_ = true;
    tracks_.resize(f.series.size());
    for (std::size_t i = 0; i < f.series.size(); ++i) {
        const auto& s = f.series[i];
        auto& tr = tracks_[i];
        tr.color = s.color;
        tr.active = s.active;
        tr.runs.clear();
        tr.misses.clear();
        tr.dirty = true;
        const std::size_t n = s.times.size();
        if (n == 0) continue;
        tr.newest = s.times[n - 1];
        std::vector<double> xs, ys;
        const auto flush = [&] {
            if (xs.size() >= 2) tr.runs.push_back(tessellate(xs, ys));
            xs.clear();
            ys.clear();
        };
        for (std::size_t k = 0; k < n; ++k) {
            const double t = s.times[k] - tr.newest;
            if (s.values[k] < 0) {
                flush();
                tr.misses.push_back(float(t));
            } else {
                xs.push_back(t);
                ys.push_back(s.values[k]);
            }
        }
        flush();
    }
}

double LatencyGraph::targetTop() const
{
    double peak = 0;
    for (const auto& tr : tracks_) {
        if (!tr.active) continue;
        for (const auto& r : tr.runs)
            for (std::size_t k = 0; k < r.t.size(); ++k)
                // Only what is on screen decides the scale.
                if (tr.newest + r.t[k] >= render_ms_ - window_ms_) peak = std::max(peak, double(r.v[k]));
    }
    return std::max(kMinTop, peak * kHeadroom);
}

void LatencyGraph::onFrame()
{
    if (!isVisible()) return;
    pull();
    const double now = pingy::now_ms();
    const double dt = last_frame_ms_ > 0 ? now - last_frame_ms_ : 16.0;
    last_frame_ms_ = now;
    // Paused, the picture freezes where it is instead of draining off the left.
    if (running_) render_ms_ = now - (interval_ms_ + kSlackMs);

    const double target = targetTop();
    const double k = 1.0 - std::exp(-std::max(0.0, dt) / kScaleTauMs);
    top_ = top_ <= 0 ? target : top_ + (target - top_) * k;
    if (std::abs(top_ - reported_top_) > reported_top_ * 0.005) {
        reported_top_ = top_;
        emit scaleTopChanged();
    }
    update();
}

void LatencyGraph::attachWindow(QQuickWindow* w)
{
    if (window_ == w) return;
    disconnect(frame_conn_);
    window_ = w;
    if (w) frame_conn_ = connect(w, &QQuickWindow::afterAnimating, this, &LatencyGraph::onFrame);
}

void LatencyGraph::itemChange(ItemChange change, const ItemChangeData& data)
{
    if (change == ItemSceneChange) attachWindow(data.window);
    QQuickItem::itemChange(change, data);
}

void LatencyGraph::geometryChange(const QRectF& now, const QRectF& before)
{
    QQuickItem::geometryChange(now, before);
    if (now.size() != before.size()) update();
}

QSGNode* LatencyGraph::updatePaintNode(QSGNode* old, UpdatePaintNodeData*)
{
    auto* root = old ? old : new QSGNode;
    const float w = float(width()), h = float(height());
    const float cx = w * 0.5f;

    // Child 0 is the grid; the rest are traces, back to front.
    if (!root->firstChild()) {
        auto* grid = new QSGGeometryNode;
        auto* geo = new QSGGeometry(QSGGeometry::defaultAttributes_ColoredPoint2D(), 0);
        geo->setDrawingMode(QSGGeometry::DrawLines);
        geo->setLineWidth(1);
        grid->setGeometry(geo);
        grid->setFlag(QSGNode::OwnsGeometry);
        grid->setMaterial(new QSGVertexColorMaterial);
        grid->setFlag(QSGNode::OwnsMaterial);
        root->appendChildNode(grid);
    }
    if (tracks_changed_) {
        while (root->childCount() > 1) {
            auto* n = root->lastChild();
            root->removeChildNode(n);
            delete n;
        }
        for (std::size_t i = 0; i < tracks_.size(); ++i) root->appendChildNode(new TraceNode);
        for (auto& tr : tracks_) tr.dirty = true;
        tracks_changed_ = false;
    }

    const std::size_t n = tracks_.size();
    const float px_per_ms = w / float(std::max(window_ms_, 1.0));
    const double top = top_ > 0 ? top_ : targetTop();

    // Grid: latency lines on the front plane, a baseline per depth layer, and
    // time lines anchored to absolute time so they scroll with the data.
    {
        auto* grid = static_cast<QSGGeometryNode*>(root->firstChild());
        std::vector<QSGGeometry::ColoredPoint2D> pts;
        const auto line = [&](float x1, float y1, float x2, float y2, float a) {
            const auto c = uchar(std::clamp(a, 0.0f, 1.0f) * 255);
            const uchar g = uchar(0.35f * c);
            QSGGeometry::ColoredPoint2D p1, p2;
            p1.set(x1, y1, g, g, g, c);
            p2.set(x2, y2, g, g, g, c);
            pts.push_back(p1);
            pts.push_back(p2);
        };
        if (w > 0 && h > 0) {
            for (int i = 0; i <= 10; ++i) {
                const float y = h - h * float(i) / 10.0f;
                line(0, y, w, y, i == 0 ? 0.9f : (i % 2 == 0 ? 0.45f : 0.22f));
            }
            for (std::size_t i = 1; i < n; ++i) {
                const float s = depthScale(i), lift = kLiftPx * float(i);
                line(cx - cx * s, h - lift, cx + cx * s, h - lift, 0.25f * depthAlpha(i));
            }
            // A time line every five intervals, positioned by the render clock.
            const double step = interval_ms_ * 5;
            const double t_left = render_ms_ - window_ms_;
            for (double t = std::ceil(t_left / step) * step; t <= render_ms_; t += step) {
                const float x = w + float(t - render_ms_) * px_per_ms;
                line(x, 0, x, h, 0.16f);
                if (n > 1) {
                    const std::size_t back = n - 1;
                    const float s = depthScale(back);
                    line(x, h, cx + (x - cx) * s, h - kLiftPx * float(back), 0.2f);
                }
            }
        }
        auto* geo = grid->geometry();
        geo->allocate(int(pts.size()));
        if (!pts.empty()) std::memcpy(geo->vertexData(), pts.data(), pts.size() * sizeof(pts[0]));
        grid->markDirty(QSGNode::DirtyGeometry);
    }

    // Traces: node 1 is the backmost target, the last node the frontmost.
    QSGNode* child = root->firstChild()->nextSibling();
    for (std::size_t slot = 0; slot < n && child; ++slot, child = child->nextSibling()) {
        const std::size_t i = n - 1 - slot;
        auto& tr = tracks_[i];
        auto* node = static_cast<TraceNode*>(child);
        if (tr.dirty) {
            node->rebuild(tr);
            tr.dirty = false;
        }
        auto& u = node->u();
        const float age = float(render_ms_ - tr.newest);
        const QColor c = kColors[tr.color % (sizeof(kColors) / sizeof(kColors[0]))];
        const float alpha = tr.active ? 1.0f : 0.18f;
        u.age = age;
        u.px_per_ms = px_per_ms;
        u.half_width = float(line_width_) * 0.5f;
        u.color[0] = c.redF();
        u.color[1] = c.greenF();
        u.color[2] = c.blueF();
        u.color[3] = alpha;
        u.size[0] = w;
        u.size[1] = h;
        u.top = float(top);
        u.fill = kFill;
        const float edge = valueAt(tr, age);
        u.edge_value = edge;
        u.glow = edge >= 0 ? kGlowMax * float(std::clamp(double(edge) / top, 0.0, 1.0)) : 0.0f;
        u.dot_radius = kDotRadius;
        u.glow_radius = kGlowRadius;
        u.depth_scale = depthScale(i);
        u.lift = kLiftPx * float(i);
        u.depth_alpha = depthAlpha(i);
        node->markDirty(QSGNode::DirtyMaterial);
    }
    return root;
}
