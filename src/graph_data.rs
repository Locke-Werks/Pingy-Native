//! What the C++ graph item pulls from Rust each frame.

#[cxx_qt::bridge(namespace = "pingy")]
pub mod ffi {
    struct GraphSeries {
        color: usize,
        active: bool,
        /// Send times on the shared clock, oldest first.
        times: Vec<f64>,
        /// Latency in ms; negative means the echo timed out.
        values: Vec<f64>,
    }

    struct GraphFrame {
        revision: u64,
        interval_ms: f64,
        history: u32,
        running: bool,
        series: Vec<GraphSeries>,
    }

    extern "Rust" {
        fn now_ms() -> f64;
        fn revision() -> u64;
        /// Copy of every target's tail: `history + extra` samples each.
        fn frame(extra: u32) -> GraphFrame;
    }
}

use ffi::{GraphFrame, GraphSeries};

fn now_ms() -> f64 {
    crate::engine::clock::now_ms()
}

fn revision() -> u64 {
    crate::engine::revision()
}

fn frame(extra: u32) -> GraphFrame {
    let rev = crate::engine::revision();
    let running = crate::engine::state().running;
    let (series, settings) = crate::engine::series(extra as usize);
    GraphFrame {
        revision: rev,
        interval_ms: f64::from(settings.interval_ms),
        history: settings.history_length,
        running,
        series: series
            .into_iter()
            .map(|s| GraphSeries { color: s.color, active: s.active, times: s.times, values: s.values })
            .collect(),
    }
}
