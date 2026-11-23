use cxx_qt_build::{CxxQtBuilder, QResource, QResources, QmlFile, QmlModule};
use qt_build_utils::QResourceFile;
use std::path::{Path, PathBuf};
use std::process::Command;

const COMPONENTS: &[&str] = &[
    "qml/Main.qml",
    "qml/Sidebar.qml",
    "qml/TargetCard.qml",
    "qml/GraphPanel.qml",
    "qml/SettingsPanel.qml",
    "qml/AddDialog.qml",
    "qml/LabeledSlider.qml",
    "qml/FlatButton.qml",
    "qml/SmoothWheel.qml",
];

const SHADERS: &[&str] = &["trace.vert", "trace.frag"];

fn main() {
    let mut files: Vec<QmlFile> = COMPONENTS.iter().map(|f| QmlFile::from(*f)).collect();
    files.push(QmlFile::from("qml/Theme.qml").singleton(true));

    let mut res = QResource::new();
    for s in SHADERS {
        res = res.file(QResourceFile::new(shader(s)).alias(format!("shaders/{s}.qsb")));
    }

    let builder = CxxQtBuilder::new_qml_module(QmlModule::new("Pingy").qml_files(files))
        .qt_module("Quick")
        .qt_module("QuickControls2")
        .files(["src/bridge.rs", "src/graph_data.rs"])
        .cpp_file("src/latency_graph.h")
        .cpp_file("src/latency_graph.cpp")
        .qrc_resources(QResources::new().resource(res));
    // The scene-graph item needs Qt's private-free QSG headers only, which
    // ship with QtQuick; no extra module.
    builder.build();

    windows_resources();
}

/// Compile a shader with the qsb that belongs to the Qt being built against, so
/// the serialized format matches what that Qt can load. The committed .qsb is
/// the fallback for machines without Qt Shader Tools, and it is produced by a
/// recent qsb, which older Qt releases may refuse.
fn shader(name: &str) -> PathBuf {
    let src = Path::new("shaders").join(name);
    let committed = Path::new(env!("CARGO_MANIFEST_DIR")).join("shaders").join(format!("{name}.qsb"));
    println!("cargo:rerun-if-changed={}", src.display());
    println!("cargo:rerun-if-changed={}", committed.display());
    let Some(qsb) = find_qsb() else {
        println!("cargo:warning=qsb not found, using committed {name}.qsb");
        return committed;
    };
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join(format!("{name}.qsb"));
    let ok = Command::new(&qsb)
        .args(["--glsl", "100es,120,150", "--hlsl", "50", "--msl", "12", "-o"])
        .arg(&out)
        .arg(&src)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        out
    } else {
        println!("cargo:warning=qsb failed on {name}, using committed copy");
        committed
    }
}

fn find_qsb() -> Option<PathBuf> {
    let qmake = std::env::var("QMAKE").unwrap_or_else(|_| "qmake6".into());
    let exe = if cfg!(windows) { "qsb.exe" } else { "qsb" };
    for var in ["QT_HOST_BINS", "QT_HOST_LIBEXECS", "QT_INSTALL_BINS"] {
        let Ok(o) = Command::new(&qmake).args(["-query", var]).output() else { continue };
        let dir = String::from_utf8_lossy(&o.stdout).trim().to_owned();
        let p = Path::new(&dir).join(exe);
        if !dir.is_empty() && p.exists() {
            return Some(p);
        }
    }
    None
}

/// Icon and version resource on Windows. The .rc is a template so the version
/// comes from Cargo.toml and cannot drift.
fn windows_resources() {
    println!("cargo:rerun-if-changed=res/pingy.rc");
    println!("cargo:rerun-if-changed=res/pingy.manifest");
    println!("cargo:rerun-if-changed=assets/pingy-native.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let ver = std::env::var("CARGO_PKG_VERSION").unwrap();
    let mut parts: Vec<String> = ver
        .split(['.', '-'])
        .take(3)
        .map(|p| p.parse::<u16>().unwrap_or(0).to_string())
        .collect();
    parts.resize(4, "0".into());
    let dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    // rc.exe wants a plain path, without the \\?\ prefix canonicalize adds.
    let plain = |p: PathBuf| p.canonicalize().unwrap().display().to_string().replace("\\\\?\\", "").replace('\\', "/");
    let rc = std::fs::read_to_string(dir.join("res/pingy.rc"))
        .unwrap()
        .replace("@COMMA@", &parts.join(","))
        .replace("@VER@", &ver)
        .replace("@ICON@", &plain(dir.join("assets/pingy-native.ico")))
        .replace("@MANIFEST@", &plain(dir.join("res/pingy.manifest")));
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("pingy.rc");
    std::fs::write(&out, rc).unwrap();
    embed_resource::compile(&out, embed_resource::NONE).manifest_optional().unwrap();
}
