# Pingy: Rust + Qt 6 port

## Intent

Pingy is a multi-host ping visualizer whose 3D perspective latency graph is the
product. The C++ version is Windows-only and built without a C runtime, which
made every change expensive. This port moves it to Rust with a Qt Quick UI so it
runs on Windows, macOS and Linux, each with a real installer.

## Goals

- Feature parity: concurrent pings to many hosts, live current/avg/min/max/loss,
  3D perspective graph with per-target depth, wide/tall responsive layout,
  timeout/TTL/interval settings, add/remove/toggle targets, persistent config,
  the four default DNS targets, latency colour coding.
- Graph motion as smooth as Ring0's sparklines.
- Installers: Forge v0.4.1 on Windows, signed and notarized `.dmg` on macOS,
  `.deb` and `.rpm` on Linux.
- Documentation rewritten: README, CONTRIBUTING, GitHub repo description/topics.

## Non-goals

- Binary size. The 66 KB story ends here.
- AppImage, Flatpak, Snap.
- Importing the old hand-written config format beyond reading the same keys.

## Anti-goals

- A graph that steps once per sample, or that jumps when the scale changes.
- A Linux package that installs fine and then shows every ping as a timeout.

## Approaches considered

- **A. Custom scene-graph item (picked).** C++ `QQuickItem` with its own
  `QSGMaterial` and shaders, compiled through cxx-qt's C++ builder, fed by Rust.
  GPU-rendered on every backend.
- B. QML `Canvas` from JavaScript. CPU-rasterized, stutters at high DPI.
- C. `QQuickPaintedItem`. Still CPU rasterization plus a texture upload a frame.

## Design

- One crate, `pingy`, cxx-qt 0.10 (same stack as Murk).
- **Engine (Rust):** one tokio task per target. Windows uses `IcmpSendEcho2`
  through the `windows` crate (no admin needed). macOS and Linux use
  `surge-ping`, trying an unprivileged ICMP datagram socket first and falling
  back to a raw socket. Results are stored with a monotonic timestamp in a
  per-target ring buffer with running stats.
- **Bridge (Rust, cxx-qt):** a QML singleton `Engine` exposing a target list
  model, settings and add/remove/toggle invokables. A plain `cxx` bridge exports
  `pingy_series()` so the C++ graph pulls timestamped samples directly.
- **Graph (C++):** `LatencyGraph` QQuickItem. One geometry node per target with
  Ring0's technique: knots at their own timestamps, render delay, time-to-x in
  the vertex shader, Fritsch-Carlson monotone cubic tessellation, exponential
  scale glide, redraw on `afterAnimating`. A perspective projection with per-target
  depth runs in the shader. Grid and axis lines are a second node.
- **UI (QML):** sidebar with target cards and stats, settings panel, add-host
  dialog, `SmoothWheel` from Murk on every scrollable view.
- **Config:** JSON via serde in the platform config dir (`directories` crate).
- **Linux ICMP:** the `.deb`/`.rpm` post-install runs
  `setcap cap_net_raw+ep /usr/bin/pingy`, so the raw-socket fallback works on
  distros whose `ping_group_range` excludes users.

## Left to the builder

Visual tuning, file layout inside `src/` and `qml/`, CI job structure.
