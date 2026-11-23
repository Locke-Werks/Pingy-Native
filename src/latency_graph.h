#pragma once

#include <QColor>
#include <QMetaObject>
#include <QQuickItem>
#include <QtQml/qqmlregistration.h>

#include <cstdint>
#include <vector>

class QSGNode;
class QQuickWindow;

// The 3D latency graph. Every target is one GPU geometry node; the vertex
// shader maps sample time to x on every frame, so motion is one constant
// velocity no matter when a reply arrived. Rust owns the data and the clock.
class LatencyGraph : public QQuickItem {
    Q_OBJECT
    QML_ELEMENT
    Q_PROPERTY(double scaleTop READ scaleTop NOTIFY scaleTopChanged)
    Q_PROPERTY(double windowMs READ windowMs NOTIFY windowMsChanged)
    Q_PROPERTY(double lineWidth READ lineWidth WRITE setLineWidth NOTIFY lineWidthChanged)

public:
    explicit LatencyGraph(QQuickItem* parent = nullptr);

    double scaleTop() const { return reported_top_; }
    double windowMs() const { return window_ms_; }
    double lineWidth() const { return line_width_; }
    void setLineWidth(double w);

    struct Run {
        std::vector<float> t;  // tessellated, ms relative to the newest knot
        std::vector<float> v;
    };
    struct Track {
        std::size_t color = 0;
        bool active = true;
        double newest = 0;          // time of the newest knot, shared clock
        std::vector<Run> runs;      // contiguous successful stretches
        std::vector<float> misses;  // timeout times, relative to newest
        bool dirty = true;
    };

signals:
    void scaleTopChanged();
    void windowMsChanged();
    void lineWidthChanged();

protected:
    QSGNode* updatePaintNode(QSGNode* old, UpdatePaintNodeData*) override;
    void itemChange(ItemChange change, const ItemChangeData& data) override;
    void geometryChange(const QRectF& now, const QRectF& before) override;

private:
    void onFrame();
    void pull();
    double targetTop() const;
    void attachWindow(QQuickWindow* w);

    std::vector<Track> tracks_;
    std::uint64_t revision_ = 0;
    double interval_ms_ = 1000;
    double window_ms_ = 59000;
    bool running_ = true;
    double render_ms_ = 0;     // the time drawn at the right edge
    double top_ = 0;           // current, gliding value at the top edge
    double reported_top_ = 0;
    double last_frame_ms_ = 0;
    double line_width_ = 1.75;
    bool tracks_changed_ = true;
    QQuickWindow* window_ = nullptr;
    QMetaObject::Connection frame_conn_;
};
