//! Pingy: multi-host ping visualizer.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod bridge;
mod config;
mod engine;
mod graph_data;
mod model;
mod ping;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};

fn main() {
    #[cfg(windows)]
    unsafe {
        // Matches installer.toml's aumid, so the running window groups under
        // the pinned taskbar shortcut.
        let _ = windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID(windows::core::w!(
            "LockeWerks.Pingy"
        ));
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("tokio runtime");
    let _guard = runtime.enter();
    engine::clock::now_ms();
    engine::start();

    let mut app = QGuiApplication::new();
    if let Some(mut app) = app.as_mut() {
        app.as_mut().set_application_name(&QString::from("Pingy"));
        app.as_mut().set_organization_name(&QString::from("Locke Werks"));
        app.as_mut().set_application_version(&QString::from(env!("CARGO_PKG_VERSION")));
    }
    let mut qml = QQmlApplicationEngine::new();
    if let Some(qml) = qml.as_mut() {
        qml.load(&QUrl::from("qrc:/qt/qml/Pingy/qml/Main.qml"));
    }
    let code = app.as_mut().map(|app| app.exec()).unwrap_or(1);
    drop(qml);
    engine::shutdown();
    drop(app);
    std::process::exit(code);
}
