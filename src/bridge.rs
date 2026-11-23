//! The QObject QML talks to. State crosses as one JSON snapshot, refreshed
//! by `poll()` from a QML timer, so no Rust thread ever touches a QObject.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    #[auto_cxx_name]
    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, snapshot)]
        #[qproperty(QString, version)]
        type Engine = super::EngineRust;

        /// Refresh `snapshot` if anything changed.
        #[qinvokable]
        fn poll(self: Pin<&mut Engine>);

        /// False if the host is blank or already listed.
        #[qinvokable]
        fn add(self: Pin<&mut Engine>, host: &QString, name: &QString) -> bool;

        #[qinvokable]
        fn remove(self: Pin<&mut Engine>, id: i32);

        #[qinvokable]
        fn set_active(self: Pin<&mut Engine>, id: i32, active: bool);

        #[qinvokable]
        fn set_running(self: Pin<&mut Engine>, running: bool);

        #[qinvokable]
        fn clear_history(self: Pin<&mut Engine>);

        #[qinvokable]
        fn apply_settings(self: Pin<&mut Engine>, timeout_ms: i32, interval_ms: i32, ttl: i32, history: i32);
    }
}

use core::pin::Pin;
use cxx_qt_lib::QString;

pub struct EngineRust {
    snapshot: QString,
    version: QString,
}

impl Default for EngineRust {
    fn default() -> Self {
        Self {
            snapshot: QString::from(&crate::engine::snapshot_json()),
            version: QString::from(env!("CARGO_PKG_VERSION")),
        }
    }
}

impl qobject::Engine {
    fn poll(self: Pin<&mut Self>) {
        // Compared as text rather than by revision: resolve errors change
        // without a bump, and the timer is only a few hertz.
        let json = QString::from(&crate::engine::snapshot_json());
        if json != *self.snapshot() {
            self.set_snapshot(json);
        }
    }

    fn add(mut self: Pin<&mut Self>, host: &QString, name: &QString) -> bool {
        let ok = crate::engine::add(&host.to_string(), &name.to_string());
        self.as_mut().poll();
        ok
    }

    fn remove(mut self: Pin<&mut Self>, id: i32) {
        crate::engine::remove(id as u64);
        self.as_mut().poll();
    }

    fn set_active(mut self: Pin<&mut Self>, id: i32, active: bool) {
        crate::engine::set_active(id as u64, active);
        self.as_mut().poll();
    }

    fn set_running(mut self: Pin<&mut Self>, running: bool) {
        crate::engine::set_running(running);
        self.as_mut().poll();
    }

    fn clear_history(mut self: Pin<&mut Self>) {
        crate::engine::clear_history();
        self.as_mut().poll();
    }

    fn apply_settings(mut self: Pin<&mut Self>, timeout_ms: i32, interval_ms: i32, ttl: i32, history: i32) {
        crate::engine::apply_settings(crate::config::Settings {
            timeout_ms: timeout_ms.max(0) as u32,
            interval_ms: interval_ms.max(0) as u32,
            ttl: ttl.clamp(1, 255) as u8,
            history_length: history.max(0) as u32,
        });
        self.as_mut().poll();
    }
}
